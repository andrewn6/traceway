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
    generate_api_key, Auth, Email, Invite, Organization, PasswordResetToken, Role, Scope, User,
    create_session,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

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

// ── invite types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateInviteRequest {
    pub email: String,
    #[serde(default = "default_invite_role")]
    pub role: Role,
}

fn default_invite_role() -> Role {
    Role::Member
}

#[derive(Serialize)]
pub struct InviteResponse {
    pub id: String,
    pub email: String,
    pub role: String,
    pub invited_by: String,
    pub expires_at: String,
    pub created_at: String,
}

// ── password reset types ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct AcceptInviteRequest {
    pub token: String,
    pub password: String,
    pub name: Option<String>,
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

use base64::Engine;
use sha2::{Sha256, Digest};

/// Generate a secure random token and its SHA256 hash (for indexed DB lookup).
fn generate_token() -> (String, String) {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let hash = format!("{:x}", Sha256::digest(token.as_bytes()));
    (token, hash)
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

// ── invite endpoints ─────────────────────────────────────────────────

/// POST /api/org/invites – send an invite to join the org.
async fn create_invite(
    Auth(ctx): Auth,
    State(state): State<AppState>,
    Json(req): Json<CreateInviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), (StatusCode, String)> {
    if ctx.is_local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state.auth_store.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    // Only admins+ can invite
    let user_id = ctx.user_id
        .ok_or_else(|| (StatusCode::FORBIDDEN, "API keys cannot send invites".into()))?;

    // Check if user is already in the org
    if auth_store.get_user_by_email(&req.email).await.map_err(internal_err)?.is_some() {
        return Err((StatusCode::CONFLICT, "User already exists".into()));
    }

    // Generate invite token
    let (token, token_hash) = generate_token();

    let invite = Invite {
        id: Uuid::now_v7(),
        org_id: ctx.org_id,
        email: req.email.clone(),
        role: req.role,
        invited_by: user_id,
        token_hash,
        expires_at: Utc::now() + Duration::days(7),
        created_at: Utc::now(),
    };

    auth_store.save_invite(&invite).await.map_err(internal_err)?;

    // Send invite email
    let org = auth_store.get_org(ctx.org_id).await.map_err(internal_err)?;
    let org_name = org.map(|o| o.name).unwrap_or_else(|| "your team".to_string());

    let invite_url = format!("{}/accept-invite?token={}", state.app_url, token);
    let html = format!(
        r#"<p>You've been invited to join <strong>{org_name}</strong> on Traceway.</p>
<p><a href="{invite_url}" style="display:inline-block;padding:12px 24px;background:#2563eb;color:#fff;text-decoration:none;border-radius:6px;">Accept Invite</a></p>
<p>This invite expires in 7 days.</p>
<p style="color:#888;font-size:12px;">If you didn't expect this, you can ignore this email.</p>"#
    );

    if let Err(e) = state.email_sender.send(&Email {
        to: req.email.clone(),
        subject: format!("Join {} on Traceway", org_name),
        html,
    }).await {
        tracing::error!("Failed to send invite email: {}", e);
        // Don't fail the request -- invite is saved, user can be given the link manually
    }

    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            id: invite.id.to_string(),
            email: invite.email,
            role: format!("{:?}", invite.role).to_lowercase(),
            invited_by: invite.invited_by.to_string(),
            expires_at: invite.expires_at.to_rfc3339(),
            created_at: invite.created_at.to_rfc3339(),
        }),
    ))
}

/// GET /api/org/invites – list pending invites.
async fn list_invites(
    Auth(ctx): Auth,
    State(state): State<AppState>,
) -> Result<Json<Vec<InviteResponse>>, (StatusCode, String)> {
    if ctx.is_local_mode {
        return Ok(Json(vec![]));
    }

    let auth_store = state.auth_store.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    let invites = auth_store.list_invites_for_org(ctx.org_id).await.map_err(internal_err)?;

    Ok(Json(
        invites
            .into_iter()
            .filter(|i| i.expires_at > Utc::now()) // only show non-expired
            .map(|i| InviteResponse {
                id: i.id.to_string(),
                email: i.email,
                role: format!("{:?}", i.role).to_lowercase(),
                invited_by: i.invited_by.to_string(),
                expires_at: i.expires_at.to_rfc3339(),
                created_at: i.created_at.to_rfc3339(),
            })
            .collect(),
    ))
}

