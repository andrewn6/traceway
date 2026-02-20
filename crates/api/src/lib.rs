pub mod auth_keys;
pub mod auth_routes;
pub mod events;
pub mod jobs;
pub mod metrics;

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
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::Multipart;
use rust_embed::Embed;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use tokio::sync::{broadcast, watch, RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::cors::{Any, CorsLayer};

use storage::{analytics, FileFilter, PersistentStore, SpanFilter};
use storage_sqlite::SqliteBackend;
use trace::{
    AnalyticsQuery, AnalyticsResponse, AnalyticsSummary, Datapoint, DatapointId, DatapointKind,
    DatapointSource, Dataset, DatasetId, FileVersion, Message, QueueItem, QueueItemId,
    QueueItemStatus, Span, SpanBuilder, SpanId, SpanKind, Trace, TraceId,
};

pub use events::{EventBus, EventSubscriber, LocalEventBus};

// --- OpenAPI ---

#[derive(OpenApi)]
#[openapi(
    info(
        title = "llm-fs API",
        version = "0.1.0",
        description = "LLM tracing and observability API"
    ),
    paths(
        // OpenAPI spec endpoint
        openapi_spec,
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
        // Response types
        CreatedSpan,
        TraceListResponse,
        SpanList,
        Stats,
        DeletedTrace,
        ClearedAll,
        ExportData,
        FileListResponse,
        FileVersionsResponse,
        DatasetResponse,
        DatasetListResponse,
        DatapointListResponse,
        ImportResponse,
        QueueListResponse,
        QueueCounts,
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
    Cleared,
}

// --- App State ---

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<PersistentStore<SqliteBackend>>>,
    pub events_tx: broadcast::Sender<SystemEvent>,
    pub start_time: Instant,
    pub config: Arc<RwLock<serde_json::Value>>,
    pub config_path: Arc<String>,
    pub shutdown_tx: Option<watch::Sender<bool>>,
    pub auth_config: auth::AuthConfig,
    pub auth_store: Option<Arc<dyn auth::AuthStore>>,
    pub api_key_lookup: Arc<dyn auth::ApiKeyLookup>,
}

pub type SharedStore = Arc<RwLock<PersistentStore<SqliteBackend>>>;

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

