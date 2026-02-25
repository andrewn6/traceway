pub mod any_backend;
pub mod auth_keys;
pub mod auth_routes;
pub mod billing_routes;
pub mod capture;
pub mod events;
pub mod jobs;
pub mod metrics;
pub mod org_store;

pub use org_store::OrgStoreManager;

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, Request, StatusCode, Uri},
    response::{
        sse::{Event, KeepAlive},
        Html, IntoResponse, Response, Sse,
    },
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::Multipart;
use rust_embed::Embed;
use chrono::{Datelike, DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use tokio::sync::{broadcast, watch, RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use storage::{analytics, encode_cursor, decode_cursor, CursorInner, FileFilter, Page, SpanFilter};

pub use any_backend::AnyBackend;
use trace::{
    AnalyticsQuery, AnalyticsResponse, AnalyticsSummary, CaptureFilters, CaptureRule,
    CaptureRuleId, Datapoint, DatapointId, DatapointKind, DatapointSource, Dataset, DatasetId,
    EvalConfig, EvalResult, EvalRun, EvalRunId, EvalRunStatus, FileVersion,
    Message, ProviderConnection, ProviderConnectionId, ProviderConnectionInfo,
    QueueItem, QueueItemId, QueueItemStatus, ScoreSummary, ScoringStrategy, Span,
    SpanBuilder, SpanId, SpanKind, Trace, TraceId,
};

pub use events::{EventBus, EventSubscriber, LocalEventBus};

// --- OpenAPI ---

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Traceway API",
        version = "0.1.0",
        description = "LLM tracing and observability API"
    ),
    paths(
        openapi_spec,
        // Traces
        list_traces,
        create_trace,
        get_trace,
        delete_trace,
        clear_all_traces,
        // Spans
        list_spans,
        get_span,
        create_span,
        complete_span,
        fail_span,
        delete_span,
        // Files
        list_files,
        get_file_versions,
        get_file_content,
        // Datasets
        list_datasets,
        create_dataset,
        get_dataset,
        update_dataset,
        delete_dataset_handler,
        // Datapoints
        list_datapoints,
        create_datapoint,
        delete_datapoint_handler,
        export_span_to_dataset,
        import_file,
        // Queue
        list_queue,
        enqueue_datapoints,
        claim_queue_item,
        submit_queue_item,
        // Analytics
        post_analytics,
        analytics_summary,
        // Stats & Export
        get_stats,
        export_json,
        // Events
        events,
        // Health & Observability
        health,
        prometheus_metrics,
    ),
    components(schemas(
        // Trace types
        trace::Span,
        trace::SpanKind,
        trace::SpanStatus,
        trace::Trace,
        trace::FileVersion,
        trace::TrackedFile,
        trace::Dataset,
        trace::Datapoint,
        trace::DatapointKind,
        trace::DatapointSource,
        trace::Message,
        trace::QueueItem,
        trace::QueueItemStatus,
        trace::AnalyticsQuery,
        trace::AnalyticsMetric,
        trace::GroupByField,
        trace::AnalyticsFilter,
        trace::AnalyticsResponse,
        trace::AnalyticsGroup,
        trace::MetricValues,
        trace::AnalyticsSummary,
        trace::ModelCost,
        trace::ModelTokens,
        // Request types
        CreateSpanRequest,
        CompleteSpanRequest,
        FailSpanRequest,
        SpanQueryParams,
        CreateTraceRequest,
        FileQueryParams,
        ExportParams,
        CreateDatasetRequest,
        UpdateDatasetRequest,
        CreateDatapointRequest,
        ExportSpanRequest,
        EnqueueRequest,
        ClaimRequest,
        SubmitRequest,
        PaginationParams,
        // Response types
        CreatedSpan,
        SpanList,
        Stats,
        DeletedTrace,
        ClearedAll,
        ExportData,
        FileListResponse,
        FileVersionsResponse,
        DatasetResponse,
        DatasetListResponse,
        ImportResponse,
        EnqueueResponse,
        HealthResponse,
        StorageHealth,
    ))
)]
pub struct ApiDoc;

/// Get OpenAPI specification
#[utoipa::path(
    get,
    path = "/api/openapi.json",
    responses(
        (status = 200, description = "OpenAPI JSON specification")
    ),
    tag = "docs"
)]
async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

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
    pub start_time: Instant,
    pub config: Arc<RwLock<serde_json::Value>>,
    pub config_path: Arc<String>,
    pub shutdown_tx: Option<watch::Sender<bool>>,
    pub auth_config: auth::AuthConfig,
    pub auth_store: Option<Arc<dyn auth::AuthStore>>,
    pub api_key_lookup: Arc<dyn auth::ApiKeyLookup>,
    pub email_sender: Arc<dyn auth::EmailSender>,
    /// Base URL for links in emails (e.g. "https://platform.traceway.ai")
    pub app_url: String,
    /// Polar.sh webhook secret for signature verification (Standard Webhooks format)
    pub polar_webhook_secret: Option<String>,
    /// Polar.sh access token for creating checkout sessions
    pub polar_access_token: Option<String>,
}

impl AppState {
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

// --- Request types ---

#[derive(Deserialize, ToSchema)]
pub struct CreateSpanRequest {
    #[schema(value_type = String)]
    pub trace_id: TraceId,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub parent_id: Option<SpanId>,
    pub name: String,
    pub kind: SpanKind,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
}

#[derive(Deserialize, ToSchema)]
pub struct CompleteSpanRequest {
    #[serde(default)]
    pub output: Option<serde_json::Value>,
}

#[derive(Deserialize, ToSchema)]
pub struct FailSpanRequest {
    pub error: String,
}

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 1000;

fn paginate<T>(items: Vec<T>, params: &PaginationParams) -> Result<Page<T>, StatusCode> {
    let limit = params.limit.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE);
    let offset = match params.cursor {
        Some(ref c) => {
            let inner = decode_cursor(c).map_err(|_| StatusCode::BAD_REQUEST)?;
            inner.last_value.parse::<usize>().map_err(|_| StatusCode::BAD_REQUEST)?
        }
        None => 0,
    };

    let total = items.len();
    let has_more = offset + limit < total;
    let page_items: Vec<T> = items.into_iter().skip(offset).take(limit).collect();

    let next_cursor = if has_more {
        Some(encode_cursor(&CursorInner {
            sort_field: "offset".into(),
            last_value: (offset + limit).to_string(),
            last_id: String::new(),
        }))
    } else {
        None
    };

    Ok(Page {
        items: page_items,
        total: Some(total),
        next_cursor,
        has_more,
    })
}

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct SpanQueryParams {
    pub kind: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub status: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub name_contains: Option<String>,
    pub path: Option<String>,
    #[schema(value_type = Option<String>)]
    #[param(value_type = Option<String>)]
    pub trace_id: Option<TraceId>,
    /// Minimum duration in ms (e.g. 500)
    pub duration_min: Option<i64>,
    /// Maximum duration in ms (e.g. 5000)
    pub duration_max: Option<i64>,
    /// Minimum total tokens (e.g. 1000)
    pub tokens_min: Option<u64>,
    /// Minimum cost in dollars (e.g. 0.01)
    pub cost_min: Option<f64>,
    /// Sort field: "started_at", "duration", "tokens", "cost", "name"
    pub sort_by: Option<String>,
    /// Sort order: "asc" or "desc" (default: "desc")
    pub sort_order: Option<String>,
    /// Max items per page (default 50, max 1000)
    pub limit: Option<usize>,
    /// Opaque cursor from a previous response
    pub cursor: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateTraceRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct FileQueryParams {
    pub path_prefix: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ExportParams {
    #[schema(value_type = Option<String>)]
    #[param(value_type = Option<String>)]
    pub trace_id: Option<TraceId>,
}

// --- Dataset request types ---

#[derive(Deserialize, ToSchema)]
pub struct CreateDatasetRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateDatasetRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateDatapointRequest {
    pub kind: DatapointKind,
}

#[derive(Deserialize, ToSchema)]
pub struct ExportSpanRequest {
    #[schema(value_type = String)]
    pub span_id: SpanId,
}

#[derive(Deserialize, ToSchema)]
pub struct EnqueueRequest {
    #[schema(value_type = Vec<String>)]
    pub datapoint_ids: Vec<DatapointId>,
}

#[derive(Deserialize, ToSchema)]
pub struct ClaimRequest {
    pub claimed_by: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SubmitRequest {
    #[serde(default)]
    pub edited_data: Option<serde_json::Value>,
}

// --- Eval request types ---

#[derive(Deserialize, ToSchema)]
pub struct CreateEvalRunRequest {
    #[serde(default)]
    pub name: Option<String>,
    pub config: EvalConfig,
    #[serde(default = "default_scoring")]
    pub scoring: ScoringStrategy,
}

fn default_scoring() -> ScoringStrategy {
    ScoringStrategy::None
}

// --- Capture Rule request types ---

#[derive(Deserialize, ToSchema)]
pub struct CreateCaptureRuleRequest {
    pub name: String,
    pub filters: CaptureFilters,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

fn default_sample_rate() -> f64 {
    1.0
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCaptureRuleRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub filters: Option<CaptureFilters>,
    #[serde(default)]
    pub sample_rate: Option<f64>,
}

// --- Comparison query params ---

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct CompareParams {
    pub runs: String,
}

// --- Response types ---

#[derive(Serialize, ToSchema)]
pub struct CreatedSpan {
    #[schema(value_type = String)]
    pub id: SpanId,
    #[schema(value_type = String)]
    pub trace_id: TraceId,
}

#[derive(Serialize, ToSchema)]
pub struct SpanList {
    pub spans: Vec<Span>,
    pub count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct Stats {
    pub trace_count: usize,
    pub span_count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct DeletedTrace {
    #[schema(value_type = String)]
    pub trace_id: TraceId,
    pub spans_deleted: usize,
}

#[derive(Serialize, ToSchema)]
pub struct ClearedAll {
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct ExportData {
    #[schema(value_type = HashMap<String, Vec<Span>>)]
    pub traces: HashMap<TraceId, Vec<Span>>,
}

#[derive(Serialize, ToSchema)]
pub struct FileListResponse {
    pub files: Vec<FileVersion>,
    pub count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct FileVersionsResponse {
    pub path: String,
    pub versions: Vec<FileVersion>,
    pub count: usize,
}

// --- Dataset response types ---

#[derive(Serialize, ToSchema)]
pub struct DatasetResponse {
    #[serde(flatten)]
    pub dataset: Dataset,
    pub datapoint_count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct DatasetListResponse {
    pub datasets: Vec<DatasetResponse>,
    pub count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct ImportResponse {
    pub imported: usize,
    #[schema(value_type = String)]
    pub dataset_id: DatasetId,
}

#[derive(Serialize, ToSchema)]
pub struct EnqueueResponse {
    pub enqueued: usize,
}

// --- Eval response types ---

#[derive(Serialize, ToSchema)]
pub struct EvalRunDetailResponse {
    #[serde(flatten)]
    pub run: EvalRun,
    pub result_items: Vec<EvalResult>,
}

#[derive(Serialize, ToSchema)]
pub struct ComparisonDatapoint {
    #[schema(value_type = String)]
    pub datapoint_id: DatapointId,
    pub input: serde_json::Value,
    pub expected: Option<serde_json::Value>,
    pub results: std::collections::HashMap<String, ComparisonCell>,
}

#[derive(Serialize, ToSchema)]
pub struct ComparisonCell {
    pub output: serde_json::Value,
    pub score: Option<f64>,
    pub latency_ms: u64,
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct ComparisonResponse {
    pub runs: Vec<EvalRun>,
    pub datapoints: Vec<ComparisonDatapoint>,
}

// --- Provider Connection request/response types ---

#[derive(Deserialize, ToSchema)]
pub struct CreateProviderConnectionRequest {
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProviderConnectionRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ProviderConnectionListResponse {
    pub connections: Vec<ProviderConnectionInfo>,
    pub count: usize,
}

#[derive(Deserialize, ToSchema)]
pub struct TestProviderConnectionRequest {
    pub provider: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ProviderModelInfo {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ProviderModelsResponse {
    pub models: Vec<ProviderModelInfo>,
    pub ok: bool,
    pub error: Option<String>,
}

// --- Scope enforcement helper ---

/// Check that the auth context has the required scope, returning 403 if not.
fn require_scope(ctx: &auth::AuthContext, scope: auth::Scope) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if ctx.has_scope(scope) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": format!("insufficient permissions: requires {:?}", scope),
                "code": "insufficient_scope"
            })),
        ))
    }
}

/// Convert store_for_org error to JSON error response.
fn store_err_json(e: (StatusCode, String)) -> (StatusCode, Json<serde_json::Value>) {
    (e.0, Json(serde_json::json!({ "error": e.1 })))
}

/// Convert store_for_org error to plain StatusCode.
fn store_err_status(e: (StatusCode, String)) -> StatusCode {
    e.0
}

// --- Trace handlers ---

/// List all traces (paginated)
#[utoipa::path(
    get,
    path = "/api/traces",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of traces"),
        (status = 403, description = "Insufficient permissions"),
    ),
    security(("bearer" = [])),
    tag = "traces"
)]
async fn list_traces(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Page<Trace>>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    let traces: Vec<Trace> = r.all_traces().cloned().collect();
    let page = paginate(traces, &params).map_err(|s| (s, Json(serde_json::json!({"error": "invalid pagination"}))))?;
    Ok(Json(page))
}

/// Create a new trace
#[utoipa::path(
    post,
    path = "/api/traces",
    request_body = CreateTraceRequest,
    responses(
        (status = 201, description = "Trace created", body = Trace),
        (status = 403, description = "Insufficient permissions"),
    ),
    security(("bearer" = [])),
    tag = "traces"
)]
async fn create_trace(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Json(req): Json<CreateTraceRequest>,
) -> Result<(StatusCode, Json<Trace>), (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesWrite)?;
    let trace = Trace::new(req.name).with_tags(req.tags);
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let mut w = store.write().await;
    w.save_trace(trace.clone()).await;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::TraceCreated { trace: trace.clone() });
    Ok((StatusCode::CREATED, Json(trace)))
}

