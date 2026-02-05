pub mod backend;
pub mod sqlite;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use trace::{Span, SpanId, SpanStatus, TraceId};

pub use backend::{StorageBackend, StorageError};
pub use sqlite::SqliteBackend;

/// Filter criteria for querying spans.
#[derive(Debug, Default, Clone)]
pub struct SpanFilter {
    pub model: Option<String>,
    pub status: Option<String>, // "running", "completed", "failed"
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub name_contains: Option<String>,
}

/// In-memory span store with dual indexes for fast lookup.
#[derive(Debug, Default)]
pub struct SpanStore {
    spans: HashMap<SpanId, Span>,
    traces: HashMap<TraceId, Vec<SpanId>>,
}

impl SpanStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, span: Span) -> SpanId {
        let id = span.id;
        let trace_id = span.trace_id;
        self.spans.insert(id, span);
        self.traces.entry(trace_id).or_default().push(id);
        id
    }

    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.spans.get(&id)
    }

    pub fn get_mut(&mut self, id: SpanId) -> Option<&mut Span> {
        self.spans.get_mut(&id)
    }

    pub fn spans_for_trace(&self, trace_id: TraceId) -> &[SpanId] {
        self.traces
            .get(&trace_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn trace_ids(&self) -> impl Iterator<Item = &TraceId> {
        self.traces.keys()
    }

    pub fn all_spans(&self) -> impl Iterator<Item = &Span> {
        self.spans.values()
    }

    pub fn complete(&mut self, id: SpanId) -> bool {
        if let Some(span) = self.spans.get_mut(&id) {
            span.complete();
            true
        } else {
            false
        }
    }

    pub fn fail(&mut self, id: SpanId, error: impl Into<String>) -> bool {
        if let Some(span) = self.spans.get_mut(&id) {
            span.fail(error);
            true
        } else {
            false
        }
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }

    pub fn delete_span(&mut self, id: SpanId) -> bool {
        if let Some(span) = self.spans.remove(&id) {
            if let Some(span_ids) = self.traces.get_mut(&span.trace_id) {
                span_ids.retain(|&sid| sid != id);
                if span_ids.is_empty() {
                    self.traces.remove(&span.trace_id);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn delete_trace(&mut self, trace_id: TraceId) -> usize {
        if let Some(span_ids) = self.traces.remove(&trace_id) {
            let count = span_ids.len();
            for id in span_ids {
                self.spans.remove(&id);
            }
            count
        } else {
            0
        }
    }

    pub fn clear(&mut self) {
        self.spans.clear();
        self.traces.clear();
    }

    pub fn filter_spans(&self, filter: &SpanFilter) -> Vec<&Span> {
        self.spans
            .values()
            .filter(|span| {
                if let Some(ref model) = filter.model {
                    match &span.metadata.model {
                        Some(m) if m == model => {}
                        _ => return false,
                    }
                }

                if let Some(ref status) = filter.status {
                    let span_status = match &span.status {
                        SpanStatus::Running { .. } => "running",
                        SpanStatus::Completed { .. } => "completed",
                        SpanStatus::Failed { .. } => "failed",
                    };
                    if span_status != status {
                        return false;
                    }
                }

                let started_at = match &span.status {
                    SpanStatus::Running { started_at } => *started_at,
                    SpanStatus::Completed { started_at, .. } => *started_at,
                    SpanStatus::Failed { started_at, .. } => *started_at,
                };

                if let Some(since) = filter.since {
                    if started_at < since {
                        return false;
                    }
                }

                if let Some(until) = filter.until {
                    if started_at > until {
                        return false;
                    }
                }

                if let Some(ref name_contains) = filter.name_contains {
                    if !span.name.contains(name_contains) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}

/// Persistent span store: in-memory SpanStore backed by a StorageBackend.
///
/// All reads hit memory. All writes go through to the backend.
pub struct PersistentStore<B: StorageBackend> {
    memory: SpanStore,
    backend: B,
}

impl<B: StorageBackend> PersistentStore<B> {
    /// Open a persistent store, loading existing data from the backend.
    pub async fn open(backend: B) -> Result<Self, StorageError> {
        let mut memory = SpanStore::new();
        let spans = backend.load_all().await?;
        let count = spans.len();
        for span in spans {
            memory.insert(span);
        }
        if count > 0 {
            tracing::info!(count, "loaded spans from storage backend");
        }
        Ok(Self { memory, backend })
    }

    pub async fn insert(&mut self, span: Span) -> SpanId {
        let id = self.memory.insert(span);
        if let Some(span) = self.memory.get(id) {
            if let Err(e) = self.backend.save_span(span).await {
                tracing::error!(%id, "failed to persist span insert: {}", e);
            }
        }
        id
    }

    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.memory.get(id)
    }

    pub fn get_mut(&mut self, id: SpanId) -> Option<&mut Span> {
        self.memory.get_mut(id)
    }

    pub fn spans_for_trace(&self, trace_id: TraceId) -> &[SpanId] {
        self.memory.spans_for_trace(trace_id)
    }

    pub fn trace_ids(&self) -> impl Iterator<Item = &TraceId> {
        self.memory.trace_ids()
    }

    pub fn all_spans(&self) -> impl Iterator<Item = &Span> {
        self.memory.all_spans()
    }

    pub fn span_count(&self) -> usize {
        self.memory.span_count()
    }

    pub fn trace_count(&self) -> usize {
        self.memory.trace_count()
    }

    pub fn filter_spans(&self, filter: &SpanFilter) -> Vec<&Span> {
        self.memory.filter_spans(filter)
    }

    /// Persist the current state of a span (e.g. after metadata update).
    pub async fn save(&self, id: SpanId) {
        if let Some(span) = self.memory.get(id) {
            if let Err(e) = self.backend.save_span(span).await {
                tracing::error!(%id, "failed to persist span: {}", e);
            }
        }
    }

    pub async fn complete(&mut self, id: SpanId) -> bool {
        if self.memory.complete(id) {
            self.save(id).await;
            true
        } else {
            false
        }
    }

    pub async fn fail(&mut self, id: SpanId, error: impl Into<String>) -> bool {
        if self.memory.fail(id, error) {
            self.save(id).await;
            true
        } else {
            false
        }
    }

    pub async fn delete_span(&mut self, id: SpanId) -> bool {
        if self.memory.delete_span(id) {
            if let Err(e) = self.backend.delete_span(id).await {
                tracing::error!(%id, "failed to persist span deletion: {}", e);
            }
            true
        } else {
            false
        }
    }

    pub async fn delete_trace(&mut self, trace_id: TraceId) -> usize {
        let count = self.memory.delete_trace(trace_id);
        if count > 0 {
            if let Err(e) = self.backend.delete_trace(trace_id).await {
                tracing::error!(%trace_id, "failed to persist trace deletion: {}", e);
            }
        }
        count
    }

    pub async fn clear(&mut self) {
        self.memory.clear();
        if let Err(e) = self.backend.clear().await {
            tracing::error!("failed to persist clear: {}", e);
        }
    }
}
