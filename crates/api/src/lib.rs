use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use storage::{SpanFilter, SpanStore};
use trace::{Span, SpanId, SpanMetadata, TraceId};

pub type SharedStore = Arc<RwLock<SpanStore>>;

// Request types

#[derive(Deserialize)]
struct CreateSpan {
    trace_id: TraceId,
    parent_id: Option<SpanId>,
    name: String,
    #[serde(default)]
    metadata: SpanMetadata,
}

#[derive(Deserialize)]
struct FailSpan {
    error: String,
}

#[derive(Deserialize)]
struct UpdateMetadata {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct SpanQueryParams {
    model: Option<String>,
    status: Option<String>,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    name_contains: Option<String>,
}

// Response types

#[derive(Serialize)]
struct CreatedSpan {
    id: SpanId,
    trace_id: TraceId,
}

#[derive(Serialize)]
struct TraceList {
    traces: Vec<TraceId>,
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

async fn list_traces(State(store): State<SharedStore>) -> Json<TraceList> {
    let r = store.read().await;
    let traces: Vec<TraceId> = r.trace_ids().copied().collect();
    let count = traces.len();
    Json(TraceList { traces, count })
}

async fn get_trace(
    State(store): State<SharedStore>,
    Path(trace_id): Path<TraceId>,
) -> Result<Json<SpanList>, StatusCode> {
    let r = store.read().await;
    let span_ids = r.spans_for_trace(trace_id);
    if span_ids.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    let spans: Vec<Span> = span_ids
        .iter()
        .filter_map(|id| r.get(*id).cloned())
        .collect();
    let count = spans.len();
    Ok(Json(SpanList { spans, count }))
}

async fn get_span(
    State(store): State<SharedStore>,
    Path(span_id): Path<SpanId>,
) -> Result<Json<Span>, StatusCode> {
    let r = store.read().await;
    r.get(span_id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn get_stats(State(store): State<SharedStore>) -> Json<Stats> {
    let r = store.read().await;
    Json(Stats {
        trace_count: r.trace_count(),
        span_count: r.span_count(),
    })
}

// POST handlers

async fn create_span(
    State(store): State<SharedStore>,
    Json(req): Json<CreateSpan>,
) -> (StatusCode, Json<CreatedSpan>) {
    let mut span = Span::new(req.trace_id, req.parent_id, req.name);
    span.metadata = req.metadata;
    let id = span.id;
    let trace_id = span.trace_id;

    let mut w = store.write().await;
    w.insert(span);

    tracing::debug!(%id, %trace_id, "span created");
    (StatusCode::CREATED, Json(CreatedSpan { id, trace_id }))
}

async fn complete_span(
    State(store): State<SharedStore>,
    Path(span_id): Path<SpanId>,
) -> StatusCode {
    let mut w = store.write().await;
    if w.complete(span_id) {
        tracing::debug!(%span_id, "span completed");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn fail_span(
    State(store): State<SharedStore>,
    Path(span_id): Path<SpanId>,
    Json(req): Json<FailSpan>,
) -> StatusCode {
    let mut w = store.write().await;
    if w.fail(span_id, req.error) {
        tracing::debug!(%span_id, "span failed");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn update_span_metadata(
    State(store): State<SharedStore>,
    Path(span_id): Path<SpanId>,
    Json(req): Json<UpdateMetadata>,
) -> StatusCode {
    let mut w = store.write().await;
    if let Some(span) = w.get_mut(span_id) {
        if let Some(model) = req.model {
            span.metadata.model = Some(model);
        }
        if let Some(input) = req.input_tokens {
            span.metadata.input_tokens = Some(input);
        }
        if let Some(output) = req.output_tokens {
            span.metadata.output_tokens = Some(output);
        }
        tracing::debug!(%span_id, "span metadata updated");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn list_spans(
    State(store): State<SharedStore>,
    Query(params): Query<SpanQueryParams>,
) -> Json<SpanList> {
    let r = store.read().await;
    let filter = SpanFilter {
        model: params.model,
        status: params.status,
        since: params.since,
        until: params.until,
        name_contains: params.name_contains,
    };
    let spans: Vec<Span> = r.filter_spans(&filter).into_iter().cloned().collect();
    let count = spans.len();
    Json(SpanList { spans, count })
}

async fn delete_span(
    State(store): State<SharedStore>,
    Path(span_id): Path<SpanId>,
) -> StatusCode {
    let mut w = store.write().await;
    if w.delete_span(span_id) {
        tracing::debug!(%span_id, "span deleted");
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn delete_trace(
    State(store): State<SharedStore>,
    Path(trace_id): Path<TraceId>,
) -> Result<Json<DeletedTrace>, StatusCode> {
    let mut w = store.write().await;
    let spans_deleted = w.delete_trace(trace_id);
    if spans_deleted > 0 {
        tracing::debug!(%trace_id, %spans_deleted, "trace deleted");
        Ok(Json(DeletedTrace {
            trace_id,
            spans_deleted,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn clear_all_traces(State(store): State<SharedStore>) -> Json<ClearedAll> {
    let mut w = store.write().await;
    w.clear();
    tracing::debug!("all traces cleared");
    Json(ClearedAll {
        message: "All traces cleared".to_string(),
    })
}

pub fn router(store: SharedStore) -> Router {
    Router::new()
        // Read
        .route("/traces", get(list_traces))
        .route("/traces/{trace_id}", get(get_trace))
        .route("/spans", get(list_spans))
        .route("/spans/{span_id}", get(get_span))
        .route("/stats", get(get_stats))
        // Write
        .route("/spans", post(create_span))
        .route("/spans/{span_id}/complete", post(complete_span))
        .route("/spans/{span_id}/fail", post(fail_span))
        .route("/spans/{span_id}/metadata", post(update_span_metadata))
        // Delete
        .route("/spans/{span_id}", delete(delete_span))
        .route("/traces/{trace_id}", delete(delete_trace))
        .route("/traces", delete(clear_all_traces))
        .with_state(store)
}

pub async fn serve(store: SharedStore, addr: &str) -> std::io::Result<()> {
    let app = router(store);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("api listening on {}", addr);
    axum::serve(listener, app)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
