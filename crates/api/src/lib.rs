use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tower_http::cors::{Any, CorsLayer};

use storage::{FileFilter, PersistentStore, SpanFilter, SqliteBackend};
use trace::{
    FileVersion, Span, SpanBuilder, SpanId, SpanKind, Trace, TraceId,
};

// --- Events ---

#[derive(Debug, Clone, Serialize)]
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
    Cleared,
}

// --- App State ---

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<PersistentStore<SqliteBackend>>>,
    pub events_tx: broadcast::Sender<SystemEvent>,
}

pub type SharedStore = Arc<RwLock<PersistentStore<SqliteBackend>>>;

// --- Request types ---

#[derive(Deserialize)]
struct CreateSpanRequest {
    trace_id: TraceId,
    #[serde(default)]
    parent_id: Option<SpanId>,
    name: String,
    kind: SpanKind,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct CompleteSpanRequest {
    #[serde(default)]
    output: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct FailSpanRequest {
    error: String,
}

#[derive(Deserialize)]
struct SpanQueryParams {
    kind: Option<String>,
    model: Option<String>,
    status: Option<String>,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    name_contains: Option<String>,
    path: Option<String>,
    trace_id: Option<TraceId>,
}

#[derive(Deserialize)]
struct CreateTraceRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct FileQueryParams {
    path_prefix: Option<String>,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct ExportParams {
    trace_id: Option<TraceId>,
}

// --- Response types ---

#[derive(Serialize)]
struct CreatedSpan {
    id: SpanId,
    trace_id: TraceId,
}

#[derive(Serialize)]
struct TraceListResponse {
    traces: Vec<Trace>,
    count: usize,
}

#[derive(Serialize)]
struct SpanList {
    spans: Vec<Span>,
    count: usize,
}

#[derive(Serialize)]
struct Stats {
    trace_count: usize,
    span_count: usize,
}

#[derive(Serialize)]
struct DeletedTrace {
    trace_id: TraceId,
    spans_deleted: usize,
}

#[derive(Serialize)]
struct ClearedAll {
    message: String,
}

#[derive(Serialize)]
struct ExportData {
    traces: HashMap<TraceId, Vec<Span>>,
}

#[derive(Serialize)]
struct FileListResponse {
    files: Vec<FileVersion>,
    count: usize,
}

#[derive(Serialize)]
struct FileVersionsResponse {
    path: String,
    versions: Vec<FileVersion>,
    count: usize,
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
        status: params.status,
        since: params.since,
        until: params.until,
        name_contains: params.name_contains,
        path: params.path,
        trace_id: params.trace_id,
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

// --- Router ---

pub fn router(store: SharedStore) -> Router {
    let (events_tx, _) = broadcast::channel(256);
    let state = AppState { store, events_tx };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
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
        .route("/files/*path", get(get_file_versions))
        // Stats & Export
        .route("/stats", get(get_stats))
        .route("/export/json", get(export_json))
        // SSE
        .route("/events", get(events))
        .layer(cors)
        .with_state(state)
}

pub async fn serve(store: SharedStore, addr: &str) -> std::io::Result<()> {
    let app = router(store);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("api listening on {}", addr);
    axum::serve(listener, app)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