/// Get all spans for a trace
#[utoipa::path(
    get,
    path = "/api/traces/{trace_id}",
    params(("trace_id" = String, Path, description = "Trace ID")),
    responses(
        (status = 200, description = "Spans in the trace", body = SpanList),
        (status = 404, description = "Trace not found"),
    ),
    security(("bearer" = [])),
    tag = "traces"
)]
async fn get_trace(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(trace_id): Path<TraceId>,
) -> Result<Json<SpanList>, StatusCode> {
    require_scope(&ctx, auth::Scope::TracesRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let span_ids = r.spans_for_trace(trace_id);
    if span_ids.is_empty() {
        // Check if trace exists in metadata
        if r.get_trace(trace_id).is_none() {
            return Err(StatusCode::NOT_FOUND);
        }
    }
    let spans: Vec<Span> = span_ids
        .iter()
        .filter_map(|id| r.get(*id).cloned())
        .collect();
    let count = spans.len();
    Ok(Json(SpanList { spans, count }))
}

// --- Span handlers ---

/// List spans with optional filters (paginated)
#[utoipa::path(
    get,
    path = "/api/spans",
    params(SpanQueryParams),
    responses(
        (status = 200, description = "Paginated filtered list of spans"),
        (status = 403, description = "Insufficient permissions"),
    ),
    security(("bearer" = [])),
    tag = "spans"
)]
async fn list_spans(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Query(params): Query<SpanQueryParams>,
) -> Result<Json<Page<Span>>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    let filter = SpanFilter {
        kind: params.kind,
        model: params.model,
        provider: params.provider,
        status: params.status,
        since: params.since,
        until: params.until,
        name_contains: params.name_contains,
        path: params.path,
        trace_id: params.trace_id,
        limit: None,
        duration_min: params.duration_min,
        duration_max: params.duration_max,
        tokens_min: params.tokens_min,
        cost_min: params.cost_min,
        sort_by: params.sort_by,
        sort_order: params.sort_order,
    };
    let spans: Vec<Span> = r.filter_spans(&filter).into_iter().cloned().collect();
    let page_params = PaginationParams { limit: params.limit, cursor: params.cursor };
    let page = paginate(spans, &page_params).map_err(|s| (s, Json(serde_json::json!({"error": "invalid pagination"}))))?;
    Ok(Json(page))
}

/// Get a single span by ID
#[utoipa::path(
    get,
    path = "/api/spans/{span_id}",
    params(("span_id" = String, Path, description = "Span ID")),
    responses(
        (status = 200, description = "Span details", body = Span),
        (status = 404, description = "Span not found"),
    ),
    security(("bearer" = [])),
    tag = "spans"
)]
async fn get_span(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
) -> Result<Json<Span>, StatusCode> {
    require_scope(&ctx, auth::Scope::TracesRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    w.get_or_load(span_id).await
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new span
#[utoipa::path(
    post,
    path = "/api/spans",
    request_body = CreateSpanRequest,
    responses(
        (status = 201, description = "Span created", body = CreatedSpan),
        (status = 403, description = "Insufficient permissions"),
    ),
    security(("bearer" = [])),
    tag = "spans"
)]
async fn create_span(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Json(req): Json<CreateSpanRequest>,
) -> Result<(StatusCode, Json<CreatedSpan>), (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesWrite)?;

    // Enforce span limit per plan (cloud mode only)
    if !ctx.is_local_mode {
        if let Some(auth_store) = state.auth_store.as_ref() {
            if let Ok(Some(org)) = auth_store.get_org(ctx.org_id).await {
                let limit = org.plan.spans_per_month();
                let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
                let r = store.read().await;
                // Count spans created since the start of the current calendar month
                let now = Utc::now();
                let month_start = now.date_naive()
                    .with_day(1)
                    .unwrap_or(now.date_naive());
                let month_start_dt = month_start.and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let monthly_filter = SpanFilter {
                    since: Some(month_start_dt),
                    ..Default::default()
                };
                let current_count = r.filter_spans(&monthly_filter).len() as u64;
                drop(r);
                if current_count >= limit {
                    return Err((StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({
                        "error": format!("Monthly span limit reached ({limit}). Upgrade your plan at /settings/billing.")
                    }))));
                }
            }
        }
    }

    // Estimate cost from model pricing if not already provided
    let kind = req.kind.with_estimated_cost();
    let mut builder = SpanBuilder::new(req.trace_id, req.name, kind);
    if let Some(parent_id) = req.parent_id {
        builder = builder.parent(parent_id);
    }
    if let Some(input) = req.input {
        builder = builder.input(input);
    }
    let span = builder.build();
    let id = span.id();
    let trace_id = span.trace_id();

    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let mut w = store.write().await;
    w.insert(span.clone()).await;
    drop(w);

    let _ = state.events_tx.send(SystemEvent::SpanCreated { span });
    tracing::debug!(%id, %trace_id, "span created");
    Ok((StatusCode::CREATED, Json(CreatedSpan { id, trace_id })))
}

/// Mark a span as completed
#[utoipa::path(
    post,
    path = "/api/spans/{span_id}/complete",
    params(("span_id" = String, Path, description = "Span ID")),
    request_body(content = CompleteSpanRequest, description = "Optional output data"),
    responses(
        (status = 200, description = "Span completed"),
        (status = 404, description = "Span not found"),
        (status = 409, description = "Span already in terminal state"),
    ),
    security(("bearer" = [])),
    tag = "spans"
)]
async fn complete_span(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
    body: Option<Json<CompleteSpanRequest>>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::TracesWrite) {
        return StatusCode::FORBIDDEN;
    }
    let output = body.and_then(|b| b.0.output);

    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;

    // Check if already terminal → 409 Conflict
    // Use get_or_load to fall back to storage backend when running multiple instances
    if let Some(span) = w.get_or_load(span_id).await {
        if span.status().is_terminal() {
            return StatusCode::CONFLICT;
        }
    } else {
        return StatusCode::NOT_FOUND;
    }

    if let Some(span) = w.complete_span(span_id, output).await {
        drop(w);
        let _ = state.events_tx.send(SystemEvent::SpanCompleted { span: span.clone() });
        tracing::debug!(%span_id, "span completed");

        // Process auto-capture rules in the background
        let store_clone = store.clone();
        let events_tx = state.events_tx.clone();
        tokio::spawn(async move {
            capture::process_capture_rules(&store_clone, &span, &events_tx).await;
        });

        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Mark a span as failed
#[utoipa::path(
    post,
    path = "/api/spans/{span_id}/fail",
    params(("span_id" = String, Path, description = "Span ID")),
    request_body = FailSpanRequest,
    responses(
        (status = 200, description = "Span marked as failed"),
        (status = 404, description = "Span not found"),
        (status = 409, description = "Span already in terminal state"),
    ),
    security(("bearer" = [])),
    tag = "spans"
)]
async fn fail_span(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
    Json(req): Json<FailSpanRequest>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::TracesWrite) {
        return StatusCode::FORBIDDEN;
    }
    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;

    // Check if already terminal → 409 Conflict
    // Use get_or_load to fall back to storage backend when running multiple instances
    if let Some(span) = w.get_or_load(span_id).await {
        if span.status().is_terminal() {
            return StatusCode::CONFLICT;
        }
    } else {
        return StatusCode::NOT_FOUND;
    }

    if let Some(span) = w.fail_span(span_id, req.error).await {
        drop(w);
        let _ = state.events_tx.send(SystemEvent::SpanFailed { span });
        tracing::debug!(%span_id, "span failed");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Get trace and span counts
#[utoipa::path(
    get,
    path = "/api/stats",
    responses(
        (status = 200, description = "Current stats", body = Stats),
    ),
    security(("bearer" = [])),
    tag = "stats"
)]
async fn get_stats(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> Result<Json<Stats>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    Ok(Json(Stats {
        trace_count: r.trace_count(),
        span_count: r.span_count(),
    }))
}

