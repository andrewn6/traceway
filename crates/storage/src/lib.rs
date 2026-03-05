pub mod analytics;
pub mod backend;
pub mod error;
pub mod filter;

use std::collections::HashMap;


use trace::{
    CaptureRule, CaptureRuleId, Datapoint, DatapointId, Dataset, DatasetId, EvalResult,
    EvalResultId, EvalRun, EvalRunId, FileVersion, ProviderConnection, ProviderConnectionId,
    QueueItem, QueueItemId, QueueItemStatus, Span, SpanId, SpanKind, Trace, TraceId,
};

pub use backend::StorageBackend;
pub use error::StorageError;
pub use filter::{
    CursorInner, DatapointFilter, FileFilter, Page, Pagination, SortOrder, SpanFilter,
    TraceFilter, decode_cursor, encode_cursor,
};

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
        let mut results: Vec<&Span> = self.spans
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

                if let Some(ref provider) = filter.provider {
                    match span.kind().provider() {
                        Some(p) if p == provider => {}
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

                if let Some(min_ms) = filter.duration_min {
                    match span.duration_ms() {
                        Some(d) if d >= min_ms => {}
                        Some(_) => return false,
                        None => return false, // running spans have no duration
                    }
                }

                if let Some(max_ms) = filter.duration_max {
                    match span.duration_ms() {
                        Some(d) if d <= max_ms => {}
                        Some(_) => return false,
                        None => return false,
                    }
                }

                if let Some(min_tokens) = filter.tokens_min {
                    match span.kind().total_tokens() {
                        Some(t) if t >= min_tokens => {}
                        _ => return false,
                    }
                }

                if let Some(min_cost) = filter.cost_min {
                    match span.kind().cost() {
                        Some(c) if c >= min_cost => {}
                        _ => return false,
                    }
                }

                // Full-text search: case-insensitive contains on serialized input/output
                if let Some(ref text) = filter.text_contains {
                    let needle = text.to_lowercase();
                    let in_input = span.input()
                        .map(|v| serde_json::to_string(v).unwrap_or_default().to_lowercase().contains(&needle))
                        .unwrap_or(false);
                    let in_output = span.output()
                        .map(|v| serde_json::to_string(v).unwrap_or_default().to_lowercase().contains(&needle))
                        .unwrap_or(false);
                    let in_name = span.name().to_lowercase().contains(&needle);
                    if !in_input && !in_output && !in_name {
                        return false;
                    }
                }

                if let Some(ref text) = filter.input_contains {
                    let needle = text.to_lowercase();
                    let found = span.input()
                        .map(|v| serde_json::to_string(v).unwrap_or_default().to_lowercase().contains(&needle))
                        .unwrap_or(false);
                    if !found { return false; }
                }

                if let Some(ref text) = filter.output_contains {
                    let needle = text.to_lowercase();
                    let found = span.output()
                        .map(|v| serde_json::to_string(v).unwrap_or_default().to_lowercase().contains(&needle))
                        .unwrap_or(false);
                    if !found { return false; }
                }

                true
            })
            .collect();

        // Apply sorting
        if let Some(ref sort_by) = filter.sort_by {
            let desc = filter.sort_order.as_deref() != Some("asc");
            match sort_by.as_str() {
                "started_at" => {
                    results.sort_by(|a, b| {
                        let cmp = a.started_at().cmp(&b.started_at());
                        if desc { cmp.reverse() } else { cmp }
                    });
                }
                "duration" => {
                    results.sort_by(|a, b| {
                        let cmp = a.duration_ms().unwrap_or(0).cmp(&b.duration_ms().unwrap_or(0));
                        if desc { cmp.reverse() } else { cmp }
                    });
                }
                "tokens" => {
                    results.sort_by(|a, b| {
                        let cmp = a.kind().total_tokens().unwrap_or(0).cmp(&b.kind().total_tokens().unwrap_or(0));
                        if desc { cmp.reverse() } else { cmp }
                    });
                }
                "cost" => {
                    results.sort_by(|a, b| {
                        let ca = a.kind().cost().unwrap_or(0.0);
                        let cb = b.kind().cost().unwrap_or(0.0);
                        let cmp = ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal);
                        if desc { cmp.reverse() } else { cmp }
                    });
                }
                "name" => {
                    results.sort_by(|a, b| {
                        let cmp = a.name().cmp(b.name());
                        if desc { cmp.reverse() } else { cmp }
                    });
                }
                _ => {
                    // Default: newest first
                    results.sort_by(|a, b| b.started_at().cmp(&a.started_at()));
                }
            }
        } else {
            // Default sort: newest first
            results.sort_by(|a, b| b.started_at().cmp(&a.started_at()));
        }

        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        results
    }
}

