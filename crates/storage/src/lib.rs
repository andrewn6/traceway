pub mod backend;
pub mod sqlite;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use trace::{FileVersion, Span, SpanId, Trace, TraceId};

pub use backend::{StorageBackend, StorageError};
pub use sqlite::SqliteBackend;

// --- Span Filter ---

#[derive(Debug, Default, Clone)]
pub struct SpanFilter {
    pub kind: Option<String>,
    pub model: Option<String>,
    pub status: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub name_contains: Option<String>,
    pub path: Option<String>,
    pub trace_id: Option<TraceId>,
}

// --- File Filter ---

#[derive(Debug, Default, Clone)]
pub struct FileFilter {
    pub path_prefix: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub associated_trace: Option<TraceId>,
}

// --- In-memory span store ---

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
        let id = span.id();
        let trace_id = span.trace_id();
        self.spans.insert(id, span);
        self.traces.entry(trace_id).or_default().push(id);
        id
    }

    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.spans.get(&id)
    }

    pub fn remove(&mut self, id: SpanId) -> Option<Span> {
        self.spans.remove(&id)
    }

    pub fn replace(&mut self, span: Span) {
        let id = span.id();
        self.spans.insert(id, span);
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

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }

    pub fn delete_span(&mut self, id: SpanId) -> bool {
        if let Some(span) = self.spans.remove(&id) {
            if let Some(span_ids) = self.traces.get_mut(&span.trace_id()) {
                span_ids.retain(|&sid| sid != id);
                if span_ids.is_empty() {
                    self.traces.remove(&span.trace_id());
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
                if let Some(ref kind) = filter.kind {
                    if span.kind().kind_name() != kind {
                        return false;
                    }
                }

                if let Some(ref model) = filter.model {
                    match span.kind().model() {
                        Some(m) if m == model => {}
                        _ => return false,
                    }
                }

                if let Some(ref status) = filter.status {
                    if span.status().as_str() != status {
                        return false;
                    }
                }

                if let Some(since) = filter.since {
                    if span.started_at() < since {
                        return false;
                    }
                }

                if let Some(until) = filter.until {
                    if span.started_at() > until {
                        return false;
                    }
                }

                if let Some(ref name_contains) = filter.name_contains {
                    if !span.name().contains(name_contains) {
                        return false;
                    }
                }

                if let Some(ref path) = filter.path {
                    match span.kind().path() {
                        Some(p) if p == path => {}
                        _ => return false,
                    }
                }

                if let Some(trace_id) = filter.trace_id {
                    if span.trace_id() != trace_id {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}

// --- Persistent store ---

pub struct PersistentStore<B: StorageBackend> {
    memory: SpanStore,
    trace_meta: HashMap<TraceId, Trace>,
    file_versions: Vec<FileVersion>,
    backend: B,
}

impl<B: StorageBackend> PersistentStore<B> {
    pub async fn open(backend: B) -> Result<Self, StorageError> {
        let mut memory = SpanStore::new();
        let spans = backend.load_all_spans().await?;
        let span_count = spans.len();
        for span in spans {
            memory.insert(span);
        }
        if span_count > 0 {
            tracing::info!(count = span_count, "loaded spans from storage backend");
        }

        let traces = backend.load_all_traces().await?;
        let mut trace_meta = HashMap::new();
        for trace in traces {
            trace_meta.insert(trace.id, trace);
        }

        let file_versions = backend.load_all_files().await?;

        Ok(Self {
            memory,
            trace_meta,
            file_versions,
            backend,
        })
    }

    // --- Span methods ---

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

    pub fn spans_for_trace(&self, trace_id: TraceId) -> &[SpanId] {
        self.memory.spans_for_trace(trace_id)
    }

    pub fn span_trace_ids(&self) -> impl Iterator<Item = &TraceId> {
        self.memory.trace_ids()
    }

    pub fn all_spans(&self) -> impl Iterator<Item = &Span> {
        self.memory.all_spans()
    }

    pub fn span_count(&self) -> usize {
        self.memory.span_count()
    }

    pub fn trace_count(&self) -> usize {
        if self.trace_meta.is_empty() {
            self.memory.trace_count()
        } else {
            self.trace_meta.len()
        }
    }

    pub fn filter_spans(&self, filter: &SpanFilter) -> Vec<&Span> {
        self.memory.filter_spans(filter)
    }

    /// Complete a span (immutable transition: Running -> Completed)
    pub async fn complete_span(
        &mut self,
        id: SpanId,
        output: Option<serde_json::Value>,
    ) -> Option<Span> {
        let span = self.memory.remove(id)?;
        if span.status().is_terminal() {
            self.memory.replace(span);
            return None;
        }
        let completed = span.complete(output);
        self.memory.replace(completed.clone());
        if let Err(e) = self.backend.save_span(&completed).await {
            tracing::error!(%id, "failed to persist span completion: {}", e);
        }
        Some(completed)
    }

    /// Fail a span (immutable transition: Running -> Failed)
    pub async fn fail_span(&mut self, id: SpanId, error: impl Into<String>) -> Option<Span> {
        let span = self.memory.remove(id)?;
        if span.status().is_terminal() {
            self.memory.replace(span);
            return None;
        }
        let failed = span.fail(error);
        self.memory.replace(failed.clone());
        if let Err(e) = self.backend.save_span(&failed).await {
            tracing::error!(%id, "failed to persist span failure: {}", e);
        }
        Some(failed)
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
        self.trace_meta.remove(&trace_id);
        if count > 0 {
            if let Err(e) = self.backend.delete_trace_spans(trace_id).await {
                tracing::error!(%trace_id, "failed to persist trace span deletion: {}", e);
            }
        }
        let _ = self.backend.delete_trace(trace_id).await;
        count
    }

    pub async fn clear(&mut self) {
        self.memory.clear();
        self.trace_meta.clear();
        self.file_versions.clear();
        if let Err(e) = self.backend.clear_spans().await {
            tracing::error!("failed to persist clear: {}", e);
        }
    }

    // --- Trace methods ---

    pub async fn save_trace(&mut self, trace: Trace) {
        if let Err(e) = self.backend.save_trace(&trace).await {
            tracing::error!("failed to persist trace: {}", e);
        }
        self.trace_meta.insert(trace.id, trace);
    }

    pub fn get_trace(&self, id: TraceId) -> Option<&Trace> {
        self.trace_meta.get(&id)
    }

    pub fn all_traces(&self) -> impl Iterator<Item = &Trace> {
        self.trace_meta.values()
    }

    // --- File methods ---

    pub async fn save_file_version(&mut self, version: FileVersion) {
        if let Err(e) = self.backend.save_file_version(&version).await {
            tracing::error!("failed to persist file version: {}", e);
        }
        self.file_versions.push(version);
    }

    pub async fn save_file_content(&self, hash: &str, content: &[u8]) {
        if let Err(e) = self.backend.save_file_content(hash, content).await {
            tracing::error!("failed to persist file content: {}", e);
        }
    }

    pub async fn load_file_content(&self, hash: &str) -> Result<Vec<u8>, StorageError> {
        self.backend.load_file_content(hash).await
    }

    pub fn list_files(&self, filter: &FileFilter) -> Vec<&FileVersion> {
        self.file_versions
            .iter()
            .filter(|fv| {
                if let Some(ref prefix) = filter.path_prefix {
                    if !fv.path.starts_with(prefix) {
                        return false;
                    }
                }
                if let Some(since) = filter.since {
                    if fv.created_at < since {
                        return false;
                    }
                }
                if let Some(until) = filter.until {
                    if fv.created_at > until {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    pub fn get_file_versions(&self, path: &str) -> Vec<&FileVersion> {
        self.file_versions
            .iter()
            .filter(|fv| fv.path == path)
            .collect()
    }
}