// --- File handlers ---

/// List tracked files
#[utoipa::path(
    get,
    path = "/api/files",
    params(FileQueryParams),
    responses(
        (status = 200, description = "List of file versions", body = FileListResponse),
    ),
    security(("bearer" = [])),
    tag = "files"
)]
async fn list_files(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Query(params): Query<FileQueryParams>,
) -> Result<Json<FileListResponse>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    let filter = FileFilter {
        path_prefix: params.path_prefix,
        since: params.since,
        until: params.until,
        ..Default::default()
    };
    let files: Vec<FileVersion> = r.list_files(&filter).into_iter().cloned().collect();
    let count = files.len();
    Ok(Json(FileListResponse { files, count }))
}

/// Get all versions of a file by path
#[utoipa::path(
    get,
    path = "/api/files/{path}",
    params(("path" = String, Path, description = "File path")),
    responses(
        (status = 200, description = "File versions", body = FileVersionsResponse),
        (status = 404, description = "File not found"),
    ),
    security(("bearer" = [])),
    tag = "files"
)]
async fn get_file_versions(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<FileVersionsResponse>, StatusCode> {
    require_scope(&ctx, auth::Scope::TracesRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let versions: Vec<FileVersion> = r.get_file_versions(&path).into_iter().cloned().collect();
    if versions.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    let count = versions.len();
    Ok(Json(FileVersionsResponse {
        path,
        versions,
        count,
    }))
}

// --- File content handler ---

/// Get file content by hash
#[utoipa::path(
    get,
    path = "/api/files/content/{hash}",
    params(("hash" = String, Path, description = "Content hash")),
    responses(
        (status = 200, description = "File content bytes"),
        (status = 404, description = "File not found"),
    ),
    security(("bearer" = [])),
    tag = "files"
)]
async fn get_file_content(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Response, StatusCode> {
    require_scope(&ctx, auth::Scope::TracesRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let content = r.load_file_content(&hash).await.map_err(|_| StatusCode::NOT_FOUND)?;
    drop(r);

    // Try to guess mime type from the hash's associated file path
    let mime = {
        let r2 = store.read().await;
        let filter = FileFilter::default();
        r2.list_files(&filter)
            .into_iter()
            .find(|f| f.hash == hash)
            .map(|f| mime_guess::from_path(&f.path).first_or_octet_stream())
            .unwrap_or_else(|| mime_guess::mime::APPLICATION_OCTET_STREAM)
    };

    Ok((
        [
            (header::CONTENT_TYPE, mime.as_ref().to_string()),
            (header::CONTENT_LENGTH, content.len().to_string()),
        ],
        content,
    )
        .into_response())
}

// --- Export handler ---

/// Export traces as JSON
#[utoipa::path(
    get,
    path = "/api/export/json",
    params(ExportParams),
    responses(
        (status = 200, description = "Exported trace data", body = ExportData),
    ),
    security(("bearer" = [])),
    tag = "export"
)]
async fn export_json(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Query(params): Query<ExportParams>,
) -> Result<Json<ExportData>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    let mut traces: HashMap<TraceId, Vec<Span>> = HashMap::new();

    if let Some(trace_id) = params.trace_id {
        let span_ids = r.spans_for_trace(trace_id);
        let spans: Vec<Span> = span_ids
            .iter()
            .filter_map(|id| r.get(*id).cloned())
            .collect();
        if !spans.is_empty() {
            traces.insert(trace_id, spans);
        }
    } else {
        for &trace_id in r.span_trace_ids() {
            let span_ids = r.spans_for_trace(trace_id);
            let spans: Vec<Span> = span_ids
                .iter()
                .filter_map(|id| r.get(*id).cloned())
                .collect();
            traces.insert(trace_id, spans);
        }
    }

    Ok(Json(ExportData { traces }))
}

// --- SSE handler ---

/// Server-sent events stream for real-time updates
#[utoipa::path(
    get,
    path = "/api/events",
    responses(
        (status = 200, description = "SSE event stream"),
    ),
    security(("bearer" = [])),
    tag = "events"
)]
async fn events(
    auth::Auth(_ctx): auth::Auth,
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let json = serde_json::to_string(&event).ok()?;
            Some(Ok(Event::default().data(json)))
        }
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

// --- Delete handlers ---

/// Delete a span
#[utoipa::path(
    delete,
    path = "/api/spans/{span_id}",
    params(("span_id" = String, Path, description = "Span ID")),
    responses(
        (status = 200, description = "Span deleted"),
        (status = 404, description = "Span not found"),
    ),
    security(("bearer" = [])),
    tag = "spans"
)]
async fn delete_span(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::TracesWrite) {
        return StatusCode::FORBIDDEN;
    }
    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;
    if w.delete_span(span_id).await {
        drop(w);
        let _ = state.events_tx.send(SystemEvent::SpanDeleted { span_id });
        tracing::debug!(%span_id, "span deleted");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Delete a trace and all its spans
#[utoipa::path(
    delete,
    path = "/api/traces/{trace_id}",
    params(("trace_id" = String, Path, description = "Trace ID")),
    responses(
        (status = 200, description = "Trace deleted", body = DeletedTrace),
        (status = 404, description = "Trace not found"),
    ),
    security(("bearer" = [])),
    tag = "traces"
)]
async fn delete_trace(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(trace_id): Path<TraceId>,
) -> Result<Json<DeletedTrace>, StatusCode> {
    require_scope(&ctx, auth::Scope::TracesWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    let spans_deleted = w.delete_trace(trace_id).await;
    drop(w);
    if spans_deleted > 0 {
        let _ = state
            .events_tx
            .send(SystemEvent::TraceDeleted { trace_id });
        tracing::debug!(%trace_id, %spans_deleted, "trace deleted");
        Ok(Json(DeletedTrace {
            trace_id,
            spans_deleted,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Clear all traces and spans
#[utoipa::path(
    delete,
    path = "/api/traces",
    responses(
        (status = 200, description = "All traces cleared", body = ClearedAll),
    ),
    security(("bearer" = [])),
    tag = "traces"
)]
async fn clear_all_traces(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> Result<Json<ClearedAll>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::TracesWrite)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let mut w = store.write().await;
    w.clear().await;
    drop(w);
    let _ = state.events_tx.send(SystemEvent::Cleared);
    tracing::debug!("all traces cleared");
    Ok(Json(ClearedAll {
        message: "All traces cleared".to_string(),
    }))
}

// --- Health handler ---

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct StorageHealth {
    pub trace_count: usize,
    pub span_count: usize,
    pub backend: String,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service health status", body = HealthResponse),
    ),
    tag = "health"
)]
async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    // Health check uses nil org_id (local mode store or any available store)
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

    // Get region/instance from env for cloud deployments
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

// --- Readiness probe (for k8s/cloud platforms) ---

async fn ready() -> StatusCode {
    // Simple readiness check - could add DB connectivity check here
    StatusCode::OK
}

// --- Liveness probe ---

async fn live() -> StatusCode {
    StatusCode::OK
}

// --- Metrics endpoint (Prometheus format) ---

/// Prometheus metrics endpoint
#[utoipa::path(
    get,
    path = "/api/metrics",
    responses(
        (status = 200, description = "Prometheus-format metrics"),
    ),
    tag = "health"
)]
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
    let metrics = metrics::Metrics::new();
    metrics.update_counts(r.span_count() as u64, r.trace_count() as u64);

    let body = metrics.export_prometheus();
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
        .into_response()
}

// --- Dataset handlers ---

/// List all datasets
#[utoipa::path(
    get,
    path = "/api/datasets",
    responses(
        (status = 200, description = "List of datasets", body = DatasetListResponse),
    ),
    security(("bearer" = [])),
    tag = "datasets"
)]
async fn list_datasets(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> Result<Json<DatasetListResponse>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::DatasetsRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    let datasets: Vec<DatasetResponse> = r
        .all_datasets()
        .map(|ds| DatasetResponse {
            datapoint_count: r.datapoint_count_for_dataset(ds.id),
            dataset: ds.clone(),
        })
        .collect();
    let count = datasets.len();
    Ok(Json(DatasetListResponse { datasets, count }))
}

/// Create a new dataset
#[utoipa::path(
    post,
    path = "/api/datasets",
    request_body = CreateDatasetRequest,
    responses(
        (status = 201, description = "Dataset created", body = Dataset),
    ),
    security(("bearer" = [])),
    tag = "datasets"
)]
async fn create_dataset(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Json(req): Json<CreateDatasetRequest>,
) -> Result<(StatusCode, Json<Dataset>), (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::DatasetsWrite)?;
    let dataset = Dataset::new(req.name, req.description);
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let mut w = store.write().await;
    w.save_dataset(dataset.clone()).await;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::DatasetCreated { dataset: dataset.clone() });
    Ok((StatusCode::CREATED, Json(dataset)))
}

/// Get a dataset by ID
#[utoipa::path(
    get,
    path = "/api/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    responses(
        (status = 200, description = "Dataset details", body = DatasetResponse),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "datasets"
)]
async fn get_dataset(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
) -> Result<Json<DatasetResponse>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let ds = r.get_dataset(dataset_id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(DatasetResponse {
        datapoint_count: r.datapoint_count_for_dataset(ds.id),
        dataset: ds.clone(),
    }))
}

/// Update a dataset
#[utoipa::path(
    put,
    path = "/api/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    request_body = UpdateDatasetRequest,
    responses(
        (status = 200, description = "Dataset updated", body = DatasetResponse),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "datasets"
)]
async fn update_dataset(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<UpdateDatasetRequest>,
) -> Result<Json<DatasetResponse>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    let ds = w.get_dataset(dataset_id).ok_or(StatusCode::NOT_FOUND)?.clone();
    let mut updated = ds;
    if let Some(name) = req.name {
        updated.name = name;
    }
    if let Some(desc) = req.description {
        updated.description = Some(desc);
    }
    updated.updated_at = chrono::Utc::now();
    w.save_dataset(updated.clone()).await;
    let count = w.datapoint_count_for_dataset(dataset_id);
    Ok(Json(DatasetResponse {
        dataset: updated,
        datapoint_count: count,
    }))
}

