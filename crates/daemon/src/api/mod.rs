pub mod any_backend;
pub mod auth_keys;
pub mod capture;
pub mod event_log;
pub mod events;
pub mod metrics;
pub mod org_store;
pub mod otlp;

pub use org_store::OrgStoreManager;

use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, watch, RwLock};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

pub use any_backend::AnyBackend;
use trace::{
    CaptureRuleId, Datapoint, Dataset, DatasetId, EvalRun, FileVersion, QueueItem, Span, SpanId,
    Trace, TraceId,
};

// --- Events ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SystemEvent {
    SpanCreated { span: Span },
    SpanCompleted { span: Span },
    SpanFailed { span: Span },
    TraceCreated { trace: Trace },
    TraceCompleted { trace: Trace },
    FileVersionCreated { file: FileVersion },
    SpanDeleted { span_id: SpanId },
    TraceDeleted { trace_id: TraceId },
    DatasetCreated { dataset: Dataset },
    DatasetDeleted { dataset_id: DatasetId },
    DatapointCreated { datapoint: Datapoint },
    QueueItemUpdated { item: QueueItem },
    EvalRunCreated { run: EvalRun },
    EvalRunUpdated { run: EvalRun },
    EvalRunCompleted { run: EvalRun },
    CaptureRuleFired { rule_id: CaptureRuleId, datapoint: Datapoint },
    Cleared,
}

// --- App State ---

#[derive(Clone)]
pub struct AppState {
    pub org_stores: Arc<OrgStoreManager>,
    pub events_tx: broadcast::Sender<SystemEvent>,
    /// Durable event log for SSE replay on reconnect.
    pub event_log: Arc<dyn events::EventLog>,
    pub start_time: Instant,
    pub config: Arc<RwLock<serde_json::Value>>,
    pub config_path: Arc<String>,
    pub shutdown_tx: Option<watch::Sender<bool>>,
    pub auth_config: auth::AuthConfig,
    pub api_key_lookup: Arc<dyn auth::ApiKeyLookup>,
}

impl AppState {
    /// Emit a system event: broadcast to live SSE subscribers AND append to durable log.
    pub fn emit_event(&self, event: SystemEvent, org_id: &str) {
        let _ = self.events_tx.send(event.clone());
        let log = self.event_log.clone();
        let org_id = org_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = log.append(&org_id, &event).await {
                tracing::warn!("failed to append event to log: {e}");
            }
        });
    }

    /// Get the store for a given org. Returns `Err((StatusCode, String))` on failure.
    /// Prefer `store_for_project` in new code.
    pub async fn store_for_org(&self, org_id: auth::OrgId) -> Result<SharedStore, (StatusCode, String)> {
        self.org_stores.get(org_id).await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, e)
        })
    }

    /// Get the store for a given org + project. Returns `Err((StatusCode, String))` on failure.
    pub async fn store_for_project(&self, org_id: auth::OrgId, project_id: auth::ProjectId) -> Result<SharedStore, (StatusCode, String)> {
        self.org_stores.get_for_project(org_id, project_id).await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, e)
        })
    }
}

pub use org_store::SharedStore;

// --- Helpers ---

fn require_scope(ctx: &auth::AuthContext, scope: auth::Scope) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if ctx.has_scope(scope) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": format!("insufficient permissions: requires {:?}", scope) })),
        ))
    }
}

// --- Health handler ---

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub version: String,
    pub storage: StorageHealth,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

#[derive(Serialize)]
pub struct StorageHealth {
    pub trace_count: usize,
    pub span_count: usize,
    pub backend: String,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let store = match state.store_for_project(uuid::Uuid::nil(), uuid::Uuid::nil()).await {
        Ok(s) => s,
        Err(_) => {
            return Json(HealthResponse {
                status: "error".to_string(),
                uptime_secs: uptime,
                version: env!("CARGO_PKG_VERSION").to_string(),
                storage: StorageHealth { trace_count: 0, span_count: 0, backend: "unavailable".to_string() },
                region: None,
                instance: None,
            });
        }
    };
    let r = store.read().await;

    let region = std::env::var("FLY_REGION")
        .or_else(|_| std::env::var("RAILWAY_REGION"))
        .ok();
    let instance = std::env::var("FLY_ALLOC_ID")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok();

    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: uptime,
        version: env!("CARGO_PKG_VERSION").to_string(),
        storage: StorageHealth {
            trace_count: r.trace_count(),
            span_count: r.span_count(),
            backend: r.backend_type().to_string(),
        },
        region,
        instance,
    })
}

async fn ready() -> StatusCode {
    StatusCode::OK
}

async fn live() -> StatusCode {
    StatusCode::OK
}