// --- Persistent store ---

pub struct PersistentStore<B: StorageBackend> {
    memory: SpanStore,
    trace_meta: HashMap<TraceId, Trace>,
    file_versions: Vec<FileVersion>,
    datasets: HashMap<DatasetId, Dataset>,
    datapoints: HashMap<DatapointId, Datapoint>,
    queue_items: HashMap<QueueItemId, QueueItem>,
    eval_runs: HashMap<EvalRunId, EvalRun>,
    eval_results: HashMap<EvalResultId, EvalResult>,
    capture_rules: HashMap<CaptureRuleId, CaptureRule>,
    provider_connections: HashMap<ProviderConnectionId, ProviderConnection>,
    backend: B,
}

impl<B: StorageBackend> PersistentStore<B> {
    pub async fn open(backend: B) -> Result<Self, StorageError> {
        let (
            spans,
            traces_list,
            file_versions,
            ds_list,
            dp_list,
            qi_list,
            er_list,
            eres_list,
            cr_list,
            pc_list,
        ) = tokio::try_join!(
            backend.load_all_spans(),
            backend.load_all_traces(),
            backend.load_all_files(),
            backend.load_all_datasets(),
            backend.load_all_datapoints(),
            backend.load_all_queue_items(),
            backend.load_all_eval_runs(),
            backend.load_all_eval_results(),
            backend.load_all_capture_rules(),
            backend.load_all_provider_connections(),
        )?;

        let mut memory = SpanStore::new();
        let span_count = spans.len();
        for span in spans {
            memory.insert(span);
        }
        if span_count > 0 {
            tracing::info!(count = span_count, "loaded spans from storage backend");
        }

        let trace_meta: HashMap<_, _> = traces_list.into_iter().map(|t| (t.id, t)).collect();
        let datasets: HashMap<_, _> = ds_list.into_iter().map(|d| (d.id, d)).collect();
        let datapoints: HashMap<_, _> = dp_list.into_iter().map(|d| (d.id, d)).collect();
        let queue_items: HashMap<_, _> = qi_list.into_iter().map(|q| (q.id, q)).collect();
        let eval_runs: HashMap<_, _> = er_list.into_iter().map(|r| (r.id, r)).collect();
        let eval_results: HashMap<_, _> = eres_list.into_iter().map(|r| (r.id, r)).collect();
        let capture_rules: HashMap<_, _> = cr_list.into_iter().map(|r| (r.id, r)).collect();
        let provider_connections: HashMap<_, _> = pc_list.into_iter().map(|p| (p.id, p)).collect();

        Ok(Self {
            memory,
            trace_meta,
            file_versions,
            datasets,
            datapoints,
            queue_items,
            eval_runs,
            eval_results,
            capture_rules,
            provider_connections,
            backend,
        })
    }