/// Delete a dataset
#[utoipa::path(
    delete,
    path = "/api/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    responses(
        (status = 200, description = "Dataset deleted"),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "datasets"
)]
async fn delete_dataset_handler(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::DatasetsWrite) {
        return StatusCode::FORBIDDEN;
    }
    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;
    if w.delete_dataset(dataset_id).await {
        drop(w);
        let _ = state
            .events_tx
            .send(SystemEvent::DatasetDeleted { dataset_id });
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

// --- Datapoint handlers ---

/// List datapoints in a dataset (paginated)
#[utoipa::path(
    get,
    path = "/api/datasets/{id}/datapoints",
    params(("id" = String, Path, description = "Dataset ID"), PaginationParams),
    responses(
        (status = 200, description = "Paginated list of datapoints"),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "datapoints"
)]
async fn list_datapoints(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Page<Datapoint>>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    if r.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let datapoints: Vec<Datapoint> = r
        .datapoints_for_dataset(dataset_id)
        .into_iter()
        .cloned()
        .collect();
    let page = paginate(datapoints, &params)?;
    Ok(Json(page))
}

/// Get a single datapoint
#[utoipa::path(
    get,
    path = "/api/datasets/{id}/datapoints/{dp_id}",
    params(
        ("id" = String, Path, description = "Dataset ID"),
        ("dp_id" = String, Path, description = "Datapoint ID"),
    ),
    responses(
        (status = 200, description = "Datapoint details", body = Datapoint),
        (status = 404, description = "Datapoint not found"),
    ),
    security(("bearer" = [])),
    tag = "datapoints"
)]
async fn get_datapoint_handler(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path((dataset_id, dp_id)): Path<(DatasetId, DatapointId)>,
) -> Result<Json<Datapoint>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let dp = r.get_datapoint(dp_id).ok_or(StatusCode::NOT_FOUND)?;
    if dp.dataset_id != dataset_id {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(dp.clone()))
}

/// Create a datapoint in a dataset
#[utoipa::path(
    post,
    path = "/api/datasets/{id}/datapoints",
    params(("id" = String, Path, description = "Dataset ID")),
    request_body = CreateDatapointRequest,
    responses(
        (status = 201, description = "Datapoint created", body = Datapoint),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "datapoints"
)]
async fn create_datapoint(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<CreateDatapointRequest>,
) -> Result<(StatusCode, Json<Datapoint>), StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    if w.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let dp = Datapoint::new(dataset_id, req.kind, DatapointSource::Manual);
    w.save_datapoint(dp.clone()).await;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::DatapointCreated { datapoint: dp.clone() });
    Ok((StatusCode::CREATED, Json(dp)))
}

/// Delete a datapoint
#[utoipa::path(
    delete,
    path = "/api/datasets/{id}/datapoints/{dp_id}",
    params(
        ("id" = String, Path, description = "Dataset ID"),
        ("dp_id" = String, Path, description = "Datapoint ID"),
    ),
    responses(
        (status = 200, description = "Datapoint deleted"),
        (status = 404, description = "Datapoint not found"),
    ),
    security(("bearer" = [])),
    tag = "datapoints"
)]
async fn delete_datapoint_handler(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path((dataset_id, dp_id)): Path<(DatasetId, DatapointId)>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::DatasetsWrite) {
        return StatusCode::FORBIDDEN;
    }
    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;
    // Verify the datapoint belongs to this dataset
    if let Some(dp) = w.get_datapoint(dp_id) {
        if dp.dataset_id != dataset_id {
            return StatusCode::NOT_FOUND;
        }
    } else {
        return StatusCode::NOT_FOUND;
    }
    if w.delete_datapoint(dp_id).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

// --- Export span → datapoint ---

/// Export a span as a datapoint into a dataset
#[utoipa::path(
    post,
    path = "/api/datasets/{id}/export-span",
    params(("id" = String, Path, description = "Dataset ID")),
    request_body = ExportSpanRequest,
    responses(
        (status = 201, description = "Datapoint created from span", body = Datapoint),
        (status = 404, description = "Dataset or span not found"),
    ),
    security(("bearer" = [])),
    tag = "datapoints"
)]
async fn export_span_to_dataset(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<ExportSpanRequest>,
) -> Result<(StatusCode, Json<Datapoint>), StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    if w.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let span = w.get(req.span_id).ok_or(StatusCode::NOT_FOUND)?.clone();

    let kind = DatapointKind::Generic {
        input: span.input().cloned().unwrap_or(serde_json::Value::Null),
        expected_output: span.output().cloned(),
        actual_output: None,
        score: None,
        metadata: HashMap::new(),
    };

    let dp = Datapoint::new(dataset_id, kind, DatapointSource::SpanExport)
        .with_source_span(req.span_id);
    w.save_datapoint(dp.clone()).await;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::DatapointCreated { datapoint: dp.clone() });
    Ok((StatusCode::CREATED, Json(dp)))
}

// --- File import (CSV/JSON/JSONL) ---

fn map_object_to_datapoint_kind(obj: &serde_json::Value) -> DatapointKind {
    if let Some(messages) = obj.get("messages") {
        if let Ok(msgs) = serde_json::from_value::<Vec<Message>>(messages.clone()) {
            let expected = obj
                .get("expected")
                .and_then(|v| serde_json::from_value::<Message>(v.clone()).ok());
            let metadata: HashMap<String, serde_json::Value> = obj
                .get("metadata")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            return DatapointKind::LlmConversation {
                messages: msgs,
                expected,
                metadata,
            };
        }
    }

    let input = obj
        .get("input")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let expected_output = obj
        .get("expected_output")
        .or_else(|| obj.get("target"))
        .cloned();
    let actual_output = obj.get("actual_output").cloned();
    let score = obj.get("score").and_then(|v| v.as_f64());
    let metadata: HashMap<String, serde_json::Value> = obj
        .get("metadata")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    DatapointKind::Generic {
        input,
        expected_output,
        actual_output,
        score,
        metadata,
    }
}

fn parse_json_import(data: &[u8]) -> Result<Vec<DatapointKind>, String> {
    // Try as Vec<DatapointKind> first
    if let Ok(kinds) = serde_json::from_slice::<Vec<DatapointKind>>(data) {
        return Ok(kinds);
    }
    // Try as Vec<Value> and map fields
    let arr: Vec<serde_json::Value> =
        serde_json::from_slice(data).map_err(|e| format!("invalid JSON: {}", e))?;
    Ok(arr.iter().map(map_object_to_datapoint_kind).collect())
}

fn parse_jsonl_import(data: &[u8]) -> Result<Vec<DatapointKind>, String> {
    let text = std::str::from_utf8(data).map_err(|e| format!("invalid UTF-8: {}", e))?;
    let mut kinds = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Try as DatapointKind first
        if let Ok(kind) = serde_json::from_str::<DatapointKind>(line) {
            kinds.push(kind);
            continue;
        }
        let obj: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("invalid JSON on line {}: {}", i + 1, e))?;
        kinds.push(map_object_to_datapoint_kind(&obj));
    }
    Ok(kinds)
}

fn parse_csv_import(data: &[u8]) -> Result<Vec<DatapointKind>, String> {
    let mut reader = csv::Reader::from_reader(data);
    let headers = reader
        .headers()
        .map_err(|e| format!("invalid CSV headers: {}", e))?
        .clone();

    let mut kinds = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("CSV parse error: {}", e))?;
        let mut obj = serde_json::Map::new();
        for (header, value) in headers.iter().zip(record.iter()) {
            // Try parsing as JSON, fall back to string
            let json_val = serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
            obj.insert(header.to_string(), json_val);
        }
        kinds.push(map_object_to_datapoint_kind(&serde_json::Value::Object(obj)));
    }
    Ok(kinds)
}

/// Import datapoints from a file (JSON, JSONL, or CSV)
#[utoipa::path(
    post,
    path = "/api/datasets/{id}/import",
    params(("id" = String, Path, description = "Dataset ID")),
    responses(
        (status = 201, description = "Datapoints imported", body = ImportResponse),
        (status = 400, description = "Invalid file format"),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "datapoints"
)]
async fn import_file(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImportResponse>), (StatusCode, String)> {
    if !ctx.has_scope(auth::Scope::DatasetsWrite) {
        return Err((StatusCode::FORBIDDEN, "insufficient permissions: requires DatasetsWrite".to_string()));
    }
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await
        .map_err(|e| (e.0, e.1))?;
    // Verify dataset exists
    {
        let r = store.read().await;
        if r.get_dataset(dataset_id).is_none() {
            return Err((StatusCode::NOT_FOUND, "dataset not found".to_string()));
        }
    }

    let mut imported = 0usize;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart error: {}", e)))?
    {
        let filename = field.file_name().unwrap_or("data").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("read error: {}", e)))?;

        let kinds = if filename.ends_with(".csv") {
            parse_csv_import(&data)
        } else if filename.ends_with(".jsonl") {
            parse_jsonl_import(&data)
        } else {
            parse_json_import(&data)
        }
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

        let mut w = store.write().await;
        for kind in kinds {
            let dp = Datapoint::new(dataset_id, kind, DatapointSource::FileUpload);
            let _ = state
                .events_tx
                .send(SystemEvent::DatapointCreated { datapoint: dp.clone() });
            w.save_datapoint(dp).await;
            imported += 1;
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(ImportResponse {
            imported,
            dataset_id,
        }),
    ))
}

// --- Queue handlers ---

/// List queue items for a dataset (paginated)
#[utoipa::path(
    get,
    path = "/api/datasets/{id}/queue",
    params(("id" = String, Path, description = "Dataset ID"), PaginationParams),
    responses(
        (status = 200, description = "Paginated queue items"),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "queue"
)]
async fn list_queue(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Page<QueueItem>>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    if r.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let items: Vec<QueueItem> = r
        .queue_items_for_dataset(dataset_id)
        .into_iter()
        .cloned()
        .collect();
    let page = paginate(items, &params)?;
    Ok(Json(page))
}

/// Enqueue datapoints for human review
#[utoipa::path(
    post,
    path = "/api/datasets/{id}/queue",
    params(("id" = String, Path, description = "Dataset ID")),
    request_body = EnqueueRequest,
    responses(
        (status = 201, description = "Datapoints enqueued", body = EnqueueResponse),
        (status = 404, description = "Dataset not found"),
    ),
    security(("bearer" = [])),
    tag = "queue"
)]
async fn enqueue_datapoints(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<EnqueueRequest>,
) -> Result<(StatusCode, Json<EnqueueResponse>), StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    if w.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut enqueued = 0;
    for dp_id in req.datapoint_ids {
        if let Some(dp) = w.get_datapoint(dp_id) {
            if dp.dataset_id != dataset_id {
                continue;
            }
            let original_data = serde_json::to_value(&dp.kind).ok();
            let item = QueueItem::new(dataset_id, dp_id, original_data);
            let _ = state
                .events_tx
                .send(SystemEvent::QueueItemUpdated { item: item.clone() });
            w.save_queue_item(item).await;
            enqueued += 1;
        }
    }
    Ok((StatusCode::CREATED, Json(EnqueueResponse { enqueued })))
}

