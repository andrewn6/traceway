//! Postgres storage backend for Traceway cloud auth.
//!
//! Handles all user-facing data in cloud mode: organizations, users,
//! API keys, invites. Trace data stays in Turbopuffer/SQLite — this
//! crate only owns the auth/identity layer.

pub mod migrations;

use async_trait::async_trait;
use auth::{
    ApiKey, ApiKeyId, AuthStore, AuthStoreError, Invite, OrgId, Organization, Role, Scope,
    User, UserId,
};
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::{error, info};

/// Postgres-backed auth store.
pub struct PostgresAuthStore {
    pool: PgPool,
}

impl PostgresAuthStore {
    /// Connect to Postgres from a URL (e.g. `DATABASE_URL` env var).
    pub async fn connect(database_url: &str) -> Result<Self, AuthStoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| AuthStoreError::Database(e.to_string()))?;

        info!("Connected to Postgres");
        Ok(Self { pool })
    }

    /// Connect from `DATABASE_URL` env var.
    pub async fn from_env() -> Result<Self, AuthStoreError> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| AuthStoreError::Database("DATABASE_URL not set".into()))?;
        Self::connect(&url).await
    }

    /// Run migrations.
    pub async fn migrate(&self) -> Result<(), AuthStoreError> {
        migrations::run(&self.pool).await
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

// ── helpers ──────────────────────────────────────────────────────────

fn db_err(e: sqlx::Error) -> AuthStoreError {
    AuthStoreError::Database(e.to_string())
}

fn scopes_to_json(scopes: &[Scope]) -> serde_json::Value {
    serde_json::to_value(scopes).unwrap_or_default()
}

fn scopes_from_json(v: serde_json::Value) -> Vec<Scope> {
    serde_json::from_value(v).unwrap_or_default()
}

fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Admin => "admin",
        Role::Member => "member",
        Role::ReadOnly => "read_only",
    }
}

fn role_from_str(s: &str) -> Role {
    match s {
        "owner" => Role::Owner,
        "admin" => Role::Admin,
        "member" => Role::Member,
        _ => Role::ReadOnly,
    }
}

fn plan_to_str(plan: auth::Plan) -> &'static str {
    match plan {
        auth::Plan::Free => "free",
        auth::Plan::Pro => "pro",
        auth::Plan::Team => "team",
        auth::Plan::Enterprise => "enterprise",
    }
}

fn plan_from_str(s: &str) -> auth::Plan {
    match s {
        "pro" => auth::Plan::Pro,
        "team" => auth::Plan::Team,
        "enterprise" => auth::Plan::Enterprise,
        _ => auth::Plan::Free,
    }
}

// ── AuthStore impl ───────────────────────────────────────────────────

#[async_trait]
impl AuthStore for PostgresAuthStore {
    // ── Organization ─────────────────────────────────────────────────

