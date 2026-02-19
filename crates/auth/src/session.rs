use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{AuthError, OrgId, Scope, UserId};

/// JWT claims for session tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Organization ID
    pub org: String,
    /// Scopes
    pub scopes: Vec<Scope>,
    /// Issued at
    pub iat: i64,
    /// Expiration
    pub exp: i64,
}

/// Parsed session token
#[derive(Debug, Clone)]
pub struct SessionToken {
    pub user_id: UserId,
    pub org_id: OrgId,
    pub scopes: Vec<Scope>,
    pub expires_at: DateTime<Utc>,
}

const SESSION_DURATION_DAYS: i64 = 7;

/// Create a new session token (JWT)
pub fn create_session(
    user_id: UserId,
    org_id: OrgId,
    scopes: Vec<Scope>,
    secret: &[u8],
) -> Result<String, AuthError> {
    let now = Utc::now();
    let exp = now + Duration::days(SESSION_DURATION_DAYS);

    let claims = SessionClaims {
        sub: user_id.to_string(),
        org: org_id.to_string(),
        scopes,
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|_| AuthError::InvalidSession)
}

/// Verify and decode a session token
pub fn verify_session(token: &str, secret: &[u8]) -> Result<SessionToken, AuthError> {
    let token_data = decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map_err(|e| {
        if e.kind() == &jsonwebtoken::errors::ErrorKind::ExpiredSignature {
            AuthError::ExpiredSession
        } else {
            AuthError::InvalidSession
        }
    })?;

    let claims = token_data.claims;

    let user_id = claims.sub.parse().map_err(|_| AuthError::InvalidSession)?;
    let org_id = claims.org.parse().map_err(|_| AuthError::InvalidSession)?;
    let expires_at = DateTime::from_timestamp(claims.exp, 0).ok_or(AuthError::InvalidSession)?;

    Ok(SessionToken {
        user_id,
        org_id,
        scopes: claims.scopes,
        expires_at,
    })
}

/// Generate a secure random secret for JWT signing
pub fn generate_secret() -> [u8; 32] {
    use rand::RngCore;
    let mut secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret);
    secret
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_session_roundtrip() {
        let secret = generate_secret();
        let user_id = Uuid::now_v7();
        let org_id = Uuid::now_v7();
        let scopes = vec![Scope::TracesRead, Scope::TracesWrite];

        let token = create_session(user_id, org_id, scopes.clone(), &secret).unwrap();
        let parsed = verify_session(&token, &secret).unwrap();

        assert_eq!(parsed.user_id, user_id);
        assert_eq!(parsed.org_id, org_id);
        assert_eq!(parsed.scopes, scopes);
    }

    #[test]
    fn test_invalid_secret() {
        let secret1 = generate_secret();
        let secret2 = generate_secret();
        let user_id = Uuid::now_v7();
        let org_id = Uuid::now_v7();

        let token = create_session(user_id, org_id, vec![], &secret1).unwrap();
        let result = verify_session(&token, &secret2);

        assert!(matches!(result, Err(AuthError::InvalidSession)));
    }
}
