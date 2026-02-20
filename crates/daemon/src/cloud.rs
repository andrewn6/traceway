//! Cloud deployment module for Traceway daemon.
//!
//! This module provides cloud-specific configuration and initialization
//! for deploying Traceway to container environments like Railway, Fly.io, etc.

use std::env;
use tracing::{info, warn};

/// Cloud deployment configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// Port to bind (from PORT env var, default 3000)
    pub port: u16,

    /// Redis URL for event bus (from REDIS_URL)
    pub redis_url: Option<String>,

    /// Turbopuffer API key (from TURBOPUFFER_API_KEY)
    pub turbopuffer_api_key: Option<String>,

    /// Turbopuffer namespace (from TURBOPUFFER_NAMESPACE, default "traceway")
    pub turbopuffer_namespace: String,

    /// Storage backend type (from STORAGE_BACKEND: "sqlite" or "turbopuffer")
    pub storage_backend: StorageBackendType,

    /// Enable metrics endpoint
    pub metrics_enabled: bool,

    /// Log format (from LOG_FORMAT: "json" or "pretty")
    pub log_format: LogFormat,

    /// Region identifier (from FLY_REGION, RAILWAY_REGION, etc.)
    pub region: Option<String>,

    /// Instance ID (from FLY_ALLOC_ID, RAILWAY_REPLICA_ID, etc.)
    pub instance_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageBackendType {
    Sqlite,
    Turbopuffer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogFormat {
    Json,
    Pretty,
}

impl CloudConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);

        let redis_url = env::var("REDIS_URL").ok();

        let turbopuffer_api_key = env::var("TURBOPUFFER_API_KEY").ok();

        let turbopuffer_namespace =
            env::var("TURBOPUFFER_NAMESPACE").unwrap_or_else(|_| "traceway".to_string());

        let storage_backend = match env::var("STORAGE_BACKEND")
            .unwrap_or_else(|_| "sqlite".to_string())
            .to_lowercase()
            .as_str()
        {
            "turbopuffer" => StorageBackendType::Turbopuffer,
            _ => StorageBackendType::Sqlite,
        };

        let metrics_enabled = env::var("METRICS_ENABLED")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(true);

        let log_format = match env::var("LOG_FORMAT")
            .unwrap_or_else(|_| "json".to_string())
            .to_lowercase()
            .as_str()
        {
            "pretty" => LogFormat::Pretty,
            _ => LogFormat::Json,
        };

        // Try multiple cloud provider region env vars
        let region = env::var("FLY_REGION")
            .or_else(|_| env::var("RAILWAY_REGION"))
            .or_else(|_| env::var("AWS_REGION"))
            .or_else(|_| env::var("REGION"))
            .ok();

        // Try multiple cloud provider instance ID env vars
        let instance_id = env::var("FLY_ALLOC_ID")
            .or_else(|_| env::var("RAILWAY_REPLICA_ID"))
            .or_else(|_| env::var("HOSTNAME"))
            .ok();

        Self {
            port,
            redis_url,
            turbopuffer_api_key,
            turbopuffer_namespace,
            storage_backend,
            metrics_enabled,
            log_format,
            region,
            instance_id,
        }
    }

    /// Get the bind address
    pub fn bind_addr(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }

    /// Check if Redis is configured
    pub fn has_redis(&self) -> bool {
        self.redis_url.is_some()
    }

    /// Check if Turbopuffer is configured
    pub fn has_turbopuffer(&self) -> bool {
        self.turbopuffer_api_key.is_some()
    }

    /// Log the cloud configuration
    pub fn log_config(&self) {
        info!(
            port = self.port,
            storage = ?self.storage_backend,
            redis = self.has_redis(),
            turbopuffer = self.has_turbopuffer(),
            metrics = self.metrics_enabled,
            region = ?self.region,
            instance = ?self.instance_id,
            "Cloud configuration loaded"
        );

        if self.storage_backend == StorageBackendType::Turbopuffer && !self.has_turbopuffer() {
            warn!("STORAGE_BACKEND=turbopuffer but TURBOPUFFER_API_KEY is not set");
        }

        if !self.has_redis() {
            warn!("REDIS_URL not set - SSE events will be local-only (single instance)");
        }
    }
}

/// Setup structured logging for cloud deployment
pub fn setup_cloud_logging(config: &CloudConfig) {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    match config.log_format {
        LogFormat::Json => {
            let subscriber = tracing_subscriber::registry().with(filter).with(
                fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_file(false)
                    .with_line_number(false),
            );
            tracing::subscriber::set_global_default(subscriber).ok();
        }
        LogFormat::Pretty => {
            let subscriber = tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().with_target(false));
            tracing::subscriber::set_global_default(subscriber).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        // Clear env vars that might interfere
        env::remove_var("PORT");
        env::remove_var("REDIS_URL");
        env::remove_var("STORAGE_BACKEND");

        let config = CloudConfig::from_env();
        assert_eq!(config.port, 3000);
        assert_eq!(config.storage_backend, StorageBackendType::Sqlite);
        assert!(!config.has_redis());
    }
}
