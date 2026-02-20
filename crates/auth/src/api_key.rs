use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{OrgId, Scope};

pub type ApiKeyId = Uuid;

/// Stored API key metadata (hash only, never the raw key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub org_id: OrgId,
    pub name: String,
    pub key_prefix: String, // First 8 chars for identification: "tw_sk"
    pub key_hash: String,   // bcrypt hash of full key
    pub scopes: Vec<Scope>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Result of generating a new API key
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedApiKey {
    pub id: ApiKeyId,
    pub key: String, // Full key - only returned once at creation
    pub key_prefix: String,
}

const KEY_PREFIX: &str = "tw_sk_";
const KEY_BYTES: usize = 24;

/// Generate a new API key
/// Returns the full key (show to user once) and metadata for storage
pub fn generate_api_key(
    org_id: OrgId,
    name: String,
    scopes: Vec<Scope>,
) -> (GeneratedApiKey, ApiKey) {
    use base64::Engine;
    use rand::RngCore;

    let id = Uuid::now_v7();

    // Generate random bytes
    let mut random_bytes = [0u8; KEY_BYTES];
    rand::thread_rng().fill_bytes(&mut random_bytes);

    // Encode as URL-safe base64
    let random_part = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes);

    // Full key: tw_sk_<base64>
    let full_key = format!("{}{}", KEY_PREFIX, random_part);
    let key_prefix = full_key[..16].to_string(); // "tw_sk_" + first few chars

    // Hash for storage
    let key_hash = hash_api_key(&full_key);

    let now = Utc::now();

    let generated = GeneratedApiKey {
        id,
        key: full_key,
        key_prefix: key_prefix.clone(),
    };

    let stored = ApiKey {
        id,
        org_id,
        name,
        key_prefix,
        key_hash,
        scopes,
        created_at: now,
        last_used_at: None,
        expires_at: None,
    };

    (generated, stored)
}

/// Hash an API key for storage
pub fn hash_api_key(key: &str) -> String {
    bcrypt::hash(key, bcrypt::DEFAULT_COST).expect("bcrypt hash failed")
}

/// Verify an API key against its stored hash
pub fn verify_api_key(key: &str, hash: &str) -> bool {
    bcrypt::verify(key, hash).unwrap_or(false)
}

/// Check if a string looks like an API key
pub fn is_api_key(s: &str) -> bool {
    s.starts_with(KEY_PREFIX) && s.len() > 20
}

/// Extract the prefix from a key for lookup
pub fn extract_prefix(key: &str) -> Option<&str> {
    if is_api_key(key) && key.len() >= 16 {
        Some(&key[..16])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify() {
        let org_id = Uuid::now_v7();
        let (generated, stored) =
            generate_api_key(org_id, "Test Key".to_string(), Scope::default_sdk());

        assert!(generated.key.starts_with("tw_sk_"));
        assert!(is_api_key(&generated.key));
        assert!(verify_api_key(&generated.key, &stored.key_hash));
        assert!(!verify_api_key("wrong_key", &stored.key_hash));
    }

    #[test]
    fn test_extract_prefix() {
        let key = "tw_sk_abc123xyz789abcdef";
        assert_eq!(extract_prefix(key), Some("tw_sk_abc123xyz7"));
    }
}