    /// Get a reference to the underlying backend
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Get the backend type
    pub fn backend_type(&self) -> &'static str {
        self.backend.backend_type()
    }

    // --- Span methods ---

    pub async fn insert(&mut self, span: Span) -> Result<SpanId, StorageError> {
        self.backend.save_span(&span).await?;
        let id = self.memory.insert(span);
        Ok(id)
    }

    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.memory.get(id)
    }

    /// Get a span by ID, falling back to the storage backend if not in memory.
    /// If found in the backend, the span is cached in memory for subsequent access.
    pub async fn get_or_load(&mut self, id: SpanId) -> Option<&Span> {
        if self.memory.get(id).is_some() {
            return self.memory.get(id);
        }
        // Try loading from backend
        match self.backend.get_span(id).await {
            Ok(Some(span)) => {
                tracing::debug!(%id, "loaded span from backend (not in memory)");
                self.memory.insert(span);
                self.memory.get(id)
            }
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(%id, "failed to load span from backend: {}", e);
                None
            }
        }
    }

    pub fn spans_for_trace(&self, trace_id: TraceId) -> &[SpanId] {
        self.memory.spans_for_trace(trace_id)
    }

    /// Get spans for a trace, falling back to the storage backend if none in memory.
    /// If found in the backend, spans are cached in memory for subsequent access.
    pub async fn spans_for_trace_or_load(&mut self, trace_id: TraceId) -> &[SpanId] {
        if !self.memory.spans_for_trace(trace_id).is_empty() {
            return self.memory.spans_for_trace(trace_id);
        }
        // Try loading from backend
        let filter = SpanFilter {
            trace_id: Some(trace_id),
            ..Default::default()
        };
        match self.backend.list_spans(&filter).await {
            Ok(spans) if !spans.is_empty() => {
                tracing::debug!(%trace_id, count = spans.len(), "loaded trace spans from backend");
                for span in spans {
                    self.memory.insert(span);
                }
                self.memory.spans_for_trace(trace_id)
            }
            Ok(_) => &[],
            Err(e) => {
                tracing::warn!(%trace_id, "failed to load trace spans from backend: {}", e);
                &[]
            }
        }
    }

    /// Sync spans and traces from the storage backend into memory.
    /// Merges new data without removing existing in-memory state.
    /// Used to keep multi-instance deployments consistent.
    pub async fn sync_from_backend(&mut self) {
        match self.backend.load_all_spans().await {
            Ok(spans) => {
                let mut loaded = 0;
                for span in spans {
                    if self.memory.get(span.id()).is_none() {
                        self.memory.insert(span);
                        loaded += 1;
                    }
                }
                if loaded > 0 {
                    tracing::debug!(loaded, "synced spans from backend");
                }
            }
            Err(e) => {
                tracing::warn!("failed to sync spans from backend: {}", e);
            }
        }
        match self.backend.load_all_traces().await {
            Ok(traces) => {
                let mut loaded = 0;
                for trace in traces {
                    if !self.trace_meta.contains_key(&trace.id) {
                        self.trace_meta.insert(trace.id, trace);
                        loaded += 1;
                    }
                }
                if loaded > 0 {
                    tracing::debug!(loaded, "synced traces from backend");
                }
            }
            Err(e) => {
                tracing::warn!("failed to sync traces from backend: {}", e);
            }
        }
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

    /// Complete a span (immutable transition: Running -> Completed).
    /// Falls back to the storage backend if the span is not in memory
    /// (e.g. when running multiple instances behind a load balancer).
    pub async fn complete_span(
        &mut self,
        id: SpanId,
        output: Option<serde_json::Value>,
    ) -> Result<Option<Span>, StorageError> {
        // Try memory first, then fall back to backend
        let span = match self.memory.remove(id) {
            Some(s) => s,
            None => {
                match self.backend.get_span(id).await {
                    Ok(Some(s)) => {
                        tracing::debug!(%id, "complete_span: loaded span from backend");
                        s
                    }
                    _ => return Ok(None),
                }
            }
        };
        if span.status().is_terminal() {
            self.memory.replace(span);
            return Ok(None);
        }
        let completed = span.complete(output);
        self.backend.save_span(&completed).await?;
        self.memory.replace(completed.clone());
        Ok(Some(completed))
    }

    /// Complete a span with an updated SpanKind (e.g. to populate token counts).
    /// Uses serde JSON round-trip to reconstruct with new kind, same pattern as SqliteBackend::deserialize_span.
    /// Falls back to the storage backend if the span is not in memory.
    pub async fn complete_span_with_kind(
        &mut self,
        id: SpanId,
        kind: SpanKind,
        output: Option<serde_json::Value>,
    ) -> Result<Option<Span>, StorageError> {
        let span = match self.memory.remove(id) {
            Some(s) => s,
            None => {
                match self.backend.get_span(id).await {
                    Ok(Some(s)) => {
                        tracing::debug!(%id, "complete_span_with_kind: loaded span from backend");
                        s
                    }
                    _ => return Ok(None),
                }
            }
        };
        if span.status().is_terminal() {
            self.memory.replace(span);
            return Ok(None);
        }
        // Serialize the span to JSON, patch in the new kind, then deserialize back
        let completed: Option<Span> = (|| {
            let mut json = serde_json::to_value(&span).ok()?;
            let kind_json = serde_json::to_value(&kind).ok()?;
            let obj = json.as_object_mut()?;
            obj.insert("kind".to_string(), kind_json);
            obj.insert("status".to_string(), serde_json::Value::String("completed".to_string()));
            obj.insert("ended_at".to_string(), serde_json::to_value(chrono::Utc::now()).ok()?);
            if let Some(out) = &output {
                obj.insert("output".to_string(), out.clone());
            }
            serde_json::from_value(json).ok()
        })();
        let Some(completed) = completed else {
            self.memory.replace(span);
            return Ok(None);
        };
        self.backend.save_span(&completed).await?;
        self.memory.replace(completed.clone());
        Ok(Some(completed))
    }

    /// Fail a span (immutable transition: Running -> Failed).
    /// Falls back to the storage backend if the span is not in memory.
    pub async fn fail_span(&mut self, id: SpanId, error: impl Into<String>) -> Result<Option<Span>, StorageError> {
        let span = match self.memory.remove(id) {
            Some(s) => s,
            None => {
                match self.backend.get_span(id).await {
                    Ok(Some(s)) => {
                        tracing::debug!(%id, "fail_span: loaded span from backend");
                        s
                    }
                    _ => return Ok(None),
                }
            }
        };
        if span.status().is_terminal() {
            self.memory.replace(span);
            return Ok(None);
        }
        let failed = span.fail(error);
        self.backend.save_span(&failed).await?;
        self.memory.replace(failed.clone());
        Ok(Some(failed))
    }

    pub async fn delete_span(&mut self, id: SpanId) -> Result<bool, StorageError> {
        // Delete from backend first, then cache
        self.backend.delete_span(id).await?;
        self.memory.delete_span(id);
        Ok(true)
    }

    pub async fn delete_trace(&mut self, trace_id: TraceId) -> Result<usize, StorageError> {
        // Delete from backend first, then cache
        self.backend.delete_trace_spans(trace_id).await?;
        self.backend.delete_trace(trace_id).await?;
        let count = self.memory.delete_trace(trace_id);
        self.trace_meta.remove(&trace_id);
        Ok(count)
    }

    /// Delete all spans started before the given cutoff time.
    /// Returns the number of spans deleted.
    pub async fn delete_spans_before(&mut self, cutoff: chrono::DateTime<chrono::Utc>) -> Result<usize, StorageError> {
        let expired_ids: Vec<SpanId> = self.memory
            .all_spans()
            .filter(|s| s.started_at() < cutoff)
            .map(|s| s.id())
            .collect();

        let count = expired_ids.len();
        for id in &expired_ids {
            self.backend.delete_span(*id).await?;
            self.memory.delete_span(*id);
        }

        // Also clean up traces that now have zero spans
        let empty_traces: Vec<TraceId> = self.trace_meta.keys()
            .filter(|tid| self.memory.spans_for_trace(**tid).is_empty())
            .cloned()
            .collect();
        for tid in empty_traces {
            self.backend.delete_trace(tid).await?;
            self.trace_meta.remove(&tid);
        }

        if count > 0 {
            tracing::info!(count, "retention cleanup: deleted expired spans");
        }
        Ok(count)
    }

    pub async fn clear(&mut self) -> Result<(), StorageError> {
        // Clear backend first, then cache
        self.backend.clear_spans().await?;
        self.memory.clear();
        self.trace_meta.clear();
        self.file_versions.clear();
        self.datasets.clear();
        self.datapoints.clear();
        self.queue_items.clear();
        self.eval_runs.clear();
        self.eval_results.clear();
        self.capture_rules.clear();
        // Note: provider_connections are NOT cleared on "clear all data" — they are org settings, not trace data.
        Ok(())
    }

    // --- Trace methods ---

    pub async fn save_trace(&mut self, trace: Trace) -> Result<(), StorageError> {
        self.backend.save_trace(&trace).await?;
        self.trace_meta.insert(trace.id, trace);
        Ok(())
    }

    pub fn get_trace(&self, id: TraceId) -> Option<&Trace> {
        self.trace_meta.get(&id)
    }

    pub fn all_traces(&self) -> impl Iterator<Item = &Trace> {
        self.trace_meta.values()
    }

    // --- File methods ---

    pub async fn save_file_version(&mut self, version: FileVersion) -> Result<(), StorageError> {
        self.backend.save_file_version(&version).await?;
        self.file_versions.push(version);
        Ok(())
    }

    pub async fn save_file_content(&self, hash: &str, content: &[u8]) -> Result<(), StorageError> {
        self.backend.save_file_content(hash, content).await?;
        Ok(())
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

    // --- Dataset methods ---

    pub async fn save_dataset(&mut self, dataset: Dataset) -> Result<(), StorageError> {
        self.backend.save_dataset(&dataset).await?;
        self.datasets.insert(dataset.id, dataset);
        Ok(())
    }

    pub fn get_dataset(&self, id: DatasetId) -> Option<&Dataset> {
        self.datasets.get(&id)
    }

    /// Get a dataset, falling back to the storage backend if not in memory.
    /// Loads and caches the dataset in memory on fallback hit.
    pub async fn get_dataset_or_load(&mut self, id: DatasetId) -> Option<&Dataset> {
        if self.datasets.contains_key(&id) {
            return self.datasets.get(&id);
        }
        match self.backend.get_dataset(id).await {
            Ok(Some(ds)) => {
                tracing::debug!(%id, "get_dataset_or_load: loaded from backend");
                self.datasets.insert(id, ds);
                self.datasets.get(&id)
            }
            _ => None,
        }
    }

    pub fn all_datasets(&self) -> impl Iterator<Item = &Dataset> {
        self.datasets.values()
    }

    pub async fn delete_dataset(&mut self, id: DatasetId) -> Result<bool, StorageError> {
        if !self.datasets.contains_key(&id) {
            return Ok(false);
        }
        // Delete from backend first (cascade handled by FK in SQLite)
        self.backend.delete_dataset(id).await?;
        // Then clean up cache
        self.datasets.remove(&id);
        // Remove associated datapoints from memory
        let dp_ids: Vec<DatapointId> = self
            .datapoints
            .values()
            .filter(|dp| dp.dataset_id == id)
            .map(|dp| dp.id)
            .collect();
        for dp_id in &dp_ids {
            self.datapoints.remove(dp_id);
        }
        // Remove associated queue items from memory
        let qi_ids: Vec<QueueItemId> = self
            .queue_items
            .values()
            .filter(|qi| qi.dataset_id == id)
            .map(|qi| qi.id)
            .collect();
        for qi_id in &qi_ids {
            self.queue_items.remove(qi_id);
        }
        // Remove associated eval runs and their results from memory
        let run_ids: Vec<EvalRunId> = self
            .eval_runs
            .values()
            .filter(|r| r.dataset_id == id)
            .map(|r| r.id)
            .collect();
        for run_id in &run_ids {
            self.eval_runs.remove(run_id);
            let result_ids: Vec<EvalResultId> = self
                .eval_results
                .values()
                .filter(|r| r.run_id == *run_id)
                .map(|r| r.id)
                .collect();
            for rid in result_ids {
                self.eval_results.remove(&rid);
            }
        }
        // Remove associated capture rules from memory
        let rule_ids: Vec<CaptureRuleId> = self
            .capture_rules
            .values()
            .filter(|r| r.dataset_id == id)
            .map(|r| r.id)
            .collect();
        for rule_id in &rule_ids {
            self.capture_rules.remove(rule_id);
        }
        Ok(true)
    }

    pub fn dataset_count(&self) -> usize {
        self.datasets.len()
    }

    // --- Datapoint methods ---

    pub async fn save_datapoint(&mut self, dp: Datapoint) -> Result<(), StorageError> {
        self.backend.save_datapoint(&dp).await?;
        self.datapoints.insert(dp.id, dp);
        Ok(())
    }

    pub fn get_datapoint(&self, id: DatapointId) -> Option<&Datapoint> {
        self.datapoints.get(&id)
    }

    pub fn datapoints_for_dataset(&self, dataset_id: DatasetId) -> Vec<&Datapoint> {
        self.datapoints
            .values()
            .filter(|dp| dp.dataset_id == dataset_id)
            .collect()
    }

    /// Load datapoints for a dataset from the storage backend and merge into memory.
    /// Used for multi-instance consistency — ensures datapoints created on other
    /// instances are available locally.
    pub async fn sync_datapoints_for_dataset(&mut self, dataset_id: DatasetId) {
        match self.backend.list_datapoints(dataset_id).await {
            Ok(dps) => {
                let count = dps.len();
                for dp in dps {
                    self.datapoints.entry(dp.id).or_insert(dp);
                }
                tracing::debug!(%dataset_id, count, "synced datapoints from backend");
            }
            Err(e) => {
                tracing::error!(%dataset_id, "failed to sync datapoints from backend: {}", e);
            }
        }
    }

    pub fn datapoint_count_for_dataset(&self, dataset_id: DatasetId) -> usize {
        self.datapoints
            .values()
            .filter(|dp| dp.dataset_id == dataset_id)
            .count()
    }

    pub async fn delete_datapoint(&mut self, id: DatapointId) -> Result<bool, StorageError> {
        if !self.datapoints.contains_key(&id) {
            return Ok(false);
        }
        // Delete from backend first
        self.backend.delete_datapoint(id).await?;
        // Then clean up cache
        self.datapoints.remove(&id);
        let qi_ids: Vec<QueueItemId> = self
            .queue_items
            .values()
            .filter(|qi| qi.datapoint_id == id)
            .map(|qi| qi.id)
            .collect();
        for qi_id in &qi_ids {
            self.queue_items.remove(qi_id);
        }
        Ok(true)
    }

    // --- Queue methods ---

    pub async fn save_queue_item(&mut self, item: QueueItem) -> Result<(), StorageError> {
        self.backend.save_queue_item(&item).await?;
        self.queue_items.insert(item.id, item);
        Ok(())
    }

    pub fn get_queue_item(&self, id: QueueItemId) -> Option<&QueueItem> {
        self.queue_items.get(&id)
    }

    pub fn queue_items_for_dataset(&self, dataset_id: DatasetId) -> Vec<&QueueItem> {
        self.queue_items
            .values()
            .filter(|qi| qi.dataset_id == dataset_id)
            .collect()
    }

    pub fn all_queue_items(&self) -> Vec<&QueueItem> {
        self.queue_items.values().collect()
    }

    pub async fn claim_queue_item(
        &mut self,
        id: QueueItemId,
        claimed_by: impl Into<String>,
    ) -> Result<Option<QueueItem>, StorageError> {
        let item = match self.queue_items.remove(&id) {
            Some(i) => i,
            None => return Ok(None),
        };
        if item.status != QueueItemStatus::Pending {
            self.queue_items.insert(id, item);
            return Ok(None);
        }
        let claimed = item.claim(claimed_by);
        self.backend.save_queue_item(&claimed).await?;
        self.queue_items.insert(id, claimed.clone());
        Ok(Some(claimed))
    }

    pub async fn complete_queue_item(
        &mut self,
        id: QueueItemId,
        edited_data: Option<serde_json::Value>,
    ) -> Result<Option<QueueItem>, StorageError> {
        let item = match self.queue_items.remove(&id) {
            Some(i) => i,
            None => return Ok(None),
        };
        if item.status != QueueItemStatus::Claimed {
            self.queue_items.insert(id, item);
            return Ok(None);
        }
        let completed = item.complete(edited_data);
        self.backend.save_queue_item(&completed).await?;
        self.queue_items.insert(id, completed.clone());
        Ok(Some(completed))
    }

    // --- Eval Run methods ---

    pub async fn save_eval_run(&mut self, run: EvalRun) -> Result<(), StorageError> {
        self.backend.save_eval_run(&run).await?;
        self.eval_runs.insert(run.id, run);
        Ok(())
    }

    pub fn get_eval_run(&self, id: EvalRunId) -> Option<&EvalRun> {
        self.eval_runs.get(&id)
    }

    pub fn eval_runs_for_dataset(&self, dataset_id: DatasetId) -> Vec<&EvalRun> {
        self.eval_runs
            .values()
            .filter(|r| r.dataset_id == dataset_id)
            .collect()
    }

    pub async fn delete_eval_run(&mut self, id: EvalRunId) -> Result<bool, StorageError> {
        if !self.eval_runs.contains_key(&id) {
            return Ok(false);
        }
        // Delete from backend first
        self.backend.delete_eval_run_results(id).await?;
        self.backend.delete_eval_run(id).await?;
        // Then clean up cache
        self.eval_runs.remove(&id);
        let result_ids: Vec<EvalResultId> = self
            .eval_results
            .values()
            .filter(|r| r.run_id == id)
            .map(|r| r.id)
            .collect();
        for rid in result_ids {
            self.eval_results.remove(&rid);
        }
        Ok(true)
    }

    // --- Eval Result methods ---

    pub async fn save_eval_result(&mut self, result: EvalResult) -> Result<(), StorageError> {
        self.backend.save_eval_result(&result).await?;
        self.eval_results.insert(result.id, result);
        Ok(())
    }

    pub fn get_eval_result(&self, id: EvalResultId) -> Option<&EvalResult> {
        self.eval_results.get(&id)
    }

    pub fn eval_results_for_run(&self, run_id: EvalRunId) -> Vec<&EvalResult> {
        self.eval_results
            .values()
            .filter(|r| r.run_id == run_id)
            .collect()
    }

    // --- Capture Rule methods ---

    pub async fn save_capture_rule(&mut self, rule: CaptureRule) -> Result<(), StorageError> {
        self.backend.save_capture_rule(&rule).await?;
        self.capture_rules.insert(rule.id, rule);
        Ok(())
    }

    pub fn get_capture_rule(&self, id: CaptureRuleId) -> Option<&CaptureRule> {
        self.capture_rules.get(&id)
    }

    pub fn capture_rules_for_dataset(&self, dataset_id: DatasetId) -> Vec<&CaptureRule> {
        self.capture_rules
            .values()
            .filter(|r| r.dataset_id == dataset_id)
            .collect()
    }

    pub fn all_enabled_capture_rules(&self) -> Vec<&CaptureRule> {
        self.capture_rules
            .values()
            .filter(|r| r.enabled)
            .collect()
    }

    pub async fn delete_capture_rule(&mut self, id: CaptureRuleId) -> Result<bool, StorageError> {
        if !self.capture_rules.contains_key(&id) {
            return Ok(false);
        }
        self.backend.delete_capture_rule(id).await?;
        self.capture_rules.remove(&id);
        Ok(true)
    }

    // --- Provider Connection operations ---

    pub async fn save_provider_connection(&mut self, conn: ProviderConnection) -> Result<(), StorageError> {
        self.backend.save_provider_connection(&conn).await?;
        self.provider_connections.insert(conn.id, conn);
        Ok(())
    }

    pub fn get_provider_connection(&self, id: ProviderConnectionId) -> Option<&ProviderConnection> {
        self.provider_connections.get(&id)
    }

    pub fn list_provider_connections(&self) -> Vec<&ProviderConnection> {
        self.provider_connections.values().collect()
    }

    pub async fn delete_provider_connection(&mut self, id: ProviderConnectionId) -> Result<bool, StorageError> {
        if !self.provider_connections.contains_key(&id) {
            return Ok(false);
        }
        self.backend.delete_provider_connection(id).await?;
        self.provider_connections.remove(&id);
        Ok(true)
    }
}