async fn prometheus_metrics(State(state): State<AppState>) -> Response {
    let store = match state.store_for_project(uuid::Uuid::nil(), uuid::Uuid::nil()).await {
        Ok(s) => s,
        Err(_) => {
            return (
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                String::new(),
            )
                .into_response();
        }
    };
    let r = store.read().await;
    let m = metrics::Metrics::new();
    m.update_counts(r.span_count() as u64, r.trace_count() as u64);

    let body = m.export_prometheus();
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
        .into_response()
}

// --- Config / Shutdown handlers ---

async fn get_config(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::Admin)?;
    let config = state.config.read().await;
    Ok(Json(config.clone()))
}

async fn update_config(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Json(new_config): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !ctx.has_scope(auth::Scope::Admin) {
        return Err((StatusCode::FORBIDDEN, "insufficient permissions: requires Admin".to_string()));
    }
    let config_path = state.config_path.as_str();
    if config_path.is_empty() {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "config path not set".to_string()));
    }

    let toml_str = toml::to_string_pretty(&new_config)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid config: {}", e)))?;

    let path = std::path::Path::new(config_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to create config directory: {}", e)))?;
    }
    std::fs::write(path, &toml_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to write config: {}", e)))?;

    let mut config = state.config.write().await;
    *config = new_config.clone();

    tracing::info!("config updated and saved to {}", config_path);
    Ok(Json(new_config))
}

async fn post_shutdown(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::Admin) {
        return StatusCode::FORBIDDEN;
    }
    if let Some(ref tx) = state.shutdown_tx {
        tracing::info!("shutdown requested via API");
        let _ = tx.send(true);
        StatusCode::ACCEPTED
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

// --- Embedded UI ---

#[derive(Embed)]
#[folder = "../../ui/build"]
struct UiAssets;

async fn serve_ui(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if let Some(file) = UiAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            [(header::CONTENT_TYPE, mime.as_ref())],
            file.data,
        )
            .into_response();
    }

    // SPA fallback: serve index.html for any non-file path
    if let Some(index) = UiAssets::get("index.html") {
        return Html(index.data).into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}

async fn serve_scalar_docs() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html>
<head>
  <title>Traceway API</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <style>
    .coming-soon-banner {
      background: linear-gradient(135deg, #6366f1, #8b5cf6);
      color: white;
      text-align: center;
      padding: 12px 16px;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      font-size: 14px;
      font-weight: 500;
      letter-spacing: 0.02em;
      position: sticky;
      top: 0;
      z-index: 1000;
    }
  </style>
</head>
<body>
  <div class="coming-soon-banner">Coming Soon — Traceway is currently in development</div>
  <script id="api-reference" data-url="/api/openapi.json"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#)
}

// --- Router ---

pub fn router(store: SharedStore) -> Router {
    router_with_start_time(store, Instant::now(), serde_json::Value::Object(Default::default()), String::new(), None)
}

/// Builder for creating a router with cloud-aware configuration.
pub struct RouterBuilder {
    org_stores: Arc<OrgStoreManager>,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
    auth_config: auth::AuthConfig,
    api_key_lookup: Option<Arc<dyn auth::ApiKeyLookup>>,
}

impl RouterBuilder {
    /// Create a builder with a single shared store (local mode).
    pub fn new(store: SharedStore) -> Self {
        Self {
            org_stores: Arc::new(OrgStoreManager::single(store)),
            start_time: Instant::now(),
            config: serde_json::Value::Object(Default::default()),
            config_path: String::new(),
            shutdown_tx: None,
            auth_config: auth::AuthConfig::local(),
            api_key_lookup: None,
        }
    }

    /// Create a builder with a pre-configured OrgStoreManager (cloud mode).
    pub fn with_org_stores(org_stores: Arc<OrgStoreManager>) -> Self {
        Self {
            org_stores,
            start_time: Instant::now(),
            config: serde_json::Value::Object(Default::default()),
            config_path: String::new(),
            shutdown_tx: None,
            auth_config: auth::AuthConfig::local(),
            api_key_lookup: None,
        }
    }

    pub fn org_stores(mut self, m: Arc<OrgStoreManager>) -> Self { self.org_stores = m; self }
    pub fn start_time(mut self, t: Instant) -> Self { self.start_time = t; self }
    pub fn config(mut self, c: serde_json::Value) -> Self { self.config = c; self }
    pub fn config_path(mut self, p: String) -> Self { self.config_path = p; self }
    pub fn shutdown_tx(mut self, tx: watch::Sender<bool>) -> Self { self.shutdown_tx = Some(tx); self }
    pub fn auth_config(mut self, c: auth::AuthConfig) -> Self { self.auth_config = c; self }
    pub fn api_key_lookup(mut self, l: Arc<dyn auth::ApiKeyLookup>) -> Self { self.api_key_lookup = Some(l); self }

