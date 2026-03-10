//! Runtime-polymorphic storage backend.
//!
//! `AnyBackend` wraps the concrete backend implementations (SQLite for local,
//! Turbopuffer for cloud) behind a single type so that the rest of the codebase
//! can be monomorphic over `PersistentStore<AnyBackend>`.

use async_trait::async_trait;
use storage_sqlite::SqliteBackend;
use storage_turbopuffer::TurbopufferBackend;
use trace::{
    CaptureRule, CaptureRuleId, Datapoint, DatapointId, Dataset, DatasetId, EvalResult,
    EvalResultId, EvalRun, EvalRunId, FileVersion, ProviderConnection, ProviderConnectionId,
    QueueItem, QueueItemId, Span, SpanId, Trace, TraceId,
};

use storage::error::StorageError;
use storage::filter::{SpanFilter, TraceFilter};
use storage::StorageBackend;

/// A storage backend that dispatches to either SQLite (local) or Turbopuffer (cloud)
/// at runtime.
pub enum AnyBackend {
    Sqlite(SqliteBackend),
    Turbopuffer(TurbopufferBackend),
}

macro_rules! delegate {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            AnyBackend::Sqlite(b) => b.$method($($arg),*).await,
            AnyBackend::Turbopuffer(b) => b.$method($($arg),*).await,
        }
    };
}

#[async_trait]
impl StorageBackend for AnyBackend {
    // --- Trace operations ---

    async fn save_trace(&self, trace: &Trace) -> Result<(), StorageError> {
        delegate!(self, save_trace, trace)
    }

    async fn get_trace(&self, id: TraceId) -> Result<Option<Trace>, StorageError> {
        delegate!(self, get_trace, id)
    }

    async fn list_traces(&self, filter: &TraceFilter) -> Result<Vec<Trace>, StorageError> {
        delegate!(self, list_traces, filter)
    }

    async fn delete_trace(&self, id: TraceId) -> Result<bool, StorageError> {
        delegate!(self, delete_trace, id)
    }

    // --- Span operations ---

    async fn save_span(&self, span: &Span) -> Result<(), StorageError> {
        delegate!(self, save_span, span)
    }

    async fn get_span(&self, id: SpanId) -> Result<Option<Span>, StorageError> {
        delegate!(self, get_span, id)
    }

    async fn list_spans(&self, filter: &SpanFilter) -> Result<Vec<Span>, StorageError> {
        delegate!(self, list_spans, filter)
    }

    async fn delete_span(&self, id: SpanId) -> Result<bool, StorageError> {
        delegate!(self, delete_span, id)
    }

    async fn delete_trace_spans(&self, trace_id: TraceId) -> Result<usize, StorageError> {
        delegate!(self, delete_trace_spans, trace_id)
    }

    async fn clear_spans(&self) -> Result<(), StorageError> {
        delegate!(self, clear_spans)
    }

    // --- Dataset operations ---

    async fn save_dataset(&self, dataset: &Dataset) -> Result<(), StorageError> {
        delegate!(self, save_dataset, dataset)
    }

    async fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>, StorageError> {
        delegate!(self, get_dataset, id)
    }

    async fn list_datasets(&self) -> Result<Vec<Dataset>, StorageError> {
        delegate!(self, list_datasets)
    }

    async fn delete_dataset(&self, id: DatasetId) -> Result<bool, StorageError> {
        delegate!(self, delete_dataset, id)
    }

    // --- Datapoint operations ---

    async fn save_datapoint(&self, dp: &Datapoint) -> Result<(), StorageError> {
        delegate!(self, save_datapoint, dp)
    }

    async fn get_datapoint(&self, id: DatapointId) -> Result<Option<Datapoint>, StorageError> {
        delegate!(self, get_datapoint, id)
    }

    async fn list_datapoints(&self, dataset_id: DatasetId) -> Result<Vec<Datapoint>, StorageError> {
        delegate!(self, list_datapoints, dataset_id)
    }

    async fn delete_datapoint(&self, id: DatapointId) -> Result<bool, StorageError> {
        delegate!(self, delete_datapoint, id)
    }

    async fn delete_dataset_datapoints(
        &self,
        dataset_id: DatasetId,
    ) -> Result<usize, StorageError> {
        delegate!(self, delete_dataset_datapoints, dataset_id)
    }

    // --- Queue operations ---

    async fn save_queue_item(&self, item: &QueueItem) -> Result<(), StorageError> {
        delegate!(self, save_queue_item, item)
    }

    async fn get_queue_item(&self, id: QueueItemId) -> Result<Option<QueueItem>, StorageError> {
        delegate!(self, get_queue_item, id)
    }

    async fn list_queue_items(
        &self,
        dataset_id: DatasetId,
    ) -> Result<Vec<QueueItem>, StorageError> {
        delegate!(self, list_queue_items, dataset_id)
    }

    async fn delete_queue_item(&self, id: QueueItemId) -> Result<bool, StorageError> {
        delegate!(self, delete_queue_item, id)
    }

    // --- Eval Run operations ---

    async fn save_eval_run(&self, run: &EvalRun) -> Result<(), StorageError> {
        delegate!(self, save_eval_run, run)
    }

    async fn get_eval_run(&self, id: EvalRunId) -> Result<Option<EvalRun>, StorageError> {
        delegate!(self, get_eval_run, id)
    }

    async fn list_eval_runs(&self, dataset_id: DatasetId) -> Result<Vec<EvalRun>, StorageError> {
        delegate!(self, list_eval_runs, dataset_id)
    }

    async fn delete_eval_run(&self, id: EvalRunId) -> Result<bool, StorageError> {
        delegate!(self, delete_eval_run, id)
    }

