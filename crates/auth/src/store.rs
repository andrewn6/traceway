//! Storage trait for auth-related data.
//!
//! This trait abstracts the persistence layer for organizations, users,
//! API keys, and invites. Implement this on your storage backend.

use async_trait::async_trait;

use crate::{ApiKey, ApiKeyId, Invite, OrgId, Organization, PasswordResetToken, Scope, User, UserId};

/// Error type for auth storage operations
#[derive(Debug, thiserror::Error)]
pub enum AuthStoreError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),
}

/// Storage trait for authentication data
#[async_trait]
pub trait AuthStore: Send + Sync {
    // --- Organization ---

    async fn save_org(&self, org: &Organization) -> Result<(), AuthStoreError>;

    async fn get_org(&self, id: OrgId) -> Result<Option<Organization>, AuthStoreError>;

    async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Organization>, AuthStoreError>;

    // --- User ---

    async fn save_user(&self, user: &User) -> Result<(), AuthStoreError>;

    async fn get_user(&self, id: UserId) -> Result<Option<User>, AuthStoreError>;

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthStoreError>;

    async fn list_users_for_org(&self, org_id: OrgId) -> Result<Vec<User>, AuthStoreError>;

    // --- API Key ---

    async fn save_api_key(&self, key: &ApiKey) -> Result<(), AuthStoreError>;

    async fn get_api_key(&self, id: ApiKeyId) -> Result<Option<ApiKey>, AuthStoreError>;

    async fn list_api_keys_for_org(&self, org_id: OrgId) -> Result<Vec<ApiKey>, AuthStoreError>;

    async fn lookup_api_key_by_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<ApiKey>, AuthStoreError>;

    async fn delete_api_key(&self, id: ApiKeyId) -> Result<bool, AuthStoreError>;

    async fn update_api_key_last_used(
        &self,
        id: ApiKeyId,
    ) -> Result<(), AuthStoreError>;

    // --- Invite ---

    async fn save_invite(&self, invite: &Invite) -> Result<(), AuthStoreError>;

    async fn get_invite_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Invite>, AuthStoreError>;

    async fn list_invites_for_org(&self, org_id: OrgId) -> Result<Vec<Invite>, AuthStoreError>;

    async fn delete_invite(&self, id: uuid::Uuid) -> Result<bool, AuthStoreError>;

    // --- Password Reset ---

    async fn save_password_reset(
        &self,
        token: &PasswordResetToken,
    ) -> Result<(), AuthStoreError>;

    async fn get_password_reset_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, AuthStoreError>;

    async fn mark_password_reset_used(
        &self,
        id: uuid::Uuid,
    ) -> Result<(), AuthStoreError>;
}