/// Claim a queue item for review
#[utoipa::path(
    post,
    path = "/api/queue/{item_id}/claim",
    params(("item_id" = String, Path, description = "Queue item ID")),
    request_body = ClaimRequest,
    responses(
        (status = 200, description = "Item claimed", body = QueueItem),
        (status = 409, description = "Item already claimed"),
    ),
    security(("bearer" = [])),
    tag = "queue"
)]
async fn claim_queue_item(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(item_id): Path<QueueItemId>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<QueueItem>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    let item = w
        .claim_queue_item(item_id, req.claimed_by)
        .await
        .ok_or(StatusCode::CONFLICT)?;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::QueueItemUpdated { item: item.clone() });
    Ok(Json(item))
}

/// Submit a reviewed queue item
#[utoipa::path(
    post,
    path = "/api/queue/{item_id}/submit",
    params(("item_id" = String, Path, description = "Queue item ID")),
    request_body = SubmitRequest,
    responses(
        (status = 200, description = "Item submitted", body = QueueItem),
        (status = 409, description = "Item not in claimed state"),
    ),
    security(("bearer" = [])),
    tag = "queue"
)]
async fn submit_queue_item(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(item_id): Path<QueueItemId>,
    Json(req): Json<SubmitRequest>,
) -> Result<Json<QueueItem>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;

    // Get the queue item to find the datapoint
    let qi = w.get_queue_item(item_id).ok_or(StatusCode::NOT_FOUND)?.clone();
    if qi.status != QueueItemStatus::Claimed {
        return Err(StatusCode::CONFLICT);
    }

    // If edited_data provided, update the datapoint's kind
    if let Some(ref edited) = req.edited_data {
        if let Some(dp) = w.get_datapoint(qi.datapoint_id).cloned() {
            let new_kind = if let Ok(kind) = serde_json::from_value::<DatapointKind>(edited.clone())
            {
                kind
            } else {
                map_object_to_datapoint_kind(edited)
            };
            let updated_dp = Datapoint {
                kind: new_kind,
                ..dp
            };
            w.save_datapoint(updated_dp).await;
        }
    }

    let item = w
        .complete_queue_item(item_id, req.edited_data)
        .await
        .ok_or(StatusCode::CONFLICT)?;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::QueueItemUpdated { item: item.clone() });
    Ok(Json(item))
}

// --- Analytics handlers ---

/// Query analytics with filters and grouping
#[utoipa::path(
    post,
    path = "/api/analytics",
    request_body = AnalyticsQuery,
    responses(
        (status = 200, description = "Analytics results", body = AnalyticsResponse),
    ),
    security(("bearer" = [])),
    tag = "analytics"
)]
async fn post_analytics(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Json(query): Json<AnalyticsQuery>,
) -> Result<Json<AnalyticsResponse>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::AnalyticsRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    let filter = SpanFilter {
        kind: query.filter.kind.clone(),
        model: query.filter.model.clone(),
        provider: query.filter.provider.clone(),
        status: query.filter.status.clone(),
        since: query.filter.since,
        until: query.filter.until,
        trace_id: query.filter.trace_id,
        ..Default::default()
    };
    let spans = r.filter_spans(&filter);
    let response = analytics::compute_analytics(&spans, &query);
    Ok(Json(response))
}

/// Get analytics summary (totals, costs, top models)
#[utoipa::path(
    get,
    path = "/api/analytics/summary",
    responses(
        (status = 200, description = "Analytics summary", body = AnalyticsSummary),
    ),
    security(("bearer" = [])),
    tag = "analytics"
)]
async fn analytics_summary(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> Result<Json<AnalyticsSummary>, (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::AnalyticsRead)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;
    let r = store.read().await;
    let spans: Vec<&trace::Span> = r.all_spans().collect();
    let trace_count = r.trace_count();
    let summary = analytics::compute_summary(&spans, trace_count);
    Ok(Json(summary))
}

// --- Eval Run handlers ---

/// List eval runs for a dataset (paginated)
async fn list_eval_runs(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Page<EvalRun>>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    if r.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let runs: Vec<EvalRun> = r.eval_runs_for_dataset(dataset_id).into_iter().cloned().collect();
    let page = paginate(runs, &params)?;
    Ok(Json(page))
}

/// Create and start an eval run for a dataset
async fn create_eval_run(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<CreateEvalRunRequest>,
) -> Result<(StatusCode, Json<EvalRun>), (StatusCode, Json<serde_json::Value>)> {
    require_scope(&ctx, auth::Scope::DatasetsWrite)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_json)?;

    // Verify dataset exists and count datapoints
    {
        let r = store.read().await;
        if r.get_dataset(dataset_id).is_none() {
            return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "dataset not found"}))));
        }
    }

    let mut run = EvalRun::new(dataset_id, req.name, req.config, req.scoring);

    // Create a trace for this eval run
    let eval_trace = Trace::new(Some(format!("eval: {}", run.name.as_deref().unwrap_or("unnamed"))));
    run.trace_id = Some(eval_trace.id);

    let mut w = store.write().await;
    w.save_trace(eval_trace).await;
    w.save_eval_run(run.clone()).await;
    drop(w);

    let _ = state.events_tx.send(SystemEvent::EvalRunCreated { run: run.clone() });

    // Spawn the eval execution task
    let run_id = run.id;
    let events_tx = state.events_tx.clone();
    let store_clone = store.clone();
    tokio::spawn(async move {
        execute_eval_run(run_id, store_clone, events_tx).await;
    });

    Ok((StatusCode::CREATED, Json(run)))
}

/// Get eval run details with results
async fn get_eval_run_handler(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(run_id): Path<EvalRunId>,
) -> Result<Json<EvalRunDetailResponse>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let run = r.get_eval_run(run_id).ok_or(StatusCode::NOT_FOUND)?.clone();
    let results: Vec<EvalResult> = r.eval_results_for_run(run_id).into_iter().cloned().collect();
    Ok(Json(EvalRunDetailResponse { run, result_items: results }))
}

/// Delete an eval run
async fn delete_eval_run_handler(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(run_id): Path<EvalRunId>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::DatasetsWrite) {
        return StatusCode::FORBIDDEN;
    }
    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;
    if w.delete_eval_run(run_id).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Cancel a running eval
async fn cancel_eval_run(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(run_id): Path<EvalRunId>,
) -> Result<Json<EvalRun>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    let run = w.get_eval_run(run_id).ok_or(StatusCode::NOT_FOUND)?.clone();
    if run.status.is_terminal() {
        return Err(StatusCode::CONFLICT);
    }
    let mut updated = run;
    updated.status = EvalRunStatus::Cancelled;
    updated.completed_at = Some(chrono::Utc::now());
    w.save_eval_run(updated.clone()).await;
    drop(w);
    let _ = state.events_tx.send(SystemEvent::EvalRunCompleted { run: updated.clone() });
    Ok(Json(updated))
}

/// Compare eval runs
async fn compare_eval_runs(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Query(params): Query<CompareParams>,
) -> Result<Json<ComparisonResponse>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    if r.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let run_ids: Vec<EvalRunId> = params.runs.split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if run_ids.len() < 2 || run_ids.len() > 4 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut runs = Vec::new();
    let mut all_results: std::collections::HashMap<EvalRunId, Vec<EvalResult>> = std::collections::HashMap::new();

    for run_id in &run_ids {
        let run = r.get_eval_run(*run_id).ok_or(StatusCode::NOT_FOUND)?.clone();
        if run.dataset_id != dataset_id {
            return Err(StatusCode::BAD_REQUEST);
        }
        let results: Vec<EvalResult> = r.eval_results_for_run(*run_id).into_iter().cloned().collect();
        all_results.insert(*run_id, results);
        runs.push(run);
    }

    // Build comparison matrix by datapoint
    let datapoints: Vec<Datapoint> = r.datapoints_for_dataset(dataset_id).into_iter().cloned().collect();
    let mut comparison_datapoints = Vec::new();

    for dp in &datapoints {
        let (input, expected) = match &dp.kind {
            DatapointKind::Generic { input, expected_output, .. } => (input.clone(), expected_output.clone()),
            DatapointKind::LlmConversation { messages, expected, .. } => {
                (serde_json::to_value(messages).unwrap_or_default(), expected.as_ref().map(|e| serde_json::to_value(e).unwrap_or_default()))
            }
        };

        let mut results_map = std::collections::HashMap::new();
        for run_id in &run_ids {
            if let Some(run_results) = all_results.get(run_id) {
                if let Some(result) = run_results.iter().find(|r| r.datapoint_id == dp.id) {
                    results_map.insert(run_id.to_string(), ComparisonCell {
                        output: result.actual_output.clone(),
                        score: result.score,
                        latency_ms: result.latency_ms,
                        status: result.status.as_str().to_string(),
                    });
                }
            }
        }

        comparison_datapoints.push(ComparisonDatapoint {
            datapoint_id: dp.id,
            input,
            expected,
            results: results_map,
        });
    }

    Ok(Json(ComparisonResponse {
        runs,
        datapoints: comparison_datapoints,
    }))
}

// --- Capture Rule handlers ---

/// List capture rules for a dataset (paginated)
async fn list_capture_rules(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Page<CaptureRule>>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    if r.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let rules: Vec<CaptureRule> = r.capture_rules_for_dataset(dataset_id).into_iter().cloned().collect();
    let page = paginate(rules, &params)?;
    Ok(Json(page))
}

/// Create a capture rule
async fn create_capture_rule(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<CreateCaptureRuleRequest>,
) -> Result<(StatusCode, Json<CaptureRule>), StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    if w.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let rule = CaptureRule::new(dataset_id, req.name, req.filters, req.sample_rate);
    w.save_capture_rule(rule.clone()).await;
    Ok((StatusCode::CREATED, Json(rule)))
}

/// Update a capture rule
async fn update_capture_rule(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(rule_id): Path<CaptureRuleId>,
    Json(req): Json<UpdateCaptureRuleRequest>,
) -> Result<Json<CaptureRule>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    let rule = w.get_capture_rule(rule_id).ok_or(StatusCode::NOT_FOUND)?.clone();
    let mut updated = rule;
    if let Some(name) = req.name {
        updated.name = name;
    }
    if let Some(filters) = req.filters {
        updated.filters = filters;
    }
    if let Some(sample_rate) = req.sample_rate {
        updated.sample_rate = sample_rate;
    }
    w.save_capture_rule(updated.clone()).await;
    Ok(Json(updated))
}

/// Delete a capture rule
async fn delete_capture_rule_handler(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(rule_id): Path<CaptureRuleId>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::DatasetsWrite) {
        return StatusCode::FORBIDDEN;
    }
    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;
    if w.delete_capture_rule(rule_id).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Toggle a capture rule enabled/disabled
