use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod api_key;
pub mod context;
pub mod middleware;
pub mod session;

// Re-exports
pub use api_key::{ApiKey, ApiKeyId, generate_api_key, hash_api_key, verify_api_key};
pub use context::{AuthContext, AuthError};
pub use middleware::{Auth, AuthConfig, ApiKeyLookup};
pub use session::{SessionToken, create_session, verify_session};

// --- ID Types ---

pub type OrgId = Uuid;
pub type UserId = Uuid;

// --- Organization ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrgId,
    pub name: String,
    pub slug: String,
    pub plan: Plan,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    pub fn new(name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            name: name.into(),
            slug: slug.into(),
            plan: Plan::Free,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create the implicit local organization (for local mode)
    pub fn local() -> Self {
        Self {
            id: Uuid::nil(),
            name: "Local".to_string(),
            slug: "local".to_string(),
            plan: Plan::Free,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

// --- User ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub name: Option<String>,
    pub org_id: OrgId,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: impl Into<String>, org_id: OrgId, role: Role) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            email: email.into(),
            name: None,
            org_id,
            role,
            created_at: now,
            updated_at: now,
        }
    }
}

// --- Role ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    Member,
    ReadOnly,
}

impl Role {
    pub fn can_write(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin | Role::Member)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_manage_org(&self) -> bool {
        matches!(self, Role::Owner)
    }
}

// --- Scope ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    TracesRead,
    TracesWrite,
    DatasetsRead,
    DatasetsWrite,
    AnalyticsRead,
    Admin,
}

impl Scope {
    pub fn all() -> Vec<Scope> {
        vec![
            Scope::TracesRead,
            Scope::TracesWrite,
            Scope::DatasetsRead,
            Scope::DatasetsWrite,
            Scope::AnalyticsRead,
            Scope::Admin,
        ]
    }

    pub fn default_sdk() -> Vec<Scope> {
        vec![
            Scope::TracesRead,
            Scope::TracesWrite,
            Scope::DatasetsRead,
            Scope::DatasetsWrite,
            Scope::AnalyticsRead,
        ]
    }

    pub fn read_only() -> Vec<Scope> {
        vec![
            Scope::TracesRead,
            Scope::DatasetsRead,
            Scope::AnalyticsRead,
        ]
    }
}

// --- Plan ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    #[default]
    Free,
    Pro,
    Team,
    Enterprise,
}

impl Plan {
    pub fn spans_per_month(&self) -> u64 {
        match self {
            Plan::Free => 10_000,
            Plan::Pro => 1_000_000,
            Plan::Team => 10_000_000,
            Plan::Enterprise => u64::MAX,
        }
    }

    pub fn max_team_members(&self) -> usize {
        match self {
            Plan::Free => 1,
            Plan::Pro => 5,
            Plan::Team => 50,
            Plan::Enterprise => usize::MAX,
        }
    }

    pub fn retention_days(&self) -> u32 {
        match self {
            Plan::Free => 7,
            Plan::Pro => 30,
            Plan::Team => 90,
            Plan::Enterprise => 365,
        }
    }
}

// --- Invite ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: Uuid,
    pub org_id: OrgId,
    pub email: String,
    pub role: Role,
    pub invited_by: UserId,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
