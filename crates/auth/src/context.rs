use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{OrgId, Scope, UserId};

/// Authentication context attached to each request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub org_id: OrgId,
    pub user_id: Option<UserId>,
    pub scopes: Vec<Scope>,
    pub is_local_mode: bool,
    pub is_api_key: bool,
}

impl AuthContext {
    /// Create context for local mode (no auth required)
    pub fn local() -> Self {
        Self {
            org_id: Uuid::nil(), // Nil UUID for local org
            user_id: None,
            scopes: Scope::all(),
            is_local_mode: true,
            is_api_key: false,
        }
    }

    /// Create context from API key authentication
    pub fn from_api_key(org_id: OrgId, scopes: Vec<Scope>) -> Self {
        Self {
            org_id,
            user_id: None,
            scopes,
            is_local_mode: false,
            is_api_key: true,
        }
    }

    /// Create context from session (dashboard user)
    pub fn from_session(org_id: OrgId, user_id: UserId, scopes: Vec<Scope>) -> Self {
        Self {
            org_id,
            user_id: Some(user_id),
            scopes,
            is_local_mode: false,
            is_api_key: false,
        }
    }

    /// Check if context has a specific scope
    pub fn has_scope(&self, scope: Scope) -> bool {
        self.is_local_mode || self.scopes.contains(&scope)
    }

    /// Check if context can read traces
    pub fn can_read_traces(&self) -> bool {
        self.has_scope(Scope::TracesRead)
    }

    /// Check if context can write traces
    pub fn can_write_traces(&self) -> bool {
        self.has_scope(Scope::TracesWrite)
    }

    /// Check if context can read datasets
    pub fn can_read_datasets(&self) -> bool {
        self.has_scope(Scope::DatasetsRead)
    }

    /// Check if context can write datasets
    pub fn can_write_datasets(&self) -> bool {
        self.has_scope(Scope::DatasetsWrite)
    }

    /// Check if context can read analytics
    pub fn can_read_analytics(&self) -> bool {
        self.has_scope(Scope::AnalyticsRead)
    }

    /// Check if context has admin access
    pub fn is_admin(&self) -> bool {
        self.has_scope(Scope::Admin)
    }
}

/// Authentication errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    #[error("missing authorization header")]
    MissingAuth,

    #[error("invalid authorization header format")]
    InvalidFormat,

    #[error("invalid API key")]
    InvalidApiKey,

    #[error("API key expired")]
    ExpiredApiKey,

    #[error("invalid session")]
    InvalidSession,

    #[error("session expired")]
    ExpiredSession,

    #[error("insufficient permissions: requires {required:?}")]
    InsufficientScope { required: Scope },

    #[error("organization not found")]
    OrgNotFound,

    #[error("user not found")]
    UserNotFound,
}

impl AuthError {
    pub fn status_code(&self) -> u16 {
        match self {
            AuthError::MissingAuth => 401,
            AuthError::InvalidFormat => 401,
            AuthError::InvalidApiKey => 401,
            AuthError::ExpiredApiKey => 401,
            AuthError::InvalidSession => 401,
            AuthError::ExpiredSession => 401,
            AuthError::InsufficientScope { .. } => 403,
            AuthError::OrgNotFound => 404,
            AuthError::UserNotFound => 404,
        }
    }
}