async fn toggle_capture_rule(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(rule_id): Path<CaptureRuleId>,
) -> Result<Json<CaptureRule>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    let rule = w.get_capture_rule(rule_id).ok_or(StatusCode::NOT_FOUND)?.clone();
    let mut updated = rule;
    updated.enabled = !updated.enabled;
    w.save_capture_rule(updated.clone()).await;
    Ok(Json(updated))
}

// --- Provider Connection handlers ---

async fn list_provider_connections(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> Result<Json<ProviderConnectionListResponse>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let connections: Vec<ProviderConnectionInfo> = r.list_provider_connections().iter().map(|c| c.to_info()).collect();
    let count = connections.len();
    Ok(Json(ProviderConnectionListResponse { connections, count }))
}

async fn create_provider_connection(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Json(req): Json<CreateProviderConnectionRequest>,
) -> Result<(StatusCode, Json<ProviderConnectionInfo>), StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut conn = ProviderConnection::new(req.name, req.provider);
    conn.base_url = req.base_url;
    conn.api_key = req.api_key;
    conn.default_model = req.default_model;
    let info = conn.to_info();
    let mut w = store.write().await;
    w.save_provider_connection(conn).await;
    Ok((StatusCode::CREATED, Json(info)))
}

async fn update_provider_connection(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(conn_id): Path<ProviderConnectionId>,
    Json(req): Json<UpdateProviderConnectionRequest>,
) -> Result<Json<ProviderConnectionInfo>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsWrite).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let mut w = store.write().await;
    let mut conn = w.get_provider_connection(conn_id).ok_or(StatusCode::NOT_FOUND)?.clone();
    if let Some(name) = req.name { conn.name = name; }
    if let Some(provider) = req.provider { conn.provider = provider; }
    if let Some(base_url) = req.base_url { conn.base_url = Some(base_url); }
    if let Some(api_key) = req.api_key { conn.api_key = Some(api_key); }
    if let Some(default_model) = req.default_model { conn.default_model = Some(default_model); }
    conn.updated_at = chrono::Utc::now();
    let info = conn.to_info();
    w.save_provider_connection(conn).await;
    Ok(Json(info))
}

async fn delete_provider_connection_handler(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(conn_id): Path<ProviderConnectionId>,
) -> StatusCode {
    if !ctx.has_scope(auth::Scope::DatasetsWrite) {
        return StatusCode::FORBIDDEN;
    }
    let store = match state.store_for_project(ctx.org_id, ctx.project_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut w = store.write().await;
    if w.delete_provider_connection(conn_id).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

// --- Provider connection: test & list models ---

/// Resolve a provider's base URL from its name if not explicitly provided.
fn default_base_url_for_provider(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "https://api.anthropic.com/v1",
        "ollama" => "http://localhost:11434/v1",
        _ => "https://api.openai.com/v1",
    }
}

/// Fetch models from an OpenAI-compatible `/models` endpoint.
async fn fetch_provider_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<ProviderModelInfo>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut req = client.get(&url).header("Accept", "application/json");
    if let Some(key) = api_key {
        if !key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
    }

    let resp = req.timeout(std::time::Duration::from_secs(10)).send().await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, body.chars().take(200).collect::<String>()));
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| format!("Invalid JSON response: {}", e))?;

    let mut models = Vec::new();
    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                let name = item.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                models.push(ProviderModelInfo { id: id.to_string(), name });
            }
        }
    }

    // Sort alphabetically by id
    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}

/// POST /api/provider-connections/test — test a connection without saving it.
/// Also returns available models on success.
async fn test_provider_connection(
    auth::Auth(ctx): auth::Auth,
    Json(req): Json<TestProviderConnectionRequest>,
) -> Json<ProviderModelsResponse> {
    if !ctx.has_scope(auth::Scope::DatasetsRead) {
        return Json(ProviderModelsResponse { models: vec![], ok: false, error: Some("Forbidden".into()) });
    }

    let base_url = req.base_url.as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_base_url_for_provider(&req.provider));

    match fetch_provider_models(base_url, req.api_key.as_deref()).await {
        Ok(models) => Json(ProviderModelsResponse { models, ok: true, error: None }),
        Err(e) => Json(ProviderModelsResponse { models: vec![], ok: false, error: Some(e) }),
    }
}

/// GET /api/provider-connections/:conn_id/models — fetch models for a saved connection.
async fn list_provider_connection_models(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(conn_id): Path<ProviderConnectionId>,
) -> Result<Json<ProviderModelsResponse>, StatusCode> {
    require_scope(&ctx, auth::Scope::DatasetsRead).map_err(|_| StatusCode::FORBIDDEN)?;
    let store = state.store_for_project(ctx.org_id, ctx.project_id).await.map_err(store_err_status)?;
    let r = store.read().await;
    let conn = r.get_provider_connection(conn_id).ok_or(StatusCode::NOT_FOUND)?;

    let base_url = conn.base_url.as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_base_url_for_provider(&conn.provider));

    let result = match fetch_provider_models(base_url, conn.api_key.as_deref()).await {
        Ok(models) => ProviderModelsResponse { models, ok: true, error: None },
        Err(e) => ProviderModelsResponse { models: vec![], ok: false, error: Some(e) },
    };
    Ok(Json(result))
}

// --- Eval execution engine ---

