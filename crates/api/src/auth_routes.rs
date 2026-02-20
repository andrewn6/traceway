//! Auth API endpoints.
//!
//! Local mode  → no auth, no signup/login, returns sensible defaults.
//! Cloud mode  → full signup/login/logout + API key CRUD backed by Postgres.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use auth::{
    generate_api_key, Auth, Organization, Role, Scope, User,
    create_session,
};

use crate::AppState;

// ── request / response types ─────────────────────────────────────────

#[derive(Serialize)]
pub struct MeResponse {
    pub org_id: String,
    pub user_id: Option<String>,
    pub scopes: Vec<Scope>,
    pub is_local_mode: bool,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub mode: String,
    pub features: Vec<String>,
}

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub org_name: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub org_id: String,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct OrgResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub plan: String,
}

#[derive(Serialize)]
pub struct MemberResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<Scope>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Serialize)]
pub struct ApiKeyCreatedResponse {
    pub id: String,
    /// Full key – only returned on creation, never stored.
    pub key: String,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<Scope>,
}

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<Scope>,
}

fn default_scopes() -> Vec<Scope> {
    Scope::default_sdk()
}

// ── helpers ──────────────────────────────────────────────────────────

/// Build a `Set-Cookie` header value for the session JWT.
fn session_cookie(token: &str) -> String {
    format!(
        "session={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800",
        token
    )
}

/// Build a `Set-Cookie` header that clears the session.
fn clear_session_cookie() -> String {
    "session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0".to_string()
}

fn slug_from_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn internal_err(e: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::error!("auth error: {}", e);
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

// ── handlers ─────────────────────────────────────────────────────────

/// GET /api/auth/config
async fn get_auth_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    let mode = if state.auth_config.local_mode {
        "local"
    } else {
        "cloud"
    };
    let features = if state.auth_config.local_mode {
        vec![]
    } else {
        vec![
            "auth".into(),
            "teams".into(),
            "api_keys".into(),
        ]
    };
    Json(ConfigResponse {
        mode: mode.into(),
        features,
    })
}

/// GET /api/auth/me
async fn get_me(Auth(ctx): Auth) -> Json<MeResponse> {
    Json(MeResponse {
        org_id: ctx.org_id.to_string(),
        user_id: ctx.user_id.map(|id| id.to_string()),
        scopes: ctx.scopes,
        is_local_mode: ctx.is_local_mode,
    })
}

/// POST /api/auth/signup – create org + user, return session cookie.
async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if state.auth_config.local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state
        .auth_store
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    // Check if email already taken
    if auth_store
        .get_user_by_email(&req.email)
        .await
        .map_err(internal_err)?
        .is_some()
    {
        return Err((StatusCode::CONFLICT, "Email already in use".into()));
    }

    // Create org
    let org_name = req.org_name.unwrap_or_else(|| format!("{}'s Org", req.email.split('@').next().unwrap_or("User")));
    let slug = slug_from_name(&org_name);
    let org = Organization::new(&org_name, &slug);

    auth_store.save_org(&org).await.map_err(internal_err)?;

    // Create user (Owner of the new org)
    let user = User::new(&req.email, org.id, Role::Owner)
        .with_password(&req.password);
    let mut user = user;
    user.name = req.name;

    auth_store.save_user(&user).await.map_err(internal_err)?;

    // Issue session JWT
    let token = create_session(user.id, org.id, Scope::all(), &state.auth_config.jwt_secret)
        .map_err(|e| internal_err(format!("Failed to create session: {}", e)))?;

    let body = AuthResponse {
        user_id: user.id.to_string(),
        org_id: org.id.to_string(),
        email: user.email,
        name: user.name,
        role: "owner".into(),
    };

    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, session_cookie(&token))],
        Json(body),
    ))
}

/// POST /api/auth/login – verify password, return session cookie.
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if state.auth_config.local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state
        .auth_store
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    let user = auth_store
        .get_user_by_email(&req.email)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid email or password".into()))?;

    if !user.verify_password(&req.password) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password".into()));
    }

    let token = create_session(user.id, user.org_id, Scope::all(), &state.auth_config.jwt_secret)
        .map_err(|e| internal_err(format!("Failed to create session: {}", e)))?;

    let body = AuthResponse {
        user_id: user.id.to_string(),
        org_id: user.org_id.to_string(),
        email: user.email,
        name: user.name,
        role: format!("{:?}", user.role).to_lowercase(),
    };

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, session_cookie(&token))],
        Json(body),
    ))
}

