use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::{verify_api_key, AuthContext, AuthError, Scope};

/// Configuration for auth middleware
#[derive(Clone)]
pub struct AuthConfig {
    /// When true, skip all auth checks (local mode)
    pub local_mode: bool,
    /// JWT secret for session verification
    pub jwt_secret: Vec<u8>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            local_mode: true,
            jwt_secret: vec![],
        }
    }
}

impl AuthConfig {
    pub fn local() -> Self {
        Self::default()
    }

    pub fn cloud(jwt_secret: Vec<u8>) -> Self {
        Self {
            local_mode: false,
            jwt_secret,
        }
    }
}

/// Trait for looking up API keys - implement this on your app state
#[async_trait::async_trait]
pub trait ApiKeyLookup: Send + Sync {
    async fn lookup_api_key(&self, prefix: &str) -> Option<(crate::OrgId, String, Vec<Scope>)>;
}

/// Auth middleware that extracts AuthContext from request
pub async fn auth_middleware<S>(
    State(config): State<Arc<AuthConfig>>,
    State(lookup): State<Arc<dyn ApiKeyLookup>>,
    mut request: Request<Body>,
    next: Next,
) -> Response
where
    S: Clone + Send + Sync + 'static,
{
    // Local mode: inject local context, skip auth
    if config.local_mode {
        request.extensions_mut().insert(AuthContext::local());
        return next.run(request).await;
    }

    // Try to extract auth from headers
    let auth_result = extract_auth(&request, &config, lookup.as_ref()).await;

    match auth_result {
        Ok(ctx) => {
            request.extensions_mut().insert(ctx);
            next.run(request).await
        }
        Err(e) => e.into_response(),
    }
}

/// Simplified middleware for use without lookup - just local mode check
pub async fn auth_middleware_simple<S>(
    State(config): State<AuthConfig>,
    mut request: Request<Body>,
    next: Next,
) -> Response
where
    S: Clone + Send + Sync + 'static,
{
    if config.local_mode {
        request.extensions_mut().insert(AuthContext::local());
        return next.run(request).await;
    }

    // For cloud mode without proper lookup, reject
    AuthError::MissingAuth.into_response()
}

async fn extract_auth(
    request: &Request<Body>,
    config: &AuthConfig,
    lookup: &dyn ApiKeyLookup,
) -> Result<AuthContext, AuthError> {
    // Check Authorization header
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        let auth_str = auth_header
            .to_str()
            .map_err(|_| AuthError::InvalidFormat)?;

        // Bearer token (API key or JWT)
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            // API key format: llmfs_sk_...
            if token.starts_with("llmfs_sk_") {
                return validate_api_key(token, lookup).await;
            }
            // JWT session token
            return validate_session(token, config);
        }

        return Err(AuthError::InvalidFormat);
    }

    // Check session cookie
    if let Some(cookie) = request.headers().get(header::COOKIE) {
        let cookie_str = cookie.to_str().map_err(|_| AuthError::InvalidFormat)?;
        if let Some(session) = extract_session_cookie(cookie_str) {
            return validate_session(&session, config);
        }
    }

    Err(AuthError::MissingAuth)
}

async fn validate_api_key(
    key: &str,
    lookup: &dyn ApiKeyLookup,
) -> Result<AuthContext, AuthError> {
    // Extract prefix for lookup
    let prefix = if key.len() >= 16 {
        &key[..16]
    } else {
        return Err(AuthError::InvalidApiKey);
    };

    // Look up key by prefix
    let (org_id, key_hash, scopes) = lookup
        .lookup_api_key(prefix)
        .await
        .ok_or(AuthError::InvalidApiKey)?;

    // Verify key hash
    if !verify_api_key(key, &key_hash) {
        return Err(AuthError::InvalidApiKey);
    }

    Ok(AuthContext::from_api_key(org_id, scopes))
}

fn validate_session(token: &str, config: &AuthConfig) -> Result<AuthContext, AuthError> {
    let session = crate::verify_session(token, &config.jwt_secret)?;
    Ok(AuthContext::from_session(
        session.org_id,
        session.user_id,
        session.scopes,
    ))
}

fn extract_session_cookie(cookies: &str) -> Option<String> {
    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix("session=") {
            return Some(value.to_string());
        }
    }
    None
}

// Implement IntoResponse for AuthError
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AuthError::MissingAuth
            | AuthError::InvalidFormat
            | AuthError::InvalidApiKey
            | AuthError::ExpiredApiKey
            | AuthError::InvalidSession
            | AuthError::ExpiredSession => StatusCode::UNAUTHORIZED,
            AuthError::InsufficientScope { .. } => StatusCode::FORBIDDEN,
            AuthError::OrgNotFound | AuthError::UserNotFound => StatusCode::NOT_FOUND,
        };

        let body = serde_json::json!({
            "error": self.to_string(),
            "code": format!("{:?}", self).to_lowercase().replace(" ", "_"),
        });

        (status, axum::Json(body)).into_response()
    }
}

/// Extension trait to extract AuthContext from request
pub trait AuthContextExt {
    fn auth_context(&self) -> Option<&AuthContext>;
}

impl<B> AuthContextExt for Request<B> {
    fn auth_context(&self) -> Option<&AuthContext> {
        self.extensions().get::<AuthContext>()
    }
}

/// Extractor for AuthContext in handlers
#[derive(Clone)]
pub struct Auth(pub AuthContext);

#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .map(Auth)
            .ok_or(AuthError::MissingAuth)
    }
}