    async fn save_org(&self, org: &Organization) -> Result<(), AuthStoreError> {
        sqlx::query(
            r#"INSERT INTO organizations (id, name, slug, plan, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (id) DO UPDATE SET
                 name = EXCLUDED.name,
                 slug = EXCLUDED.slug,
                 plan = EXCLUDED.plan,
                 updated_at = EXCLUDED.updated_at"#,
        )
        .bind(org.id)
        .bind(&org.name)
        .bind(&org.slug)
        .bind(plan_to_str(org.plan))
        .bind(org.created_at)
        .bind(org.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn get_org(&self, id: OrgId) -> Result<Option<Organization>, AuthStoreError> {
        let row = sqlx::query_as::<_, OrgRow>(
            "SELECT id, name, slug, plan, created_at, updated_at FROM organizations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Organization>, AuthStoreError> {
        let row = sqlx::query_as::<_, OrgRow>(
            "SELECT id, name, slug, plan, created_at, updated_at FROM organizations WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| r.into()))
    }

    // ── User ─────────────────────────────────────────────────────────

    async fn save_user(&self, user: &User) -> Result<(), AuthStoreError> {
        sqlx::query(
            r#"INSERT INTO users (id, email, name, password_hash, org_id, role, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (id) DO UPDATE SET
                 email = EXCLUDED.email,
                 name = EXCLUDED.name,
                 password_hash = EXCLUDED.password_hash,
                 role = EXCLUDED.role,
                 updated_at = EXCLUDED.updated_at"#,
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(user.org_id)
        .bind(role_to_str(user.role))
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn get_user(&self, id: UserId) -> Result<Option<User>, AuthStoreError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, email, name, password_hash, org_id, role, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthStoreError> {
        let row = sqlx::query_as::<_, UserRow>(
            "SELECT id, email, name, password_hash, org_id, role, created_at, updated_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| r.into()))
    }

    async fn list_users_for_org(&self, org_id: OrgId) -> Result<Vec<User>, AuthStoreError> {
        let rows = sqlx::query_as::<_, UserRow>(
            "SELECT id, email, name, password_hash, org_id, role, created_at, updated_at FROM users WHERE org_id = $1 ORDER BY created_at",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // ── API Key ──────────────────────────────────────────────────────

    async fn save_api_key(&self, key: &ApiKey) -> Result<(), AuthStoreError> {
        sqlx::query(
            r#"INSERT INTO api_keys (id, org_id, name, key_prefix, key_hash, scopes, created_at, last_used_at, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (id) DO UPDATE SET
                 name = EXCLUDED.name,
                 scopes = EXCLUDED.scopes,
                 last_used_at = EXCLUDED.last_used_at"#,
        )
        .bind(key.id)
        .bind(key.org_id)
        .bind(&key.name)
        .bind(&key.key_prefix)
        .bind(&key.key_hash)
        .bind(scopes_to_json(&key.scopes))
        .bind(key.created_at)
        .bind(key.last_used_at)
        .bind(key.expires_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn get_api_key(&self, id: ApiKeyId) -> Result<Option<ApiKey>, AuthStoreError> {
        let row = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, org_id, name, key_prefix, key_hash, scopes, created_at, last_used_at, expires_at FROM api_keys WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| r.into()))
    }

    async fn list_api_keys_for_org(&self, org_id: OrgId) -> Result<Vec<ApiKey>, AuthStoreError> {
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, org_id, name, key_prefix, key_hash, scopes, created_at, last_used_at, expires_at FROM api_keys WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn lookup_api_key_by_prefix(&self, prefix: &str) -> Result<Option<ApiKey>, AuthStoreError> {
        let row = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, org_id, name, key_prefix, key_hash, scopes, created_at, last_used_at, expires_at FROM api_keys WHERE key_prefix = $1",
        )
        .bind(prefix)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| r.into()))
    }

    async fn delete_api_key(&self, id: ApiKeyId) -> Result<bool, AuthStoreError> {
        let result = sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn update_api_key_last_used(&self, id: ApiKeyId) -> Result<(), AuthStoreError> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    // ── Invite ───────────────────────────────────────────────────────

    async fn save_invite(&self, invite: &Invite) -> Result<(), AuthStoreError> {
        sqlx::query(
            r#"INSERT INTO invites (id, org_id, email, role, invited_by, token_hash, expires_at, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(invite.id)
        .bind(invite.org_id)
        .bind(&invite.email)
        .bind(role_to_str(invite.role))
        .bind(invite.invited_by)
        .bind(&invite.token_hash)
        .bind(invite.expires_at)
        .bind(invite.created_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn get_invite_by_token_hash(&self, token_hash: &str) -> Result<Option<Invite>, AuthStoreError> {
        let row = sqlx::query_as::<_, InviteRow>(
            "SELECT id, org_id, email, role, invited_by, token_hash, expires_at, created_at FROM invites WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(row.map(|r| r.into()))
    }

    async fn list_invites_for_org(&self, org_id: OrgId) -> Result<Vec<Invite>, AuthStoreError> {
        let rows = sqlx::query_as::<_, InviteRow>(
            "SELECT id, org_id, email, role, invited_by, token_hash, expires_at, created_at FROM invites WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_invite(&self, id: uuid::Uuid) -> Result<bool, AuthStoreError> {
        let result = sqlx::query("DELETE FROM invites WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}

// ── Row types for sqlx ───────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct OrgRow {
    id: uuid::Uuid,
    name: String,
    slug: String,
    plan: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OrgRow> for Organization {
    fn from(r: OrgRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            slug: r.slug,
            plan: plan_from_str(&r.plan),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: uuid::Uuid,
    email: String,
    name: Option<String>,
    password_hash: Option<String>,
    org_id: uuid::Uuid,
    role: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        Self {
            id: r.id,
            email: r.email,
            name: r.name,
            password_hash: r.password_hash,
            org_id: r.org_id,
            role: role_from_str(&r.role),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: uuid::Uuid,
    org_id: uuid::Uuid,
    name: String,
    key_prefix: String,
    key_hash: String,
    scopes: serde_json::Value,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
}

impl From<ApiKeyRow> for ApiKey {
    fn from(r: ApiKeyRow) -> Self {
        Self {
            id: r.id,
            org_id: r.org_id,
            name: r.name,
            key_prefix: r.key_prefix,
            key_hash: r.key_hash,
            scopes: scopes_from_json(r.scopes),
            created_at: r.created_at,
            last_used_at: r.last_used_at,
            expires_at: r.expires_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct InviteRow {
    id: uuid::Uuid,
    org_id: uuid::Uuid,
    email: String,
    role: String,
    invited_by: uuid::Uuid,
    token_hash: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl From<InviteRow> for Invite {
    fn from(r: InviteRow) -> Self {
        Self {
            id: r.id,
            org_id: r.org_id,
            email: r.email,
            role: role_from_str(&r.role),
            invited_by: r.invited_by,
            token_hash: r.token_hash,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}