    pub fn build(self) -> Router {
        build_router(
            self.org_stores,
            self.start_time,
            self.config,
            self.config_path,
            self.shutdown_tx,
            self.auth_config,
            self.api_key_lookup,
        )
    }
}

pub fn router_with_start_time(
    store: SharedStore,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
) -> Router {
    let org_stores = Arc::new(OrgStoreManager::single(store));
    build_router(org_stores, start_time, config, config_path, shutdown_tx, auth::AuthConfig::local(), None)
}

fn build_router(
    org_stores: Arc<OrgStoreManager>,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
    auth_config: auth::AuthConfig,
    api_key_lookup: Option<Arc<dyn auth::ApiKeyLookup>>,
) -> Router {
    let (events_tx, _) = broadcast::channel(256);

    // Create durable event log. In local mode, use SQLite alongside the config.
    // In cloud mode, fall back to NoopEventLog (events are ephemeral via Redis Pub/Sub).
    let event_log: Arc<dyn events::EventLog> = if auth_config.local_mode {
        let log_path = if config_path.is_empty() {
            std::path::PathBuf::from("data/event_log.db")
        } else {
            let p = std::path::Path::new(&config_path);
            p.parent().unwrap_or(std::path::Path::new("data")).join("event_log.db")
        };
        match event_log::SqliteEventLog::open(&log_path) {
            Ok(log) => {
                tracing::info!(path = %log_path.display(), "opened SQLite event log");
                let log = Arc::new(log);
                event_log::spawn_event_log_trimmer(log.clone(), std::time::Duration::from_secs(86400));
                log
            }
            Err(e) => {
                tracing::warn!("failed to open event log at {}: {e}, using noop", log_path.display());
                Arc::new(events::NoopEventLog)
            }
        }
    } else {
        Arc::new(events::NoopEventLog)
    };

    let api_key_lookup: Arc<dyn auth::ApiKeyLookup> = api_key_lookup.unwrap_or_else(|| {
        Arc::new(auth_keys::NoopApiKeyLookup) as Arc<dyn auth::ApiKeyLookup>
    });

    let state = AppState {
        org_stores,
        events_tx,
        event_log,
        start_time,
        config: Arc::new(RwLock::new(config)),
        config_path: Arc::new(config_path),
        shutdown_tx,
        auth_config: auth_config.clone(),
        api_key_lookup,
    };

    // In cloud mode with a separate frontend origin, we need explicit origins
    // and credentials support. ALLOWED_ORIGINS env var is comma-separated.
    // In local mode (no env var), allow any origin without credentials.
    let cors = if let Ok(origins) = std::env::var("ALLOWED_ORIGINS") {
        let origins: Vec<axum::http::HeaderValue> = origins
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                match trimmed.parse::<axum::http::HeaderValue>() {
                    Ok(v) => {
                        tracing::info!(origin = trimmed, "CORS: allowing origin");
                        Some(v)
                    }
                    Err(e) => {
                        tracing::warn!(origin = trimmed, error = %e, "CORS: failed to parse origin, skipping");
                        None
                    }
                }
            })
            .collect();
        if origins.is_empty() {
            tracing::warn!("CORS: ALLOWED_ORIGINS set but no valid origins parsed, falling back to permissive");
        }
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
                axum::http::Method::PATCH,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                header::ORIGIN,
                header::COOKIE,
            ])
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Rust API is now ingest/infra-only. Public product APIs moved to Encore.
    let public = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/live", get(live))
        .route("/metrics", get(prometheus_metrics))
        .route("/config", get(get_config).put(update_config))
        .route("/shutdown", post(post_shutdown));

    let api = Router::new().merge(public);

    // OTLP ingest routes — outside /api, with self-contained auth.
    let otlp = Router::new()
        .route("/v1/traces", post(otlp::ingest_traces));

    let app = Router::new()
        .nest("/api", api)
        .merge(otlp);

    // In cloud mode, serve Scalar API docs at root; in local mode, serve embedded UI
    let app = if state.auth_config.local_mode {
        app.fallback(serve_ui)
    } else {
        app.route("/", get(serve_scalar_docs))
            .fallback(|| async { StatusCode::NOT_FOUND })
    };

    app.layer(cors)
        .with_state(state)
}

// --- Server ---

pub async fn serve(store: SharedStore, addr: &str) -> std::io::Result<()> {
    let org_stores = Arc::new(OrgStoreManager::single(store));
    serve_with_shutdown(org_stores, addr, Instant::now(), serde_json::Value::Object(Default::default()), String::new(), None, std::future::pending()).await
}

pub async fn serve_with_shutdown(
    org_stores: Arc<OrgStoreManager>,
    addr: &str,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let app = build_router(org_stores, start_time, config, config_path, shutdown_tx, auth::AuthConfig::local(), None);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("api listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