    // --- Eval Result operations ---

    async fn save_eval_result(&self, result: &EvalResult) -> Result<(), StorageError> {
        delegate!(self, save_eval_result, result)
    }

    async fn get_eval_result(&self, id: EvalResultId) -> Result<Option<EvalResult>, StorageError> {
        delegate!(self, get_eval_result, id)
    }

    async fn list_eval_results(&self, run_id: EvalRunId) -> Result<Vec<EvalResult>, StorageError> {
        delegate!(self, list_eval_results, run_id)
    }

    async fn delete_eval_run_results(&self, run_id: EvalRunId) -> Result<usize, StorageError> {
        delegate!(self, delete_eval_run_results, run_id)
    }

    // --- Capture Rule operations ---

    async fn save_capture_rule(&self, rule: &CaptureRule) -> Result<(), StorageError> {
        delegate!(self, save_capture_rule, rule)
    }

    async fn get_capture_rule(&self, id: CaptureRuleId) -> Result<Option<CaptureRule>, StorageError> {
        delegate!(self, get_capture_rule, id)
    }

    async fn list_capture_rules(&self, dataset_id: DatasetId) -> Result<Vec<CaptureRule>, StorageError> {
        delegate!(self, list_capture_rules, dataset_id)
    }

    async fn delete_capture_rule(&self, id: CaptureRuleId) -> Result<bool, StorageError> {
        delegate!(self, delete_capture_rule, id)
    }

    // --- Provider Connection operations ---

    async fn save_provider_connection(&self, conn: &ProviderConnection) -> Result<(), StorageError> {
        delegate!(self, save_provider_connection, conn)
    }

    async fn get_provider_connection(&self, id: ProviderConnectionId) -> Result<Option<ProviderConnection>, StorageError> {
        delegate!(self, get_provider_connection, id)
    }

    async fn list_provider_connections(&self) -> Result<Vec<ProviderConnection>, StorageError> {
        delegate!(self, list_provider_connections)
    }

    async fn delete_provider_connection(&self, id: ProviderConnectionId) -> Result<bool, StorageError> {
        delegate!(self, delete_provider_connection, id)
    }

    // --- File operations ---

    async fn save_file_version(&self, version: &FileVersion) -> Result<(), StorageError> {
        delegate!(self, save_file_version, version)
    }

    async fn list_file_versions(&self) -> Result<Vec<FileVersion>, StorageError> {
        delegate!(self, list_file_versions)
    }

    async fn save_file_content(&self, hash: &str, content: &[u8]) -> Result<(), StorageError> {
        delegate!(self, save_file_content, hash, content)
    }

    async fn load_file_content(&self, hash: &str) -> Result<Vec<u8>, StorageError> {
        delegate!(self, load_file_content, hash)
    }

    // --- Batch operations ---

    async fn save_spans_batch(&self, spans: &[Span]) -> Result<(), StorageError> {
        delegate!(self, save_spans_batch, spans)
    }

    async fn save_datapoints_batch(&self, datapoints: &[Datapoint]) -> Result<(), StorageError> {
        delegate!(self, save_datapoints_batch, datapoints)
    }

    // --- Load-all operations ---

    async fn load_all_spans(&self) -> Result<Vec<Span>, StorageError> {
        delegate!(self, load_all_spans)
    }

    async fn load_all_traces(&self) -> Result<Vec<Trace>, StorageError> {
        delegate!(self, load_all_traces)
    }

    async fn load_all_datasets(&self) -> Result<Vec<Dataset>, StorageError> {
        delegate!(self, load_all_datasets)
    }

    async fn load_all_datapoints(&self) -> Result<Vec<Datapoint>, StorageError> {
        delegate!(self, load_all_datapoints)
    }

    async fn list_datapoints_all(&self) -> Result<Vec<Datapoint>, StorageError> {
        delegate!(self, list_datapoints_all)
    }

    async fn load_all_queue_items(&self) -> Result<Vec<QueueItem>, StorageError> {
        delegate!(self, load_all_queue_items)
    }

    async fn list_queue_items_all(&self) -> Result<Vec<QueueItem>, StorageError> {
        delegate!(self, list_queue_items_all)
    }

    async fn load_all_files(&self) -> Result<Vec<FileVersion>, StorageError> {
        delegate!(self, load_all_files)
    }

    // --- Eval/Rule load-all ---

    async fn load_all_eval_runs(&self) -> Result<Vec<EvalRun>, StorageError> {
        delegate!(self, load_all_eval_runs)
    }

    async fn list_eval_runs_all(&self) -> Result<Vec<EvalRun>, StorageError> {
        delegate!(self, list_eval_runs_all)
    }

    async fn load_all_eval_results(&self) -> Result<Vec<EvalResult>, StorageError> {
        delegate!(self, load_all_eval_results)
    }

    async fn list_eval_results_all(&self) -> Result<Vec<EvalResult>, StorageError> {
        delegate!(self, list_eval_results_all)
    }

    async fn load_all_capture_rules(&self) -> Result<Vec<CaptureRule>, StorageError> {
        delegate!(self, load_all_capture_rules)
    }

    async fn list_capture_rules_all(&self) -> Result<Vec<CaptureRule>, StorageError> {
        delegate!(self, list_capture_rules_all)
    }

    async fn load_all_provider_connections(&self) -> Result<Vec<ProviderConnection>, StorageError> {
        delegate!(self, load_all_provider_connections)
    }

    // --- Metadata ---

    fn backend_type(&self) -> &'static str {
        match self {
            AnyBackend::Sqlite(b) => b.backend_type(),
            AnyBackend::Turbopuffer(b) => b.backend_type(),
        }
    }
}
