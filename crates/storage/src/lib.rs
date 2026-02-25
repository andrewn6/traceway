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
    ) -> Option<Span> {
        // Try memory first, then fall back to backend
        let span = match self.memory.remove(id) {
            Some(s) => s,
            None => {
                match self.backend.get_span(id).await {
                    Ok(Some(s)) => {
                        tracing::debug!(%id, "complete_span: loaded span from backend");
                        s
                    }
                    _ => return None,
                }
            }
        };
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

    /// Complete a span with an updated SpanKind (e.g. to populate token counts).
    /// Uses serde JSON round-trip to reconstruct with new kind, same pattern as SqliteBackend::deserialize_span.
    /// Falls back to the storage backend if the span is not in memory.
    pub async fn complete_span_with_kind(
        &mut self,
        id: SpanId,
        kind: SpanKind,
        output: Option<serde_json::Value>,
    ) -> Option<Span> {
        let span = match self.memory.remove(id) {
            Some(s) => s,
            None => {
                match self.backend.get_span(id).await {
                    Ok(Some(s)) => {
                        tracing::debug!(%id, "complete_span_with_kind: loaded span from backend");
                        s
                    }
                    _ => return None,
                }
            }
        };
        if span.status().is_terminal() {
            self.memory.replace(span);
            return None;
        }
        // Serialize the span to JSON, patch in the new kind, then deserialize back
        let mut json = serde_json::to_value(&span).ok()?;
        let kind_json = serde_json::to_value(&kind).ok()?;
        json.as_object_mut()?.insert("kind".to_string(), kind_json);
        json.as_object_mut()?
            .insert("status".to_string(), serde_json::Value::String("completed".to_string()));
        json.as_object_mut()?
            .insert("ended_at".to_string(), serde_json::to_value(chrono::Utc::now()).ok()?);
        if let Some(out) = &output {
            json.as_object_mut()?
                .insert("output".to_string(), out.clone());
        }
        let completed: Span = serde_json::from_value(json).ok()?;
        self.memory.replace(completed.clone());
        if let Err(e) = self.backend.save_span(&completed).await {
            tracing::error!(%id, "failed to persist span completion with kind: {}", e);
        }
        Some(completed)
    }

    /// Fail a span (immutable transition: Running -> Failed).
    /// Falls back to the storage backend if the span is not in memory.
    pub async fn fail_span(&mut self, id: SpanId, error: impl Into<String>) -> Option<Span> {
        let span = match self.memory.remove(id) {
            Some(s) => s,
            None => {
                match self.backend.get_span(id).await {
                    Ok(Some(s)) => {
                        tracing::debug!(%id, "fail_span: loaded span from backend");
                        s
                    }
                    _ => return None,
                }
            }
        };
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

    /// Delete all spans started before the given cutoff time.
    /// Returns the number of spans deleted.
    pub async fn delete_spans_before(&mut self, cutoff: chrono::DateTime<chrono::Utc>) -> usize {
        let expired_ids: Vec<SpanId> = self.memory
            .all_spans()
            .filter(|s| s.started_at() < cutoff)
            .map(|s| s.id())
            .collect();

        let count = expired_ids.len();
        for id in expired_ids {
            self.memory.delete_span(id);
            if let Err(e) = self.backend.delete_span(id).await {
                tracing::error!(%id, "failed to persist span deletion during retention cleanup: {}", e);
            }
        }

        // Also clean up traces that now have zero spans
        let empty_traces: Vec<TraceId> = self.trace_meta.keys()
            .filter(|tid| self.memory.spans_for_trace(**tid).is_empty())
            .cloned()
            .collect();
        for tid in empty_traces {
            self.trace_meta.remove(&tid);
            let _ = self.backend.delete_trace(tid).await;
        }

        if count > 0 {
            tracing::info!(count, "retention cleanup: deleted expired spans");
        }
        count
    }

    pub async fn clear(&mut self) {
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

    // --- Dataset methods ---

    pub async fn save_dataset(&mut self, dataset: Dataset) {
        if let Err(e) = self.backend.save_dataset(&dataset).await {
            tracing::error!("failed to persist dataset: {}", e);
        }
        self.datasets.insert(dataset.id, dataset);
    }

    pub fn get_dataset(&self, id: DatasetId) -> Option<&Dataset> {
        self.datasets.get(&id)
    }

    pub fn all_datasets(&self) -> impl Iterator<Item = &Dataset> {
        self.datasets.values()
    }

    pub async fn delete_dataset(&mut self, id: DatasetId) -> bool {
        if self.datasets.remove(&id).is_some() {
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
            // Cascade delete handled by FK in SQLite, just delete the dataset
            let _ = self.backend.delete_dataset(id).await;
            true
        } else {
            false
        }
    }

    pub fn dataset_count(&self) -> usize {
        self.datasets.len()
    }

    // --- Datapoint methods ---

    pub async fn save_datapoint(&mut self, dp: Datapoint) {
        if let Err(e) = self.backend.save_datapoint(&dp).await {
            tracing::error!("failed to persist datapoint: {}", e);
        }
        self.datapoints.insert(dp.id, dp);
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

    pub fn datapoint_count_for_dataset(&self, dataset_id: DatasetId) -> usize {
        self.datapoints
            .values()
            .filter(|dp| dp.dataset_id == dataset_id)
            .count()
    }

    pub async fn delete_datapoint(&mut self, id: DatapointId) -> bool {
        if self.datapoints.remove(&id).is_some() {
            // Remove queue items referencing this datapoint
            let qi_ids: Vec<QueueItemId> = self
                .queue_items
                .values()
                .filter(|qi| qi.datapoint_id == id)
                .map(|qi| qi.id)
                .collect();
            for qi_id in &qi_ids {
                self.queue_items.remove(qi_id);
            }
            let _ = self.backend.delete_datapoint(id).await;
            true
        } else {
            false
        }
    }

    // --- Queue methods ---

    pub async fn save_queue_item(&mut self, item: QueueItem) {
        if let Err(e) = self.backend.save_queue_item(&item).await {
            tracing::error!("failed to persist queue item: {}", e);
        }
        self.queue_items.insert(item.id, item);
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

    pub async fn claim_queue_item(
        &mut self,
        id: QueueItemId,
        claimed_by: impl Into<String>,
    ) -> Option<QueueItem> {
        let item = self.queue_items.remove(&id)?;
        if item.status != QueueItemStatus::Pending {
            self.queue_items.insert(id, item);
            return None;
        }
        let claimed = item.claim(claimed_by);
        if let Err(e) = self.backend.save_queue_item(&claimed).await {
            tracing::error!("failed to persist queue item claim: {}", e);
        }
        self.queue_items.insert(id, claimed.clone());
        Some(claimed)
    }

    pub async fn complete_queue_item(
        &mut self,
        id: QueueItemId,
        edited_data: Option<serde_json::Value>,
    ) -> Option<QueueItem> {
        let item = self.queue_items.remove(&id)?;
        if item.status != QueueItemStatus::Claimed {
            self.queue_items.insert(id, item);
            return None;
        }
        let completed = item.complete(edited_data);
        if let Err(e) = self.backend.save_queue_item(&completed).await {
            tracing::error!("failed to persist queue item completion: {}", e);
        }
        self.queue_items.insert(id, completed.clone());
        Some(completed)
    }

    // --- Eval Run methods ---

    pub async fn save_eval_run(&mut self, run: EvalRun) {
        if let Err(e) = self.backend.save_eval_run(&run).await {
            tracing::error!("failed to persist eval run: {}", e);
        }
        self.eval_runs.insert(run.id, run);
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

    pub async fn delete_eval_run(&mut self, id: EvalRunId) -> bool {
        if self.eval_runs.remove(&id).is_some() {
            // Remove associated results from memory
            let result_ids: Vec<EvalResultId> = self
                .eval_results
                .values()
                .filter(|r| r.run_id == id)
                .map(|r| r.id)
                .collect();
            for rid in result_ids {
                self.eval_results.remove(&rid);
            }
            let _ = self.backend.delete_eval_run_results(id).await;
            let _ = self.backend.delete_eval_run(id).await;
            true
        } else {
            false
        }
    }

    // --- Eval Result methods ---

    pub async fn save_eval_result(&mut self, result: EvalResult) {
        if let Err(e) = self.backend.save_eval_result(&result).await {
            tracing::error!("failed to persist eval result: {}", e);
        }
        self.eval_results.insert(result.id, result);
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

    pub async fn save_capture_rule(&mut self, rule: CaptureRule) {
        if let Err(e) = self.backend.save_capture_rule(&rule).await {
            tracing::error!("failed to persist capture rule: {}", e);
        }
        self.capture_rules.insert(rule.id, rule);
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

    pub async fn delete_capture_rule(&mut self, id: CaptureRuleId) -> bool {
        if self.capture_rules.remove(&id).is_some() {
            let _ = self.backend.delete_capture_rule(id).await;
            true
        } else {
            false
        }
    }

    // --- Provider Connection operations ---

    pub async fn save_provider_connection(&mut self, conn: ProviderConnection) {
        if let Err(e) = self.backend.save_provider_connection(&conn).await {
            tracing::error!("failed to persist provider connection: {}", e);
        }
        self.provider_connections.insert(conn.id, conn);
    }

    pub fn get_provider_connection(&self, id: ProviderConnectionId) -> Option<&ProviderConnection> {
        self.provider_connections.get(&id)
    }

    pub fn list_provider_connections(&self) -> Vec<&ProviderConnection> {
        self.provider_connections.values().collect()
    }

    pub async fn delete_provider_connection(&mut self, id: ProviderConnectionId) -> bool {
        if self.provider_connections.remove(&id).is_some() {
            let _ = self.backend.delete_provider_connection(id).await;
            true
        } else {
            false
        }
    }
}
