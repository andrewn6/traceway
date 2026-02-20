//! Authentication integration for the API layer.
//!
//! This module provides:
//! - API key lookup implementation backed by environment or storage
//! - Auth middleware wiring for cloud mode
//! - Query parameter auth extraction for SSE endpoints

use std::sync::Arc;

use async_trait::async_trait;
use auth::{ApiKeyLookup, AuthConfig, OrgId, Scope};
use tracing::{debug, info};

/// API key record stored in memory
#[derive(Clone)]
pub struct ApiKeyRecord {
    pub prefix: String,
    pub key_hash: String,
    pub org_id: OrgId,
    pub scopes: Vec<Scope>,
}

/// Environment-based API key lookup.
/// 
/// Reads API keys from LLMFS_API_KEYS environment variable.
/// Format: "key1,key2,key3" where each key is the full API key.
/// All keys share the same org (default org) and full scopes.
///
/// For production, you'd replace this with database-backed lookup.
pub struct EnvApiKeyLookup {
    keys: Vec<ApiKeyRecord>,
}

impl EnvApiKeyLookup {
    pub fn from_env() -> Self {
        let mut keys = Vec::new();
        
        // Load keys from LLMFS_API_KEYS env var
        if let Ok(api_keys) = std::env::var("LLMFS_API_KEYS") {
            for key in api_keys.split(',') {
                let key = key.trim();
                if key.is_empty() {
                    continue;
                }
                
                // Validate format
                if !key.starts_with("llmfs_sk_") || key.len() < 16 {
                    tracing::warn!("Invalid API key format in LLMFS_API_KEYS: {}", &key[..8.min(key.len())]);
                    continue;
                }
                
                let prefix = key[..16].to_string();
                let key_hash = auth::hash_api_key(key);
                
                keys.push(ApiKeyRecord {
                    prefix,
                    key_hash,
                    org_id: uuid::Uuid::nil(), // Default org for env-based keys
                    scopes: Scope::all(),
                });
            }
            
            if !keys.is_empty() {
                info!(count = keys.len(), "Loaded API keys from environment");
            }
        }
        
        // Also support single key via LLMFS_API_KEY
        if let Ok(key) = std::env::var("LLMFS_API_KEY") {
            let key = key.trim();
            if key.starts_with("llmfs_sk_") && key.len() >= 16 {
                let prefix = key[..16].to_string();
                let key_hash = auth::hash_api_key(key);
                
                keys.push(ApiKeyRecord {
                    prefix,
                    key_hash,
                    org_id: uuid::Uuid::nil(),
                    scopes: Scope::all(),
                });
                
                info!("Loaded API key from LLMFS_API_KEY");
            }
        }
        
        Self { keys }
    }
    
    /// Check if any keys are configured
    pub fn has_keys(&self) -> bool {
        !self.keys.is_empty()
    }
}

#[async_trait]
impl ApiKeyLookup for EnvApiKeyLookup {
    async fn lookup_api_key(&self, prefix: &str) -> Option<(OrgId, String, Vec<Scope>)> {
        self.keys
            .iter()
            .find(|k| k.prefix == prefix)
            .map(|k| (k.org_id, k.key_hash.clone(), k.scopes.clone()))
    }
}

/// No-op lookup that always returns None (for local mode)
pub struct NoopApiKeyLookup;

#[async_trait]
impl ApiKeyLookup for NoopApiKeyLookup {
    async fn lookup_api_key(&self, _prefix: &str) -> Option<(OrgId, String, Vec<Scope>)> {
        None
    }
}

/// Create auth config from environment
pub fn auth_config_from_env() -> AuthConfig {
    let is_cloud = std::env::var("LLMFS_CLOUD").is_ok()
        || std::env::var("STORAGE_BACKEND").is_ok();

    if is_cloud {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| {
                // Generate a random secret if not set (single-instance only)
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hasher};
                let s = RandomState::new();
                let mut h = s.build_hasher();
                h.write_u64(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64);
                format!("auto_{:x}", h.finish())
            });
        AuthConfig::cloud(secret.into_bytes())
    } else {
        AuthConfig::local()
    }
}

/// Extract auth token from query parameters (for SSE which can't use headers)
pub fn extract_token_from_query(query: &str) -> Option<String> {
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("token=") {
            // Simple percent-decode for common cases
            let decoded = value
                .replace("%20", " ")
                .replace("%2B", "+")
                .replace("%3D", "=")
                .replace("%2F", "/")
                .replace("%5F", "_");
            return Some(decoded);
        }
    }
    None
}
