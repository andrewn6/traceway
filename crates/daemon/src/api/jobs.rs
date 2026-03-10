//! Job queue module for background task processing.
//!
//! This module provides a Redis-backed job queue that's compatible with BullMQ,
//! allowing jobs to be processed by Node.js workers or Rust workers interchangeably.
//!
//! BullMQ uses a specific Redis key structure:
//! - `bull:<queue>:id` - Job ID counter
//! - `bull:<queue>:wait` - List of waiting job IDs
//! - `bull:<queue>:active` - List of active job IDs
//! - `bull:<queue>:completed` - Set of completed job IDs
//! - `bull:<queue>:failed` - Set of failed job IDs
//! - `bull:<queue>:<job_id>` - Hash containing job data

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Job status enum matching BullMQ conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Waiting,
    Active,
    Completed,
    Failed,
    Delayed,
    Paused,
}

/// A job in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub data: serde_json::Value,
    pub opts: JobOptions,
    pub progress: u8,
    pub delay: u64,
    pub timestamp: i64,
    #[serde(default)]
    pub attempts_made: u32,
    #[serde(default)]
    pub stacktrace: Vec<String>,
    #[serde(default)]
    pub return_value: Option<serde_json::Value>,
    #[serde(default)]
    pub finished_on: Option<i64>,
    #[serde(default)]
    pub processed_on: Option<i64>,
}

/// Job options matching BullMQ JobsOptions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobOptions {
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub backoff: Option<BackoffOptions>,
    #[serde(default)]
    pub delay: u64,
    #[serde(default)]
    pub remove_on_complete: bool,
    #[serde(default)]
    pub remove_on_fail: bool,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackoffOptions {
    Fixed { delay: u64 },
    Exponential { delay: u64 },
}

/// Job queue trait
pub trait JobQueue: Send + Sync {
    /// Add a job to the queue
    fn add(
        &self,
        name: &str,
        data: serde_json::Value,
        opts: Option<JobOptions>,
    ) -> impl std::future::Future<Output = Result<Job, JobError>> + Send;

    /// Get a job by ID
    fn get_job(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<Job>, JobError>> + Send;

    /// Get jobs by status
    fn get_jobs(
        &self,
        status: JobStatus,
        start: usize,
        end: usize,
    ) -> impl std::future::Future<Output = Result<Vec<Job>, JobError>> + Send;

    /// Get queue counts
    fn get_counts(
        &self,
    ) -> impl std::future::Future<Output = Result<HashMap<JobStatus, usize>, JobError>> + Send;
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("Redis error: {0}")]
    Redis(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Job not found: {0}")]
    NotFound(String),
}

/// Redis-backed job queue compatible with BullMQ
#[cfg(feature = "cloud")]
pub mod redis_queue {
    use super::*;
    use redis::aio::ConnectionManager;
    use redis::AsyncCommands;

    pub struct RedisJobQueue {
        conn: ConnectionManager,
        queue_name: String,
        prefix: String,
    }

    impl RedisJobQueue {
        pub async fn new(redis_url: &str, queue_name: &str) -> Result<Self, JobError> {
            let client =
                redis::Client::open(redis_url).map_err(|e| JobError::Redis(e.to_string()))?;
            let conn = ConnectionManager::new(client)
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;

            Ok(Self {
                conn,
                queue_name: queue_name.to_string(),
                prefix: "bull".to_string(),
            })
        }

        fn key(&self, suffix: &str) -> String {
            format!("{}:{}:{}", self.prefix, self.queue_name, suffix)
        }

        async fn next_id(&self) -> Result<String, JobError> {
            let mut conn = self.conn.clone();
            let id: i64 = conn
                .incr(self.key("id"), 1)
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            Ok(id.to_string())
        }
    }

    impl JobQueue for RedisJobQueue {
        async fn add(
            &self,
            name: &str,
            data: serde_json::Value,
            opts: Option<JobOptions>,
        ) -> Result<Job, JobError> {
            let mut conn = self.conn.clone();
            let opts = opts.unwrap_or_default();

            let id = self.next_id().await?;
            let timestamp = Utc::now().timestamp_millis();

            let job = Job {
                id: id.clone(),
                name: name.to_string(),
                data,
                opts: opts.clone(),
                progress: 0,
                delay: opts.delay,
                timestamp,
                attempts_made: 0,
                stacktrace: vec![],
                return_value: None,
                finished_on: None,
                processed_on: None,
            };

            let job_json = serde_json::to_string(&job)?;

            // Store job data as hash (BullMQ format)
            let job_key = self.key(&id);
            conn.hset::<_, _, _, ()>(&job_key, "data", &job_json)
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            conn.hset::<_, _, _, ()>(&job_key, "name", name)
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            conn.hset::<_, _, _, ()>(&job_key, "timestamp", timestamp)
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;

            // Add to waiting list
            if opts.delay > 0 {
                // Delayed job - add to delayed sorted set
                let score = timestamp + (opts.delay as i64);
                conn.zadd::<_, _, _, ()>(self.key("delayed"), &id, score)
                    .await
                    .map_err(|e| JobError::Redis(e.to_string()))?;
            } else {
                // Immediate job - add to wait list
                conn.lpush::<_, _, ()>(self.key("wait"), &id)
                    .await
                    .map_err(|e| JobError::Redis(e.to_string()))?;
            }

            debug!(queue = %self.queue_name, job_id = %id, name, "Job added");
            Ok(job)
        }

