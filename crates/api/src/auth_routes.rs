//! Auth API endpoints for cloud mode
//! 
//! These endpoints are only functional in cloud mode.
//! In local mode, they return appropriate defaults or errors.

use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use auth::{
    generate_api_key, Auth, AuthConfig, AuthContext, Organization, Scope, User,
};

use crate::AppState;

// --- Request/Response types ---

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub org_id: String,
    pub user_id: Option<String>,
    pub scopes: Vec<Scope>,
    pub is_local_mode: bool,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<Scope>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyCreatedResponse {
    pub id: String,
    pub key: String,  // Full key - only returned on creation
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<Scope>,
}

fn default_scopes() -> Vec<Scope> {
    Scope::default_sdk()
}

#[derive(Debug, Serialize)]
pub struct OrgResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub plan: String,
}

#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub mode: String,
    pub features: Vec<String>,
}

// --- Handlers ---

/// GET /api/auth/me - Get current auth context
async fn get_me(Auth(ctx): Auth) -> Json<MeResponse> {
    Json(MeResponse {
        org_id: ctx.org_id.to_string(),
        user_id: ctx.user_id.map(|id| id.to_string()),
        scopes: ctx.scopes,
        is_local_mode: ctx.is_local_mode,
    })
}

/// GET /api/auth/config - Get auth configuration
async fn get_auth_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    let mode = if state.auth_config.local_mode { "local" } else { "cloud" };
    let features = if state.auth_config.local_mode {
        vec![]
    } else {
        vec![
            "auth".to_string(),
            "teams".to_string(),
            "api_keys".to_string(),
        ]
    };
    Json(ConfigResponse {
        mode: mode.to_string(),
        features,
    })
}

/// GET /api/org - Get current organization
async fn get_org(Auth(ctx): Auth) -> Result<Json<OrgResponse>, StatusCode> {
    if ctx.is_local_mode {
        return Ok(Json(OrgResponse {
            id: ctx.org_id.to_string(),
            name: "Local".to_string(),
            slug: "local".to_string(),
            plan: "free".to_string(),
        }));
    }
    
    // In cloud mode, would look up from storage
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/org/api-keys - List API keys for org
async fn list_api_keys(Auth(ctx): Auth) -> Result<Json<Vec<ApiKeyResponse>>, StatusCode> {
    if ctx.is_local_mode {
        // No API keys in local mode
        return Ok(Json(vec![]));
    }
    
    // In cloud mode, would look up from storage
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// POST /api/org/api-keys - Create a new API key
async fn create_api_key(
    Auth(ctx): Auth,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyCreatedResponse>), StatusCode> {
    if ctx.is_local_mode {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    
    let (generated, _stored) = generate_api_key(ctx.org_id, req.name.clone(), req.scopes.clone());
    
    // In cloud mode, would save to storage
    
    Ok((StatusCode::CREATED, Json(ApiKeyCreatedResponse {
        id: generated.id.to_string(),
        key: generated.key,
        name: req.name,
        key_prefix: generated.key_prefix,
        scopes: req.scopes,
    })))
}

/// DELETE /api/org/api-keys/:id - Delete an API key
async fn delete_api_key(
    Auth(ctx): Auth,
    axum::extract::Path(key_id): axum::extract::Path<String>,
) -> StatusCode {
    if ctx.is_local_mode {
        return StatusCode::NOT_IMPLEMENTED;
    }
    
    // In cloud mode, would delete from storage
    let _ = key_id;
    StatusCode::NOT_IMPLEMENTED
}

/// GET /api/org/members - List org members
async fn list_members(Auth(ctx): Auth) -> Result<Json<Vec<MemberResponse>>, StatusCode> {
    if ctx.is_local_mode {
        return Ok(Json(vec![]));
    }
    
    // In cloud mode, would look up from storage
    Err(StatusCode::NOT_IMPLEMENTED)
}

// --- Router ---

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(get_me))
        .route("/auth/config", get(get_auth_config))
        .route("/org", get(get_org))
        .route("/org/api-keys", get(list_api_keys).post(create_api_key))
        .route("/org/api-keys/:id", delete(delete_api_key))
        .route("/org/members", get(list_members))
}
