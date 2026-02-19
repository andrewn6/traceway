//! Event bus abstraction for SSE fanout.
//!
//! This module provides both local (in-process) and cloud (Redis Pub/Sub) event bus
//! implementations for real-time event distribution across multiple server instances.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::SystemEvent;

/// Event bus trait for publishing and subscribing to system events
#[async_trait]
pub trait EventBus: Send + Sync + 'static {
    /// Publish an event to all subscribers
    async fn publish(&self, event: SystemEvent);

    /// Get a subscriber that receives events
    fn subscribe(&self) -> EventSubscriber;

    /// Get the number of active subscribers
    fn subscriber_count(&self) -> usize;
}

/// A subscriber that can receive events
pub struct EventSubscriber {
    inner: EventSubscriberInner,
}

enum EventSubscriberInner {
    Local(broadcast::Receiver<SystemEvent>),
    #[cfg(feature = "cloud")]
    Cloud(tokio::sync::mpsc::Receiver<SystemEvent>),
}

impl EventSubscriber {
    /// Receive the next event (blocking)
    pub async fn recv(&mut self) -> Option<SystemEvent> {
        match &mut self.inner {
            EventSubscriberInner::Local(rx) => rx.recv().await.ok(),
            #[cfg(feature = "cloud")]
            EventSubscriberInner::Cloud(rx) => rx.recv().await,
        }
    }
}

/// Local event bus using tokio broadcast channel (single-node only)
pub struct LocalEventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl LocalEventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Get the underlying broadcast sender (for backward compatibility)
    pub fn sender(&self) -> broadcast::Sender<SystemEvent> {
        self.tx.clone()
    }
}

impl Default for LocalEventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

#[async_trait]
impl EventBus for LocalEventBus {
    async fn publish(&self, event: SystemEvent) {
        let _ = self.tx.send(event);
    }

    fn subscribe(&self) -> EventSubscriber {
        EventSubscriber {
            inner: EventSubscriberInner::Local(self.tx.subscribe()),
        }
    }

    fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

/// Cloud event bus using Redis Pub/Sub for cross-node SSE fanout
#[cfg(feature = "cloud")]
pub mod cloud {
    use super::*;
    use redis::aio::ConnectionManager;
    use redis::AsyncCommands;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const REDIS_CHANNEL: &str = "llmfs:events";

    /// Redis-backed event bus for multi-node deployments
    pub struct RedisEventBus {
        /// Redis connection manager for publishing
        publisher: ConnectionManager,
        /// Redis client for creating subscriber connections
        client: redis::Client,
        /// Local broadcast for distributing events to local SSE handlers
        local_tx: broadcast::Sender<SystemEvent>,
        /// Counter for subscriber tracking
        subscriber_count: Arc<AtomicUsize>,
    }

    impl RedisEventBus {
        /// Create a new Redis event bus from a connection URL
        pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
            let client = redis::Client::open(redis_url)?;
            let publisher = ConnectionManager::new(client.clone()).await?;
            let (local_tx, _) = broadcast::channel(256);

            let bus = Self {
                publisher,
                client,
                local_tx,
                subscriber_count: Arc::new(AtomicUsize::new(0)),
            };

            // Start the Redis subscription listener
            bus.start_listener().await?;

            info!("Redis event bus initialized");
            Ok(bus)
        }

        /// Create from environment variable REDIS_URL
        pub async fn from_env() -> Result<Self, redis::RedisError> {
            let url = std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string());
            Self::new(&url).await
        }

        /// Start the background Redis subscription listener
        async fn start_listener(&self) -> Result<(), redis::RedisError> {
            let client = self.client.clone();
            let local_tx = self.local_tx.clone();

            tokio::spawn(async move {
                loop {
                    match Self::run_subscriber(&client, &local_tx).await {
                        Ok(()) => {
                            info!("Redis subscriber exited cleanly");
                            break;
                        }
                        Err(e) => {
                            error!("Redis subscriber error: {}, reconnecting in 1s", e);
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            });

            Ok(())
        }

        async fn run_subscriber(
            client: &redis::Client,
            local_tx: &broadcast::Sender<SystemEvent>,
        ) -> Result<(), redis::RedisError> {
            let conn = client.get_async_pubsub().await?;
            let mut pubsub = conn;
            pubsub.subscribe(REDIS_CHANNEL).await?;

            info!("Subscribed to Redis channel: {}", REDIS_CHANNEL);

            let mut stream = pubsub.on_message();
            while let Some(msg) = futures::StreamExt::next(&mut stream).await {
                let payload: String = msg.get_payload()?;
                match serde_json::from_str::<SystemEvent>(&payload) {
                    Ok(event) => {
                        debug!("Received event from Redis: {:?}", event);
                        let _ = local_tx.send(event);
                    }
                    Err(e) => {
                        warn!("Failed to deserialize event: {}", e);
                    }
                }
            }

            Ok(())
        }
    }

    #[async_trait]
    impl EventBus for RedisEventBus {
        async fn publish(&self, event: SystemEvent) {
            let payload = match serde_json::to_string(&event) {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                    return;
                }
            };

            let mut conn = self.publisher.clone();
            if let Err(e) = conn.publish::<_, _, ()>(REDIS_CHANNEL, &payload).await {
                error!("Failed to publish event to Redis: {}", e);
                // Fall back to local broadcast
                let _ = self.local_tx.send(event);
            } else {
                debug!("Published event to Redis");
            }
        }

        fn subscribe(&self) -> EventSubscriber {
            self.subscriber_count.fetch_add(1, Ordering::Relaxed);
            EventSubscriber {
                inner: EventSubscriberInner::Local(self.local_tx.subscribe()),
            }
        }

        fn subscriber_count(&self) -> usize {
            self.subscriber_count.load(Ordering::Relaxed)
        }
    }
}

#[cfg(feature = "cloud")]
pub use cloud::RedisEventBus;

/// Create the appropriate event bus based on configuration
pub async fn create_event_bus() -> Arc<dyn EventBus> {
    #[cfg(feature = "cloud")]
    {
        if std::env::var("REDIS_URL").is_ok() {
            match RedisEventBus::from_env().await {
                Ok(bus) => {
                    info!("Using Redis event bus for cloud deployment");
                    return Arc::new(bus);
                }
                Err(e) => {
                    warn!("Failed to connect to Redis, falling back to local: {}", e);
                }
            }
        }
    }

    info!("Using local event bus");
    Arc::new(LocalEventBus::default())
}