        async fn get_job(&self, id: &str) -> Result<Option<Job>, JobError> {
            let mut conn = self.conn.clone();
            let job_key = self.key(id);

            let data: Option<String> = conn
                .hget(&job_key, "data")
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;

            match data {
                Some(json) => Ok(Some(serde_json::from_str(&json)?)),
                None => Ok(None),
            }
        }

        async fn get_jobs(
            &self,
            status: JobStatus,
            start: usize,
            end: usize,
        ) -> Result<Vec<Job>, JobError> {
            let mut conn = self.conn.clone();

            let key = match status {
                JobStatus::Waiting => self.key("wait"),
                JobStatus::Active => self.key("active"),
                JobStatus::Completed => self.key("completed"),
                JobStatus::Failed => self.key("failed"),
                JobStatus::Delayed => self.key("delayed"),
                JobStatus::Paused => self.key("paused"),
            };

            // Get job IDs from list or set
            let ids: Vec<String> = match status {
                JobStatus::Waiting | JobStatus::Active => conn
                    .lrange(&key, start as isize, end as isize)
                    .await
                    .map_err(|e| JobError::Redis(e.to_string()))?,
                _ => conn
                    .zrange(&key, start as isize, end as isize)
                    .await
                    .map_err(|e| JobError::Redis(e.to_string()))?,
            };

            let mut jobs = Vec::new();
            for id in ids {
                if let Some(job) = self.get_job(&id).await? {
                    jobs.push(job);
                }
            }

            Ok(jobs)
        }

        async fn get_counts(&self) -> Result<HashMap<JobStatus, usize>, JobError> {
            let mut conn = self.conn.clone();
            let mut counts = HashMap::new();

            // Get wait list length
            let wait: usize = conn
                .llen(self.key("wait"))
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            counts.insert(JobStatus::Waiting, wait);

            // Get active list length
            let active: usize = conn
                .llen(self.key("active"))
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            counts.insert(JobStatus::Active, active);

            // Get completed set size
            let completed: usize = conn
                .zcard(self.key("completed"))
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            counts.insert(JobStatus::Completed, completed);

            // Get failed set size
            let failed: usize = conn
                .zcard(self.key("failed"))
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            counts.insert(JobStatus::Failed, failed);

            // Get delayed set size
            let delayed: usize = conn
                .zcard(self.key("delayed"))
                .await
                .map_err(|e| JobError::Redis(e.to_string()))?;
            counts.insert(JobStatus::Delayed, delayed);

            Ok(counts)
        }
    }
}

#[cfg(feature = "cloud")]
pub use redis_queue::RedisJobQueue;

/// In-memory job queue for local development/testing
pub struct MemoryJobQueue {
    jobs: std::sync::RwLock<HashMap<String, Job>>,
    waiting: std::sync::RwLock<Vec<String>>,
    next_id: std::sync::atomic::AtomicU64,
    queue_name: String,
}

impl MemoryJobQueue {
    pub fn new(queue_name: &str) -> Self {
        Self {
            jobs: std::sync::RwLock::new(HashMap::new()),
            waiting: std::sync::RwLock::new(Vec::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
            queue_name: queue_name.to_string(),
        }
    }
}

impl JobQueue for MemoryJobQueue {
    async fn add(
        &self,
        name: &str,
        data: serde_json::Value,
        opts: Option<JobOptions>,
    ) -> Result<Job, JobError> {
        let opts = opts.unwrap_or_default();
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_string();

        let job = Job {
            id: id.clone(),
            name: name.to_string(),
            data,
            opts,
            progress: 0,
            delay: 0,
            timestamp: Utc::now().timestamp_millis(),
            attempts_made: 0,
            stacktrace: vec![],
            return_value: None,
            finished_on: None,
            processed_on: None,
        };

        self.jobs.write().unwrap().insert(id.clone(), job.clone());
        self.waiting.write().unwrap().push(id);

        Ok(job)
    }

    async fn get_job(&self, id: &str) -> Result<Option<Job>, JobError> {
        Ok(self.jobs.read().unwrap().get(id).cloned())
    }

    async fn get_jobs(
        &self,
        _status: JobStatus,
        start: usize,
        end: usize,
    ) -> Result<Vec<Job>, JobError> {
        let waiting = self.waiting.read().unwrap();
        let jobs = self.jobs.read().unwrap();

        Ok(waiting
            .iter()
            .skip(start)
            .take(end - start + 1)
            .filter_map(|id| jobs.get(id).cloned())
            .collect())
    }

    async fn get_counts(&self) -> Result<HashMap<JobStatus, usize>, JobError> {
        let mut counts = HashMap::new();
        counts.insert(JobStatus::Waiting, self.waiting.read().unwrap().len());
        counts.insert(JobStatus::Active, 0);
        counts.insert(JobStatus::Completed, 0);
        counts.insert(JobStatus::Failed, 0);
        Ok(counts)
    }
}