/// POST /api/auth/logout – clear session cookie.
async fn logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::SET_COOKIE, clear_session_cookie())],
        Json(serde_json::json!({ "ok": true })),
    )
}

// ── org endpoints ────────────────────────────────────────────────────

/// GET /api/org
async fn get_org(
    Auth(ctx): Auth,
    State(state): State<AppState>,
) -> Result<Json<OrgResponse>, (StatusCode, String)> {
    if ctx.is_local_mode {
        return Ok(Json(OrgResponse {
            id: ctx.org_id.to_string(),
            name: "Local".into(),
            slug: "local".into(),
            plan: "free".into(),
        }));
    }

    let auth_store = state.auth_store.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into())
    })?;

    let org = auth_store
        .get_org(ctx.org_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Organization not found".into()))?;

    Ok(Json(OrgResponse {
        id: org.id.to_string(),
        name: org.name,
        slug: org.slug,
        plan: format!("{:?}", org.plan).to_lowercase(),
    }))
}

/// GET /api/org/members
async fn list_members(
    Auth(ctx): Auth,
    State(state): State<AppState>,
) -> Result<Json<Vec<MemberResponse>>, (StatusCode, String)> {
    if ctx.is_local_mode {
        return Ok(Json(vec![]));
    }

    let auth_store = state.auth_store.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into())
    })?;

    let users = auth_store
        .list_users_for_org(ctx.org_id)
        .await
        .map_err(internal_err)?;

    Ok(Json(
        users
            .into_iter()
            .map(|u| MemberResponse {
                id: u.id.to_string(),
                email: u.email,
                name: u.name,
                role: format!("{:?}", u.role).to_lowercase(),
            })
            .collect(),
    ))
}

// ── api key endpoints ────────────────────────────────────────────────

/// GET /api/org/api-keys
async fn list_api_keys(
    Auth(ctx): Auth,
    State(state): State<AppState>,
) -> Result<Json<Vec<ApiKeyResponse>>, (StatusCode, String)> {
    if ctx.is_local_mode {
        return Ok(Json(vec![]));
    }

    let auth_store = state.auth_store.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into())
    })?;

    let keys = auth_store
        .list_api_keys_for_org(ctx.org_id)
        .await
        .map_err(internal_err)?;

    Ok(Json(
        keys.into_iter()
            .map(|k| ApiKeyResponse {
                id: k.id.to_string(),
                name: k.name,
                key_prefix: k.key_prefix,
                scopes: k.scopes,
                created_at: k.created_at.to_rfc3339(),
                last_used_at: k.last_used_at.map(|t| t.to_rfc3339()),
            })
            .collect(),
    ))
}

/// POST /api/org/api-keys
async fn create_api_key_handler(
    Auth(ctx): Auth,
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyCreatedResponse>), (StatusCode, String)> {
    if ctx.is_local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state.auth_store.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into())
    })?;

    let (generated, stored) = generate_api_key(ctx.org_id, req.name.clone(), req.scopes.clone());

    auth_store
        .save_api_key(&stored)
        .await
        .map_err(internal_err)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiKeyCreatedResponse {
            id: generated.id.to_string(),
            key: generated.key,
            name: req.name,
            key_prefix: generated.key_prefix,
            scopes: req.scopes,
        }),
    ))
}

/// DELETE /api/org/api-keys/:id
async fn delete_api_key_handler(
    Auth(ctx): Auth,
    State(state): State<AppState>,
    axum::extract::Path(key_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if ctx.is_local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state.auth_store.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into())
    })?;

    let id: uuid::Uuid = key_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid key ID".into()))?;

    // Verify the key belongs to the caller's org
    if let Some(key) = auth_store.get_api_key(id).await.map_err(internal_err)? {
        if key.org_id != ctx.org_id {
            return Err((StatusCode::FORBIDDEN, "Not your key".into()));
        }
    }

    let deleted = auth_store
        .delete_api_key(id)
        .await
        .map_err(internal_err)?;

    if deleted {
        Ok(StatusCode::OK)
    } else {
        Err((StatusCode::NOT_FOUND, "API key not found".into()))
    }
}

// ── routers ──────────────────────────────────────────────────────────

/// Public auth routes (no auth middleware needed).
pub fn public_auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/config", get(get_auth_config))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
}

/// Protected auth routes (auth middleware must be applied by caller).
pub fn protected_auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(get_me))
        .route("/org", get(get_org))
        .route(
            "/org/api-keys",
            get(list_api_keys).post(create_api_key_handler),
        )
        .route("/org/api-keys/:id", delete(delete_api_key_handler))
        .route("/org/members", get(list_members))
}
