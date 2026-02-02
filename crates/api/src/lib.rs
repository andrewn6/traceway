use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tokio::sync::RwLock;

use storage::SpanStore;
use trace::{Span, SpanId, TraceId};

pub type SharedStore = Arc<RwLock<SpanStore>>;

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

pub fn router(store: SharedStore) -> Router {
    Router::new()
        .route("/traces", get(list_traces))
        .route("/traces/{trace_id}", get(get_trace))
        .route("/spans/{span_id}", get(get_span))
        .route("/stats", get(get_stats))
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