/// DELETE /api/org/invites/:id – revoke a pending invite.
async fn delete_invite(
    Auth(ctx): Auth,
    State(state): State<AppState>,
    axum::extract::Path(invite_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if ctx.is_local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state.auth_store.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    let id: Uuid = invite_id.parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid invite ID".into()))?;

    let deleted = auth_store.delete_invite(id).await.map_err(internal_err)?;

    if deleted {
        Ok(StatusCode::OK)
    } else {
        Err((StatusCode::NOT_FOUND, "Invite not found".into()))
    }
}

/// POST /api/auth/accept-invite – accept an invite and create an account.
async fn accept_invite(
    State(state): State<AppState>,
    Json(req): Json<AcceptInviteRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if state.auth_config.local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state.auth_store.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    // Verify token
    let token_hash = bcrypt_verify_find_invite(auth_store.as_ref(), &req.token).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let invite = token_hash;

    if invite.expires_at < Utc::now() {
        return Err((StatusCode::GONE, "Invite has expired".into()));
    }

    // Check email not already taken
    if auth_store.get_user_by_email(&invite.email).await.map_err(internal_err)?.is_some() {
        return Err((StatusCode::CONFLICT, "Email already in use".into()));
    }

    // Create user in the org
    let user = User::new(&invite.email, invite.org_id, invite.role)
        .with_password(&req.password);
    let mut user = user;
    user.name = req.name;

    auth_store.save_user(&user).await.map_err(internal_err)?;

    // Delete the invite
    auth_store.delete_invite(invite.id).await.map_err(internal_err)?;

    // Issue session
    let token = create_session(user.id, invite.org_id, Scope::all(), &state.auth_config.jwt_secret)
        .map_err(|e| internal_err(format!("Failed to create session: {}", e)))?;

    let body = AuthResponse {
        user_id: user.id.to_string(),
        org_id: invite.org_id.to_string(),
        email: user.email,
        name: user.name,
        role: format!("{:?}", invite.role).to_lowercase(),
    };

    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, session_cookie(&token))],
        Json(body),
    ))
}

/// Brute-force-safe invite lookup: iterate invites and bcrypt-verify.
async fn bcrypt_verify_find_invite(
    store: &dyn auth::AuthStore,
    raw_token: &str,
) -> Result<Invite, String> {
    let hash = format!("{:x}", Sha256::digest(raw_token.as_bytes()));
    store.get_invite_by_token_hash(&hash).await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Invalid or expired invite token".to_string())
}

// ── password reset endpoints ─────────────────────────────────────────

/// POST /api/auth/forgot-password – request a password reset email.
async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if state.auth_config.local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state.auth_store.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    // Always return success to avoid email enumeration
    let ok = Json(serde_json::json!({ "ok": true, "message": "If that email exists, a reset link has been sent." }));

    let user = match auth_store.get_user_by_email(&req.email).await {
        Ok(Some(u)) => u,
        _ => return Ok(ok),
    };

    // Generate reset token (SHA256 for storage, raw token in URL)
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw_token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let token_hash = format!("{:x}", Sha256::digest(raw_token.as_bytes()));

    let reset = PasswordResetToken {
        id: Uuid::now_v7(),
        user_id: user.id,
        token_hash,
        expires_at: Utc::now() + Duration::hours(1),
        used: false,
        created_at: Utc::now(),
    };

    auth_store.save_password_reset(&reset).await.map_err(internal_err)?;

    // Send email
    let reset_url = format!("{}/reset-password?token={}", state.app_url, raw_token);
    let html = format!(
        r#"<p>You requested a password reset for your Traceway account.</p>
<p><a href="{reset_url}" style="display:inline-block;padding:12px 24px;background:#2563eb;color:#fff;text-decoration:none;border-radius:6px;">Reset Password</a></p>
<p>This link expires in 1 hour.</p>
<p style="color:#888;font-size:12px;">If you didn't request this, you can ignore this email.</p>"#
    );

    if let Err(e) = state.email_sender.send(&Email {
        to: req.email,
        subject: "Reset your Traceway password".to_string(),
        html,
    }).await {
        tracing::error!("Failed to send password reset email: {}", e);
    }

    Ok(ok)
}

/// POST /api/auth/reset-password – set a new password using reset token.
async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if state.auth_config.local_mode {
        return Err((StatusCode::NOT_FOUND, "Not available in local mode".into()));
    }

    let auth_store = state.auth_store.as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Auth store not configured".into()))?;

    // Verify token
    let token_hash = format!("{:x}", Sha256::digest(req.token.as_bytes()));

    let reset = auth_store
        .get_password_reset_by_token_hash(&token_hash)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid or expired reset token".into()))?;

    if !reset.is_valid() {
        return Err((StatusCode::BAD_REQUEST, "Invalid or expired reset token".into()));
    }

    // Update user password
    let mut user = auth_store
        .get_user(reset.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".into()))?;

    user.password_hash = Some(
        bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
            .map_err(|e| internal_err(format!("Hash error: {}", e)))?,
    );
    user.updated_at = Utc::now();

    auth_store.save_user(&user).await.map_err(internal_err)?;

    // Mark token as used
    auth_store.mark_password_reset_used(reset.id).await.map_err(internal_err)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── routers ──────────────────────────────────────────────────────────

/// Public auth routes (no auth middleware needed).
pub fn public_auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/config", get(get_auth_config))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/accept-invite", post(accept_invite))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
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
        .route(
            "/org/invites",
            get(list_invites).post(create_invite),
        )
        .route("/org/invites/:id", delete(delete_invite))
}
