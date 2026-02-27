use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{OrgId, ProjectId, Scope};

pub type ApiKeyId = Uuid;

/// Stored API key metadata (hash only, never the raw key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub org_id: OrgId,
    pub project_id: ProjectId,
    pub name: String,
    pub key_prefix: String, // First 16 chars for identification: "tw_sk_XXXXXXXXXX"
    pub key_hash: String,   // SHA-256 hex hash of full key (or legacy bcrypt hash)
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
/// SHA-256 hashes are 64-char hex strings; bcrypt hashes start with "$2b$"
const BCRYPT_PREFIX: &str = "$2";

/// Generate a new API key
/// Returns the full key (show to user once) and metadata for storage
pub fn generate_api_key(
    org_id: OrgId,
    project_id: ProjectId,
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

    // Hash for storage — SHA-256 (sub-microsecond vs ~75ms for bcrypt)
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
        project_id,
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

/// Hash an API key for storage using SHA-256.
///
/// API keys are high-entropy random strings (192 bits of randomness),
/// so a fast hash is perfectly secure — no need for bcrypt's slow KDF.
pub fn hash_api_key(key: &str) -> String {
    format!("{:x}", Sha256::digest(key.as_bytes()))
}

/// Verify an API key against its stored hash.
///
/// Supports both new SHA-256 hashes (64-char hex) and legacy bcrypt hashes
/// (starting with "$2b$") for backward compatibility with keys created before
/// the migration.
pub fn verify_api_key(key: &str, hash: &str) -> bool {
    if hash.starts_with(BCRYPT_PREFIX) {
        // Legacy bcrypt hash — slow path (~75ms), only for old keys
        bcrypt::verify(key, hash).unwrap_or(false)
    } else {
        // SHA-256 hash — fast path (<1us)
        let computed = format!("{:x}", Sha256::digest(key.as_bytes()));
        constant_time_eq(computed.as_bytes(), hash.as_bytes())
    }
}

/// Constant-time comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
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
        let project_id = Uuid::now_v7();
        let (generated, stored) = generate_api_key(
            org_id,
            project_id,
            "Test Key".to_string(),
            Scope::default_sdk(),
        );

        assert!(generated.key.starts_with("tw_sk_"));
        assert!(is_api_key(&generated.key));
        assert!(verify_api_key(&generated.key, &stored.key_hash));
        assert!(!verify_api_key("wrong_key", &stored.key_hash));
    }

    #[test]
    fn test_sha256_hash_format() {
        let hash = hash_api_key("tw_sk_test123");
        // SHA-256 produces a 64-char hex string
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_legacy_bcrypt_compat() {
        let key = "tw_sk_test_legacy_key_value";
        let bcrypt_hash = bcrypt::hash(key, 4).unwrap(); // low cost for test speed
        assert!(verify_api_key(key, &bcrypt_hash));
        assert!(!verify_api_key("wrong", &bcrypt_hash));
    }

    #[test]
    fn test_extract_prefix() {
        let key = "tw_sk_abc123xyz789abcdef";
        assert_eq!(extract_prefix(key), Some("tw_sk_abc123xyz7"));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
    }
}