async fn execute_eval_run(
    run_id: EvalRunId,
    store: SharedStore,
    events_tx: broadcast::Sender<SystemEvent>,
) {
    use std::time::Instant as StdInstant;

    // Get the run and datapoints
    let (mut run, datapoints, scoring) = {
        let r = store.read().await;
        let run = match r.get_eval_run(run_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let datapoints: Vec<Datapoint> = r.datapoints_for_dataset(run.dataset_id).into_iter().cloned().collect();
        let scoring = run.scoring.clone();
        (run, datapoints, scoring)
    };

    // Mark as running
    run.status = EvalRunStatus::Running;
    run.results.total = datapoints.len();
    {
        let mut w = store.write().await;
        w.save_eval_run(run.clone()).await;
    }
    let _ = events_tx.send(SystemEvent::EvalRunUpdated { run: run.clone() });

    // Resolve provider connection if referenced
    let provider_conn: Option<ProviderConnection> = if let Some(conn_id) = run.config.provider_connection_id {
        let r = store.read().await;
        r.get_provider_connection(conn_id).cloned()
    } else {
        None
    };

    // Build the LLM client URL — provider connection takes precedence
    let provider = provider_conn.as_ref().map(|c| c.provider.as_str())
        .or(run.config.provider.as_deref())
        .unwrap_or("openai");
    let base_url = provider_conn.as_ref().and_then(|c| c.base_url.clone())
        .or_else(|| run.config.provider_url.clone())
        .unwrap_or_else(|| {
            match provider {
                "anthropic" => "https://api.anthropic.com/v1".to_string(),
                "ollama" => "http://localhost:11434/v1".to_string(),
                _ => "https://api.openai.com/v1".to_string(),
            }
        });

    // Get API key — provider connection takes precedence, then env var fallback
    let api_key = if let Some(ref conn) = provider_conn {
        conn.api_key.clone().unwrap_or_default()
    } else {
        let api_key_env = run.config.api_key_env.as_deref().unwrap_or(match provider {
            "anthropic" => "ANTHROPIC_API_KEY",
            "ollama" => "",
            _ => "OPENAI_API_KEY",
        });
        if api_key_env.is_empty() {
            String::new()
        } else {
            std::env::var(api_key_env).unwrap_or_default()
        }
    };

    let client = reqwest::Client::new();
    let mut completed = 0usize;
    let mut failed = 0usize;
    let mut scores: Vec<f64> = Vec::new();

    for dp in &datapoints {
        // Check if run was cancelled
        {
            let r = store.read().await;
            if let Some(current) = r.get_eval_run(run_id) {
                if current.status == EvalRunStatus::Cancelled {
                    return;
                }
            }
        }

        let mut eval_result = EvalResult::new(run_id, dp.id);

        // Build the messages from the datapoint
        let messages = match &dp.kind {
            DatapointKind::LlmConversation { messages, .. } => {
                let mut msgs: Vec<serde_json::Value> = messages.iter().map(|m| {
                    serde_json::json!({"role": m.role, "content": m.content})
                }).collect();
                if let Some(ref sys) = run.config.system_prompt {
                    msgs.insert(0, serde_json::json!({"role": "system", "content": sys}));
                }
                msgs
            }
            DatapointKind::Generic { input, .. } => {
                let user_content = if input.is_string() {
                    input.as_str().unwrap_or("").to_string()
                } else {
                    serde_json::to_string(input).unwrap_or_default()
                };
                let mut msgs = Vec::new();
                if let Some(ref sys) = run.config.system_prompt {
                    msgs.push(serde_json::json!({"role": "system", "content": sys}));
                }
                msgs.push(serde_json::json!({"role": "user", "content": user_content}));
                msgs
            }
        };

        // Make the LLM call
        let start = StdInstant::now();
        let mut request_body = serde_json::json!({
            "model": run.config.model,
            "messages": messages,
        });
        if let Some(temp) = run.config.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tok) = run.config.max_tokens {
            request_body["max_tokens"] = serde_json::json!(max_tok);
        }

        let url = format!("{}/chat/completions", base_url);
        let resp: Result<reqwest::Response, reqwest::Error> = client.post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .send()
            .await;

        eval_result.latency_ms = start.elapsed().as_millis() as u64;

        match resp {
            Ok(response) => {
                let resp_status = response.status();
                match response.bytes().await {
                    Ok(body_bytes) if resp_status.is_success() => {
                        match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                            Ok(body) => {
                                let content = body.pointer("/choices/0/message/content")
                                    .cloned()
                                    .unwrap_or(serde_json::Value::Null);
                                eval_result.actual_output = content;

                                // Extract token usage
                                if let Some(usage) = body.get("usage") {
                                    eval_result.input_tokens = usage.get("prompt_tokens").and_then(|v: &serde_json::Value| v.as_u64()).map(|v| v as u32);
                                    eval_result.output_tokens = usage.get("completion_tokens").and_then(|v: &serde_json::Value| v.as_u64()).map(|v| v as u32);
                                }

                                // Score the result
                                let expected = match &dp.kind {
                                    DatapointKind::LlmConversation { expected, .. } => expected.as_ref().map(|e| serde_json::Value::String(e.content.clone())),
                                    DatapointKind::Generic { expected_output, .. } => expected_output.clone(),
                                };

                                match &scoring {
                                    ScoringStrategy::ExactMatch => {
                                        if let Some(ref exp) = expected {
                                            let actual_str = eval_result.actual_output.as_str().unwrap_or("");
                                            let expected_str = exp.as_str().unwrap_or("");
                                            let score = if actual_str.trim() == expected_str.trim() { 1.0 } else { 0.0 };
                                            eval_result.score = Some(score);
                                            eval_result.status = if score >= 0.5 { trace::EvalResultStatus::Passed } else { trace::EvalResultStatus::Failed };
                                        } else {
                                            eval_result.status = trace::EvalResultStatus::Passed;
                                        }
                                    }
                                    ScoringStrategy::Contains => {
                                        if let Some(ref exp) = expected {
                                            let actual_str = eval_result.actual_output.as_str().unwrap_or("").to_lowercase();
                                            let expected_str = exp.as_str().unwrap_or("").to_lowercase();
                                            let score = if actual_str.contains(&expected_str) { 1.0 } else { 0.0 };
                                            eval_result.score = Some(score);
                                            eval_result.status = if score >= 0.5 { trace::EvalResultStatus::Passed } else { trace::EvalResultStatus::Failed };
                                        } else {
                                            eval_result.status = trace::EvalResultStatus::Passed;
                                        }
                                    }
                                    ScoringStrategy::LlmJudge => {
                                        // For LLM judge, we make a second call to the same model
                                        let judge_messages = vec![
                                            serde_json::json!({"role": "system", "content": "You are an evaluation judge. Score the response on a scale of 0.0 to 1.0. Respond with ONLY a JSON object: {\"score\": <number>, \"reason\": \"<brief explanation>\"}"}),
                                            serde_json::json!({"role": "user", "content": format!(
                                                "Input: {}\nExpected: {}\nActual: {}\n\nScore the actual response.",
                                                serde_json::to_string(&messages.last()).unwrap_or_default(),
                                                expected.as_ref().map(|e| serde_json::to_string(e).unwrap_or_default()).unwrap_or_else(|| "N/A".to_string()),
                                                serde_json::to_string(&eval_result.actual_output).unwrap_or_default(),
                                            )}),
                                        ];
                                        let judge_body = serde_json::json!({
                                            "model": run.config.model,
                                            "messages": judge_messages,
                                            "temperature": 0,
                                        });
                                        let judge_result_resp: Result<reqwest::Response, _> = client.post(&url)
                                            .header("Content-Type", "application/json")
                                            .header("Authorization", format!("Bearer {}", api_key))
                                            .json(&judge_body)
                                            .send()
                                            .await;
                                        if let Ok(judge_resp) = judge_result_resp {
                                            if let Ok(judge_bytes) = judge_resp.bytes().await {
                                                if let Ok(judge_body) = serde_json::from_slice::<serde_json::Value>(&judge_bytes) {
                                                    let judge_content = judge_body.pointer("/choices/0/message/content")
                                                        .and_then(|v: &serde_json::Value| v.as_str())
                                                        .unwrap_or("");
                                                    if let Ok(judge_parsed) = serde_json::from_str::<serde_json::Value>(judge_content) {
                                                        eval_result.score = judge_parsed.get("score").and_then(|v: &serde_json::Value| v.as_f64());
                                                        eval_result.score_reason = judge_parsed.get("reason").and_then(|v: &serde_json::Value| v.as_str()).map(|s| s.to_string());
                                                    }
                                                }
                                            }
                                        }
                                        eval_result.status = if eval_result.score.unwrap_or(0.0) >= 0.5 {
                                            trace::EvalResultStatus::Passed
                                        } else {
                                            trace::EvalResultStatus::Failed
                                        };
                                    }
                                    ScoringStrategy::None => {
                                        eval_result.status = trace::EvalResultStatus::Passed;
                                    }
                                }

                                if let Some(s) = eval_result.score {
                                    scores.push(s);
                                }
                                completed += 1;
                            }
                            Err(e) => {
                                eval_result.status = trace::EvalResultStatus::Error;
                                eval_result.error = Some(format!("Failed to parse response: {}", e));
                                failed += 1;
                            }
                        }
                    }
                    Ok(body_bytes) => {
                        let error_text = String::from_utf8_lossy(&body_bytes);
                        eval_result.status = trace::EvalResultStatus::Error;
                        eval_result.error = Some(format!("HTTP {}: {}", resp_status, error_text));
                        failed += 1;
                    }
                    Err(e) => {
                        eval_result.status = trace::EvalResultStatus::Error;
                        eval_result.error = Some(format!("Failed to read response: {}", e));
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                eval_result.status = trace::EvalResultStatus::Error;
                eval_result.error = Some(format!("Request failed: {}", e));
                failed += 1;
            }
        }

        // Save the result
        {
            let mut w = store.write().await;
            w.save_eval_result(eval_result).await;

            // Update run progress
            if let Some(current_run) = w.get_eval_run(run_id).cloned() {
                let mut updated = current_run;
                updated.results.completed = completed;
                updated.results.failed = failed;
                updated.results.scores = compute_score_summary(&scores);
                w.save_eval_run(updated.clone()).await;
                let _ = events_tx.send(SystemEvent::EvalRunUpdated { run: updated });
            }
        }
    }

    // Mark as completed
    {
        let mut w = store.write().await;
        if let Some(current_run) = w.get_eval_run(run_id).cloned() {
            let mut final_run = current_run;
            if final_run.status != EvalRunStatus::Cancelled {
                final_run.status = if failed > 0 && completed == 0 {
                    EvalRunStatus::Failed
                } else {
                    EvalRunStatus::Completed
                };
            }
            final_run.completed_at = Some(chrono::Utc::now());
            final_run.results.completed = completed;
            final_run.results.failed = failed;
            final_run.results.scores = compute_score_summary(&scores);
            w.save_eval_run(final_run.clone()).await;
            let _ = events_tx.send(SystemEvent::EvalRunCompleted { run: final_run });
        }
    }
}

fn compute_score_summary(scores: &[f64]) -> ScoreSummary {
    if scores.is_empty() {
        return ScoreSummary::default();
    }
    let mut sorted = scores.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sum: f64 = sorted.iter().sum();
    let mean = sum / sorted.len() as f64;
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };
    let pass_count = sorted.iter().filter(|&&s| s >= 0.5).count();
    ScoreSummary {
        mean: Some(mean),
        median: Some(median),
        min: sorted.first().copied(),
        max: sorted.last().copied(),
        pass_rate: Some(pass_count as f64 / sorted.len() as f64),
    }
}

// --- Config handlers ---

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
    // Write TOML to the config file path
    let config_path = state.config_path.as_str();
    if config_path.is_empty() {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "config path not set".to_string()));
    }

    // Validate that the JSON can be converted to TOML
    let toml_str = toml::to_string_pretty(&new_config)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid config: {}", e)))?;

    // Write to disk
    let path = std::path::Path::new(config_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to create config directory: {}", e)))?;
    }
    std::fs::write(path, &toml_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to write config: {}", e)))?;

    // Update in-memory config
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

    // Try the exact path first
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

/// Scalar API docs page served at the root in cloud mode.
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

// --- Auth Middleware ---

/// Cached auth result with expiry.
struct CachedAuth {
    ctx: auth::AuthContext,
    expires: std::time::Instant,
}

/// Auth middleware shared state.
#[derive(Clone)]
struct AuthMiddlewareState {
    config: auth::AuthConfig,
    lookup: Arc<dyn auth::ApiKeyLookup>,
    /// Cache: bearer token -> (AuthContext, expiry). TTL = 60s.
    cache: Arc<std::sync::Mutex<HashMap<String, CachedAuth>>>,
}

/// Tower middleware layer that injects `AuthContext` into every request.
#[derive(Clone)]
struct AuthLayer {
    state: AuthMiddlewareState,
}

impl<S> tower::Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            state: self.state.clone(),
        }
    }
}

/// Tower middleware service that extracts auth and injects `AuthContext`.
#[derive(Clone)]
struct AuthMiddleware<S> {
    inner: S,
    state: AuthMiddlewareState,
}

impl<S> tower::Service<Request<Body>> for AuthMiddleware<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();
        // Swap to make sure we have a ready service
        std::mem::swap(&mut self.inner, &mut inner);

        Box::pin(async move {
            if state.config.local_mode {
                request.extensions_mut().insert(auth::AuthContext::local());
                return inner.call(request).await;
            }

            // Extract header values before async boundary (Body is not Sync)
            let auth_header = request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok().map(String::from));
            let cookie_header = request
                .headers()
                .get(header::COOKIE)
                .and_then(|v| v.to_str().ok().map(String::from));
            let query_string = request.uri().query().map(String::from);

            // Check cache for Bearer tokens (avoids repeated bcrypt + DB lookup)
            let cached_ctx = auth_header.as_ref().and_then(|token| {
                let cache = state.cache.lock().ok()?;
                let cached = cache.get(token.as_str())?;
                if cached.expires > Instant::now() {
                    Some(cached.ctx.clone())
                } else {
                    None
                }
            });
            if let Some(ctx) = cached_ctx {
                request.extensions_mut().insert(ctx);
                return inner.call(request).await;
            }

            match extract_auth(
                auth_header.as_deref(),
                cookie_header.as_deref(),
                query_string.as_deref(),
                &state.config,
                state.lookup.as_ref(),
            ).await {
                Ok(ctx) => {
                    // Cache successful Bearer auth for 60 seconds
                    if let Some(ref token) = auth_header {
                        if let Ok(mut cache) = state.cache.lock() {
                            if cache.len() > 100 {
                                let now = Instant::now();
                                cache.retain(|_, v| v.expires > now);
                            }
                            cache.insert(token.clone(), CachedAuth {
                                ctx: ctx.clone(),
                                expires: Instant::now() + std::time::Duration::from_secs(60),
                            });
                        }
                    }
                    request.extensions_mut().insert(ctx);
                    inner.call(request).await
                }
                Err(e) => Ok(e.into_response()),
            }
        })
    }
}