#[derive(Deserialize, ToSchema)]
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
    pub trace_id: Option<TraceId>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateTraceRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct FileQueryParams {
    pub path_prefix: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

#[derive(Deserialize, ToSchema)]
pub struct ExportParams {
    #[schema(value_type = Option<String>)]
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

// --- Response types ---

#[derive(Serialize, ToSchema)]
pub struct CreatedSpan {
    #[schema(value_type = String)]
    pub id: SpanId,
    #[schema(value_type = String)]
    pub trace_id: TraceId,
}

#[derive(Serialize, ToSchema)]
pub struct TraceListResponse {
    pub traces: Vec<Trace>,
    pub count: usize,
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
pub struct DatapointListResponse {
    pub datapoints: Vec<Datapoint>,
    pub count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct ImportResponse {
    pub imported: usize,
    #[schema(value_type = String)]
    pub dataset_id: DatasetId,
}

#[derive(Serialize, ToSchema)]
pub struct QueueListResponse {
    pub items: Vec<QueueItem>,
    pub counts: QueueCounts,
}

#[derive(Serialize, ToSchema)]
pub struct QueueCounts {
    pub pending: usize,
    pub claimed: usize,
    pub completed: usize,
}

#[derive(Serialize, ToSchema)]
pub struct EnqueueResponse {
    pub enqueued: usize,
}

// --- Trace handlers ---

async fn list_traces(State(state): State<AppState>) -> Json<TraceListResponse> {
    let r = state.store.read().await;
    let traces: Vec<Trace> = r.all_traces().cloned().collect();
    let count = traces.len();
    Json(TraceListResponse { traces, count })
}

async fn create_trace(
    State(state): State<AppState>,
    Json(req): Json<CreateTraceRequest>,
) -> (StatusCode, Json<Trace>) {
    let trace = Trace::new(req.name).with_tags(req.tags);
    let mut w = state.store.write().await;
    w.save_trace(trace.clone()).await;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::TraceCreated { trace: trace.clone() });
    (StatusCode::CREATED, Json(trace))
}

async fn get_trace(
    State(state): State<AppState>,
    Path(trace_id): Path<TraceId>,
) -> Result<Json<SpanList>, StatusCode> {
    let r = state.store.read().await;
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

async fn list_spans(
    State(state): State<AppState>,
    Query(params): Query<SpanQueryParams>,
) -> Json<SpanList> {
    let r = state.store.read().await;
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
    };
    let spans: Vec<Span> = r.filter_spans(&filter).into_iter().cloned().collect();
    let count = spans.len();
    Json(SpanList { spans, count })
}

async fn get_span(
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
) -> Result<Json<Span>, StatusCode> {
    let r = state.store.read().await;
    r.get(span_id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_span(
    State(state): State<AppState>,
    Json(req): Json<CreateSpanRequest>,
) -> (StatusCode, Json<CreatedSpan>) {
    let mut builder = SpanBuilder::new(req.trace_id, req.name, req.kind);
    if let Some(parent_id) = req.parent_id {
        builder = builder.parent(parent_id);
    }
    if let Some(input) = req.input {
        builder = builder.input(input);
    }
    let span = builder.build();
    let id = span.id();
    let trace_id = span.trace_id();

    let mut w = state.store.write().await;
    w.insert(span.clone()).await;
    drop(w);

    let _ = state.events_tx.send(SystemEvent::SpanCreated { span });
    tracing::debug!(%id, %trace_id, "span created");
    (StatusCode::CREATED, Json(CreatedSpan { id, trace_id }))
}

async fn complete_span(
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
    body: Option<Json<CompleteSpanRequest>>,
) -> StatusCode {
    let output = body.and_then(|b| b.0.output);

    let mut w = state.store.write().await;

    // Check if already terminal → 409 Conflict
    if let Some(span) = w.get(span_id) {
        if span.status().is_terminal() {
            return StatusCode::CONFLICT;
        }
    } else {
        return StatusCode::NOT_FOUND;
    }

    if let Some(span) = w.complete_span(span_id, output).await {
        drop(w);
        let _ = state.events_tx.send(SystemEvent::SpanCompleted { span });
        tracing::debug!(%span_id, "span completed");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn fail_span(
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
    Json(req): Json<FailSpanRequest>,
) -> StatusCode {
    let mut w = state.store.write().await;

    // Check if already terminal → 409 Conflict
    if let Some(span) = w.get(span_id) {
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

async fn get_stats(State(state): State<AppState>) -> Json<Stats> {
    let r = state.store.read().await;
    Json(Stats {
        trace_count: r.trace_count(),
        span_count: r.span_count(),
    })
}

// --- File handlers ---

async fn list_files(
    State(state): State<AppState>,
    Query(params): Query<FileQueryParams>,
) -> Json<FileListResponse> {
    let r = state.store.read().await;
    let filter = FileFilter {
        path_prefix: params.path_prefix,
        since: params.since,
        until: params.until,
        ..Default::default()
    };
    let files: Vec<FileVersion> = r.list_files(&filter).into_iter().cloned().collect();
    let count = files.len();
    Json(FileListResponse { files, count })
}

async fn get_file_versions(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<FileVersionsResponse>, StatusCode> {
    let r = state.store.read().await;
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

async fn get_file_content(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Response, StatusCode> {
    let r = state.store.read().await;
    let content = r.load_file_content(&hash).await.map_err(|_| StatusCode::NOT_FOUND)?;
    drop(r);

    // Try to guess mime type from the hash's associated file path
    let mime = {
        let r2 = state.store.read().await;
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

async fn export_json(
    State(state): State<AppState>,
    Query(params): Query<ExportParams>,
) -> Json<ExportData> {
    let r = state.store.read().await;
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

    Json(ExportData { traces })
}

// --- SSE handler ---

async fn events(
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

async fn delete_span(
    State(state): State<AppState>,
    Path(span_id): Path<SpanId>,
) -> StatusCode {
    let mut w = state.store.write().await;
    if w.delete_span(span_id).await {
        drop(w);
        let _ = state.events_tx.send(SystemEvent::SpanDeleted { span_id });
        tracing::debug!(%span_id, "span deleted");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn delete_trace(
    State(state): State<AppState>,
    Path(trace_id): Path<TraceId>,
) -> Result<Json<DeletedTrace>, StatusCode> {
    let mut w = state.store.write().await;
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

async fn clear_all_traces(State(state): State<AppState>) -> Json<ClearedAll> {
    let mut w = state.store.write().await;
    w.clear().await;
    drop(w);
    let _ = state.events_tx.send(SystemEvent::Cleared);
    tracing::debug!("all traces cleared");
    Json(ClearedAll {
        message: "All traces cleared".to_string(),
    })
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

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let r = state.store.read().await;

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
            backend: "sqlite".to_string(), // TODO: Get from store
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

async fn prometheus_metrics(State(state): State<AppState>) -> Response {
    let r = state.store.read().await;
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

async fn list_datasets(State(state): State<AppState>) -> Json<DatasetListResponse> {
    let r = state.store.read().await;
    let datasets: Vec<DatasetResponse> = r
        .all_datasets()
        .map(|ds| DatasetResponse {
            datapoint_count: r.datapoint_count_for_dataset(ds.id),
            dataset: ds.clone(),
        })
        .collect();
    let count = datasets.len();
    Json(DatasetListResponse { datasets, count })
}

async fn create_dataset(
    State(state): State<AppState>,
    Json(req): Json<CreateDatasetRequest>,
) -> (StatusCode, Json<Dataset>) {
    let dataset = Dataset::new(req.name, req.description);
    let mut w = state.store.write().await;
    w.save_dataset(dataset.clone()).await;
    drop(w);
    let _ = state
        .events_tx
        .send(SystemEvent::DatasetCreated { dataset: dataset.clone() });
    (StatusCode::CREATED, Json(dataset))
}

async fn get_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
) -> Result<Json<DatasetResponse>, StatusCode> {
    let r = state.store.read().await;
    let ds = r.get_dataset(dataset_id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(DatasetResponse {
        datapoint_count: r.datapoint_count_for_dataset(ds.id),
        dataset: ds.clone(),
    }))
}

async fn update_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<UpdateDatasetRequest>,
) -> Result<Json<Dataset>, StatusCode> {
    let mut w = state.store.write().await;
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
    Ok(Json(updated))
}

async fn delete_dataset_handler(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
) -> StatusCode {
    let mut w = state.store.write().await;
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

async fn list_datapoints(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
) -> Result<Json<DatapointListResponse>, StatusCode> {
    let r = state.store.read().await;
    if r.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let datapoints: Vec<Datapoint> = r
        .datapoints_for_dataset(dataset_id)
        .into_iter()
        .cloned()
        .collect();
    let count = datapoints.len();
    Ok(Json(DatapointListResponse { datapoints, count }))
}

async fn create_datapoint(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<CreateDatapointRequest>,
) -> Result<(StatusCode, Json<Datapoint>), StatusCode> {
    let mut w = state.store.write().await;
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

async fn delete_datapoint_handler(
    State(state): State<AppState>,
    Path((dataset_id, dp_id)): Path<(DatasetId, DatapointId)>,
) -> StatusCode {
    let mut w = state.store.write().await;
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

async fn export_span_to_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<ExportSpanRequest>,
) -> Result<(StatusCode, Json<Datapoint>), StatusCode> {
    let mut w = state.store.write().await;
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

async fn import_file(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImportResponse>), (StatusCode, String)> {
    // Verify dataset exists
    {
        let r = state.store.read().await;
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

        let mut w = state.store.write().await;
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

async fn list_queue(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
) -> Result<Json<QueueListResponse>, StatusCode> {
    let r = state.store.read().await;
    if r.get_dataset(dataset_id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let items: Vec<QueueItem> = r
        .queue_items_for_dataset(dataset_id)
        .into_iter()
        .cloned()
        .collect();
    let counts = QueueCounts {
        pending: items
            .iter()
            .filter(|i| i.status == QueueItemStatus::Pending)
            .count(),
        claimed: items
            .iter()
            .filter(|i| i.status == QueueItemStatus::Claimed)
            .count(),
        completed: items
            .iter()
            .filter(|i| i.status == QueueItemStatus::Completed)
            .count(),
    };
    Ok(Json(QueueListResponse { items, counts }))
}

async fn enqueue_datapoints(
    State(state): State<AppState>,
    Path(dataset_id): Path<DatasetId>,
    Json(req): Json<EnqueueRequest>,
) -> Result<(StatusCode, Json<EnqueueResponse>), StatusCode> {
    let mut w = state.store.write().await;
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

async fn claim_queue_item(
    State(state): State<AppState>,
    Path(item_id): Path<QueueItemId>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<QueueItem>, StatusCode> {
    let mut w = state.store.write().await;
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

async fn submit_queue_item(
    State(state): State<AppState>,
    Path(item_id): Path<QueueItemId>,
    Json(req): Json<SubmitRequest>,
) -> Result<Json<QueueItem>, StatusCode> {
    let mut w = state.store.write().await;

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

async fn post_analytics(
    State(state): State<AppState>,
    Json(query): Json<AnalyticsQuery>,
) -> Json<AnalyticsResponse> {
    let r = state.store.read().await;
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
    Json(response)
}

async fn analytics_summary(State(state): State<AppState>) -> Json<AnalyticsSummary> {
    let r = state.store.read().await;
    let spans: Vec<&trace::Span> = r.all_spans().collect();
    let trace_count = r.trace_count();
    let summary = analytics::compute_summary(&spans, trace_count);
    Json(summary)
}

// --- Config handlers ---

async fn get_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.read().await;
    Json(config.clone())
}

async fn update_config(
    State(state): State<AppState>,
    Json(new_config): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
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

async fn post_shutdown(State(state): State<AppState>) -> StatusCode {
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

// --- Auth Middleware ---

/// Auth middleware shared state.
#[derive(Clone)]
struct AuthMiddlewareState {
    config: auth::AuthConfig,
    lookup: Arc<dyn auth::ApiKeyLookup>,
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

            match extract_auth(
                auth_header.as_deref(),
                cookie_header.as_deref(),
                query_string.as_deref(),
                &state.config,
                state.lookup.as_ref(),
            ).await {
                Ok(ctx) => {
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
            // API key format: llmfs_sk_...
            if token.starts_with("llmfs_sk_") {
                let prefix = if token.len() >= 16 { &token[..16] } else {
                    return Err(auth::AuthError::InvalidApiKey);
                };
                let (org_id, key_hash, scopes) = lookup
                    .lookup_api_key(prefix)
                    .await
                    .ok_or(auth::AuthError::InvalidApiKey)?;
                if !auth::verify_api_key(token, &key_hash) {
                    return Err(auth::AuthError::InvalidApiKey);
                }
                return Ok(auth::AuthContext::from_api_key(org_id, scopes));
            }
            // JWT session token
            let session = auth::verify_session(token, &config.jwt_secret)?;
            return Ok(auth::AuthContext::from_session(session.org_id, session.user_id, session.scopes));
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
                    return Ok(auth::AuthContext::from_session(session.org_id, session.user_id, session.scopes));
                }
            }
        }
    }

    // Check query param (for SSE)
    if let Some(query) = query_string {
        if let Some(token) = auth_keys::extract_token_from_query(query) {
            let session = auth::verify_session(&token, &config.jwt_secret)?;
            return Ok(auth::AuthContext::from_session(session.org_id, session.user_id, session.scopes));
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
    store: SharedStore,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
    auth_config: auth::AuthConfig,
    auth_store: Option<Arc<dyn auth::AuthStore>>,
    api_key_lookup: Option<Arc<dyn auth::ApiKeyLookup>>,
}

impl RouterBuilder {
    pub fn new(store: SharedStore) -> Self {
        Self {
            store,
            start_time: Instant::now(),
            config: serde_json::Value::Object(Default::default()),
            config_path: String::new(),
            shutdown_tx: None,
            auth_config: auth::AuthConfig::local(),
            auth_store: None,
            api_key_lookup: None,
        }
    }

    pub fn start_time(mut self, t: Instant) -> Self { self.start_time = t; self }
    pub fn config(mut self, c: serde_json::Value) -> Self { self.config = c; self }
    pub fn config_path(mut self, p: String) -> Self { self.config_path = p; self }
    pub fn shutdown_tx(mut self, tx: watch::Sender<bool>) -> Self { self.shutdown_tx = Some(tx); self }
    pub fn auth_config(mut self, c: auth::AuthConfig) -> Self { self.auth_config = c; self }
    pub fn auth_store(mut self, s: Arc<dyn auth::AuthStore>) -> Self { self.auth_store = Some(s); self }
    pub fn api_key_lookup(mut self, l: Arc<dyn auth::ApiKeyLookup>) -> Self { self.api_key_lookup = Some(l); self }

    pub fn build(self) -> Router {
        build_router(
            self.store,
            self.start_time,
            self.config,
            self.config_path,
            self.shutdown_tx,
            self.auth_config,
            self.auth_store,
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
    build_router(store, start_time, config, config_path, shutdown_tx, auth::AuthConfig::local(), None, None)
}

fn build_router(
    store: SharedStore,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
    auth_config: auth::AuthConfig,
    auth_store: Option<Arc<dyn auth::AuthStore>>,
    api_key_lookup: Option<Arc<dyn auth::ApiKeyLookup>>,
) -> Router {
    let (events_tx, _) = broadcast::channel(256);
    let api_key_lookup = api_key_lookup.unwrap_or_else(|| {
        Arc::new(auth_keys::NoopApiKeyLookup) as Arc<dyn auth::ApiKeyLookup>
    });
    let state = AppState {
        store,
        events_tx,
        start_time,
        config: Arc::new(RwLock::new(config)),
        config_path: Arc::new(config_path),
        shutdown_tx,
        auth_config: auth_config.clone(),
        auth_store,
        api_key_lookup: api_key_lookup.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Auth middleware state (injected as Extension for `from_fn`)
    let auth_mw_state = AuthMiddlewareState {
        config: state.auth_config.clone(),
        lookup: state.api_key_lookup.clone(),
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
        .route("/datasets/:id/datapoints/:dp_id", delete(delete_datapoint_handler))
        .route("/datasets/:id/export-span", post(export_span_to_dataset))
        .route("/datasets/:id/import", post(import_file))
        .route("/datasets/:id/queue", get(list_queue).post(enqueue_datapoints))
        .route("/queue/:item_id/claim", post(claim_queue_item))
        .route("/queue/:item_id/submit", post(submit_queue_item))
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
        .merge(auth_routes::public_auth_router());

    let api = Router::new()
        .merge(protected)
        .merge(public);

    Router::new()
        .nest("/api", api)
        // Embedded UI (SPA fallback)
        .fallback(serve_ui)
        .layer(cors)
        .with_state(state)
}

pub async fn serve(store: SharedStore, addr: &str) -> std::io::Result<()> {
    serve_with_shutdown(store, addr, Instant::now(), serde_json::Value::Object(Default::default()), String::new(), None, std::future::pending()).await
}

pub async fn serve_with_shutdown(
    store: SharedStore,
    addr: &str,
    start_time: Instant,
    config: serde_json::Value,
    config_path: String,
    shutdown_tx: Option<watch::Sender<bool>>,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let app = router_with_start_time(store, start_time, config, config_path, shutdown_tx);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("api listening on {}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