/// Extract auth context from pre-extracted request headers/query.
async fn extract_auth(
    auth_header: Option<&str>,
    cookie_header: Option<&str>,
    query_string: Option<&str>,
    config: &auth::AuthConfig,
    lookup: &dyn auth::ApiKeyLookup,
) -> Result<auth::AuthContext, auth::AuthError> {
    // Check Authorization header
    if let Some(auth_str) = auth_header {
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            // API key format: tw_sk_...
            if token.starts_with("tw_sk_") {
                let prefix = if token.len() >= 16 { &token[..16] } else {
                    return Err(auth::AuthError::InvalidApiKey);
                };
                let (org_id, project_id, key_hash, scopes) = lookup
                    .lookup_api_key(prefix)
                    .await
                    .ok_or(auth::AuthError::InvalidApiKey)?;
                if !auth::verify_api_key(token, &key_hash) {
                    return Err(auth::AuthError::InvalidApiKey);
                }
                return Ok(auth::AuthContext::from_api_key(org_id, project_id, scopes));
            }
            // JWT session token
            let session = auth::verify_session(token, &config.jwt_secret)?;
            return Ok(auth::AuthContext::from_session(session.org_id, session.project_id, session.user_id, session.scopes));
        }

        return Err(auth::AuthError::InvalidFormat);
    }

    // Check session cookie
    if let Some(cookie_str) = cookie_header {
        for c in cookie_str.split(';') {
            let c = c.trim();
            if let Some(value) = c.strip_prefix("session=") {
                if !value.is_empty() {
                    let session = auth::verify_session(value, &config.jwt_secret)?;
                    return Ok(auth::AuthContext::from_session(session.org_id, session.project_id, session.user_id, session.scopes));
                }
            }
        }
    }

    // Check query param (for SSE)
    if let Some(query) = query_string {
        if let Some(token) = auth_keys::extract_token_from_query(query) {
            let session = auth::verify_session(&token, &config.jwt_secret)?;
            return Ok(auth::AuthContext::from_session(session.org_id, session.project_id, session.user_id, session.scopes));
        }
    }

    Err(auth::AuthError::MissingAuth)
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
    auth_store: Option<Arc<dyn auth::AuthStore>>,
    api_key_lookup: Option<Arc<dyn auth::ApiKeyLookup>>,
    email_sender: Option<Arc<dyn auth::EmailSender>>,
    app_url: Option<String>,
    polar_webhook_secret: Option<String>,
    polar_access_token: Option<String>,
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
            auth_store: None,
            api_key_lookup: None,
            email_sender: None,
            app_url: None,
            polar_webhook_secret: None,
            polar_access_token: None,
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
            auth_store: None,
            api_key_lookup: None,
            email_sender: None,
            app_url: None,
            polar_webhook_secret: None,
            polar_access_token: None,
        }
    }

    /// Use a per-org store manager instead of a single store.
    pub fn org_stores(mut self, m: Arc<OrgStoreManager>) -> Self { self.org_stores = m; self }
    pub fn start_time(mut self, t: Instant) -> Self { self.start_time = t; self }
    pub fn config(mut self, c: serde_json::Value) -> Self { self.config = c; self }
    pub fn config_path(mut self, p: String) -> Self { self.config_path = p; self }
    pub fn shutdown_tx(mut self, tx: watch::Sender<bool>) -> Self { self.shutdown_tx = Some(tx); self }
    pub fn auth_config(mut self, c: auth::AuthConfig) -> Self { self.auth_config = c; self }
    pub fn auth_store(mut self, s: Arc<dyn auth::AuthStore>) -> Self { self.auth_store = Some(s); self }
    pub fn api_key_lookup(mut self, l: Arc<dyn auth::ApiKeyLookup>) -> Self { self.api_key_lookup = Some(l); self }
    pub fn email_sender(mut self, e: Arc<dyn auth::EmailSender>) -> Self { self.email_sender = Some(e); self }
    pub fn app_url(mut self, u: String) -> Self { self.app_url = Some(u); self }
    pub fn polar_webhook_secret(mut self, s: String) -> Self { self.polar_webhook_secret = Some(s); self }
    pub fn polar_access_token(mut self, s: String) -> Self { self.polar_access_token = Some(s); self }

    pub fn build(self) -> Router {
        build_router(
            self.org_stores,
            self.start_time,
            self.config,
            self.config_path,
            self.shutdown_tx,
            self.auth_config,
            self.auth_store,
            self.api_key_lookup,
            self.email_sender,
            self.app_url,
            self.polar_webhook_secret,
            self.polar_access_token,
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
    build_router(org_stores, start_time, config, config_path, shutdown_tx, auth::AuthConfig::local(), None, None, None, None, None, None)
}

fn build_router(
    org_stores: Arc<OrgStoreManager>,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
    auth_config: auth::AuthConfig,
    auth_store: Option<Arc<dyn auth::AuthStore>>,
    api_key_lookup: Option<Arc<dyn auth::ApiKeyLookup>>,
    email_sender: Option<Arc<dyn auth::EmailSender>>,
    app_url: Option<String>,
    polar_webhook_secret: Option<String>,
    polar_access_token: Option<String>,
) -> Router {
    let (events_tx, _) = broadcast::channel(256);
    let api_key_lookup = api_key_lookup.unwrap_or_else(|| {
        Arc::new(auth_keys::NoopApiKeyLookup) as Arc<dyn auth::ApiKeyLookup>
    });
    let email_sender = email_sender.unwrap_or_else(|| {
        Arc::new(auth::NoopEmailSender) as Arc<dyn auth::EmailSender>
    });
    let app_url = app_url.unwrap_or_else(|| "http://localhost:3000".to_string());
    let state = AppState {
        org_stores,
        events_tx,
        start_time,
        config: Arc::new(RwLock::new(config)),
        config_path: Arc::new(config_path),
        shutdown_tx,
        auth_config: auth_config.clone(),
        auth_store,
        api_key_lookup: api_key_lookup.clone(),
        email_sender,
        app_url,
        polar_webhook_secret,
        polar_access_token,
    };

    // In cloud mode with a separate frontend origin, we need explicit origins
    // and credentials support. ALLOWED_ORIGINS env var is comma-separated.
    // In local mode (no env var), allow any origin without credentials.
    let cors = if let Ok(origins) = std::env::var("ALLOWED_ORIGINS") {
        let origins: Vec<axum::http::HeaderValue> = origins
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
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

    // Auth middleware state (injected as Extension for `from_fn`)
    let auth_mw_state = AuthMiddlewareState {
        config: state.auth_config.clone(),
        lookup: state.api_key_lookup.clone(),
        cache: Arc::new(std::sync::Mutex::new(HashMap::new())),
    };

    // Protected routes (auth middleware applied)
    let protected = Router::new()
        // Traces
        .route("/traces", get(list_traces).post(create_trace).delete(clear_all_traces))
        .route("/traces/:trace_id", get(get_trace).delete(delete_trace))
        // Spans
        .route("/spans", get(list_spans).post(create_span))
        .route("/spans/:span_id", get(get_span).delete(delete_span))
        .route("/spans/:span_id/complete", post(complete_span))
        .route("/spans/:span_id/fail", post(fail_span))
        // Files
        .route("/files", get(list_files))
        .route("/files/content/:hash", get(get_file_content))
        .route("/files/*path", get(get_file_versions))
        // Datasets
        .route("/datasets", get(list_datasets).post(create_dataset))
        .route("/datasets/:id", get(get_dataset).put(update_dataset).delete(delete_dataset_handler))
        .route("/datasets/:id/datapoints", get(list_datapoints).post(create_datapoint))
        .route("/datasets/:id/datapoints/:dp_id", get(get_datapoint_handler).delete(delete_datapoint_handler))
        .route("/datasets/:id/export-span", post(export_span_to_dataset))
        .route("/datasets/:id/import", post(import_file))
        .route("/datasets/:id/queue", get(list_queue).post(enqueue_datapoints))
        .route("/queue/:item_id/claim", post(claim_queue_item))
        .route("/queue/:item_id/submit", post(submit_queue_item))
        // Eval runs
        .route("/datasets/:id/eval", get(list_eval_runs).post(create_eval_run))
        .route("/datasets/:id/compare", get(compare_eval_runs))
        .route("/eval/:run_id", get(get_eval_run_handler).delete(delete_eval_run_handler))
        .route("/eval/:run_id/cancel", post(cancel_eval_run))
        // Capture rules
        .route("/datasets/:id/rules", get(list_capture_rules).post(create_capture_rule))
        .route("/rules/:rule_id", axum::routing::put(update_capture_rule).delete(delete_capture_rule_handler))
        .route("/rules/:rule_id/toggle", post(toggle_capture_rule))
        // Provider connections
        .route("/provider-connections", get(list_provider_connections).post(create_provider_connection))
        .route("/provider-connections/test", post(test_provider_connection))
        .route("/provider-connections/:conn_id", axum::routing::put(update_provider_connection).delete(delete_provider_connection_handler))
        .route("/provider-connections/:conn_id/models", get(list_provider_connection_models))
        // Analytics
        .route("/analytics", post(post_analytics))
        .route("/analytics/summary", get(analytics_summary))
        // Stats & Export
        .route("/stats", get(get_stats))
        .route("/export/json", get(export_json))
        // Config & Shutdown
        .route("/config", get(get_config).put(update_config))
        .route("/shutdown", post(post_shutdown))
        // SSE
        .route("/events", get(events))
        // Auth routes that require auth (me, org, api-keys)
        .merge(auth_routes::protected_auth_router())
        // Billing routes that require auth (checkout)
        .merge(billing_routes::billing_protected_router())
        .layer(AuthLayer { state: auth_mw_state });

    // Public routes (no auth required)
    let public = Router::new()
        // Health & Observability
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/live", get(live))
        .route("/metrics", get(prometheus_metrics))
        // OpenAPI spec
        .route("/openapi.json", get(openapi_spec))
        // Auth routes that don't require auth (config, signup, login, logout)
        .merge(auth_routes::public_auth_router())
        // Billing webhooks (Polar.sh)
        .merge(billing_routes::billing_router());

    let api = Router::new()
        .merge(protected)
        .merge(public);

    let app = Router::new()
        .nest("/api", api);

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

/// Background retention cleanup task.
/// Runs every hour (or until shutdown), deleting spans older than the org's retention window.
/// In per-org (cloud) mode, iterates all cached org stores and checks each org's plan.
/// Should be spawned as `tokio::spawn(run_retention_cleanup(...))`.
pub async fn run_retention_cleanup(
    org_stores: Arc<OrgStoreManager>,
    auth_store: Arc<dyn auth::AuthStore>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    use chrono::{Duration, Utc};
    use std::time::Duration as StdDuration;

    let interval = StdDuration::from_secs(60 * 60); // 1 hour

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown_rx.changed() => {
                tracing::info!("retention cleanup: shutting down");
                return;
            }
        }

        tracing::debug!("retention cleanup: starting sweep");

        let cached = org_stores.cached_stores().await;
        for (org_id, store) in cached {
            let retention_days = match auth_store.get_org(org_id).await {
                Ok(Some(org)) => org.plan.retention_days(),
                _ => {
                    tracing::warn!(%org_id, "retention cleanup: could not fetch org, skipping");
                    continue;
                }
            };

            let cutoff = Utc::now() - Duration::days(retention_days as i64);
            let mut w = store.write().await;
            let deleted = w.delete_spans_before(cutoff).await;
            drop(w);
            if deleted > 0 {
                tracing::info!(%org_id, deleted, retention_days, "retention cleanup: cleaned org");
            }
        }

        tracing::debug!("retention cleanup: sweep complete");
    }
}

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
    let app = build_router(org_stores, start_time, config, config_path, shutdown_tx, auth::AuthConfig::local(), None, None, None, None, None, None);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("api listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
