use async_trait::async_trait;
use trace::{
    CaptureRule, CaptureRuleId, Datapoint, DatapointId, Dataset, DatasetId, EvalResult,
    EvalResultId, EvalRun, EvalRunId, FileVersion, ProviderConnection, ProviderConnectionId,
    QueueItem, QueueItemId, Span, SpanId, Trace, TraceId,
};

use crate::error::StorageError;
use crate::filter::{SpanFilter, TraceFilter};

/// Trait for pluggable storage backends.
///
/// This trait defines a unified interface for storage operations,
/// enabling seamless switching between local SQLite and cloud Turbopuffer storage.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // --- Trace operations ---

    /// Save or update a trace.
    async fn save_trace(&self, trace: &Trace) -> Result<(), StorageError>;

    /// Get a trace by ID.
    async fn get_trace(&self, id: TraceId) -> Result<Option<Trace>, StorageError>;

    /// List traces matching the filter.
    async fn list_traces(&self, filter: &TraceFilter) -> Result<Vec<Trace>, StorageError>;

    /// Delete a trace by ID. Returns true if deleted.
    async fn delete_trace(&self, id: TraceId) -> Result<bool, StorageError>;

    // --- Span operations ---

    /// Save or update a span.
    async fn save_span(&self, span: &Span) -> Result<(), StorageError>;

    /// Get a span by ID.
    async fn get_span(&self, id: SpanId) -> Result<Option<Span>, StorageError>;

    /// List spans matching the filter.
    async fn list_spans(&self, filter: &SpanFilter) -> Result<Vec<Span>, StorageError>;

    /// Delete a span by ID. Returns true if deleted.
    async fn delete_span(&self, id: SpanId) -> Result<bool, StorageError>;

    /// Delete all spans for a trace. Returns count of deleted spans.
    async fn delete_trace_spans(&self, trace_id: TraceId) -> Result<usize, StorageError>;

    /// Clear all spans.
    async fn clear_spans(&self) -> Result<(), StorageError>;

    // --- Dataset operations ---

    /// Save or update a dataset.
    async fn save_dataset(&self, dataset: &Dataset) -> Result<(), StorageError>;

    /// Get a dataset by ID.
    async fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>, StorageError>;

    /// List all datasets.
    async fn list_datasets(&self) -> Result<Vec<Dataset>, StorageError>;

    /// Delete a dataset by ID. Returns true if deleted.
    async fn delete_dataset(&self, id: DatasetId) -> Result<bool, StorageError>;

    // --- Datapoint operations ---

    /// Save or update a datapoint.
    async fn save_datapoint(&self, dp: &Datapoint) -> Result<(), StorageError>;

    /// Get a datapoint by ID.
    async fn get_datapoint(&self, id: DatapointId) -> Result<Option<Datapoint>, StorageError>;

    /// List datapoints for a dataset.
    async fn list_datapoints(&self, dataset_id: DatasetId) -> Result<Vec<Datapoint>, StorageError>;

    /// Delete a datapoint by ID. Returns true if deleted.
    async fn delete_datapoint(&self, id: DatapointId) -> Result<bool, StorageError>;

    /// Delete all datapoints for a dataset. Returns count of deleted.
    async fn delete_dataset_datapoints(&self, dataset_id: DatasetId) -> Result<usize, StorageError>;

    // --- Queue operations ---

    /// Save or update a queue item.
    async fn save_queue_item(&self, item: &QueueItem) -> Result<(), StorageError>;

    /// Get a queue item by ID.
    async fn get_queue_item(&self, id: QueueItemId) -> Result<Option<QueueItem>, StorageError>;

    /// List queue items for a dataset.
    async fn list_queue_items(&self, dataset_id: DatasetId) -> Result<Vec<QueueItem>, StorageError>;

    /// Delete a queue item by ID. Returns true if deleted.
    async fn delete_queue_item(&self, id: QueueItemId) -> Result<bool, StorageError>;

    // --- Eval Run operations ---

    /// Save or update an eval run.
    async fn save_eval_run(&self, run: &EvalRun) -> Result<(), StorageError>;

    /// Get an eval run by ID.
    async fn get_eval_run(&self, id: EvalRunId) -> Result<Option<EvalRun>, StorageError>;

    /// List eval runs for a dataset.
    async fn list_eval_runs(&self, dataset_id: DatasetId) -> Result<Vec<EvalRun>, StorageError>;

    /// Delete an eval run by ID. Returns true if deleted.
    async fn delete_eval_run(&self, id: EvalRunId) -> Result<bool, StorageError>;

    // --- Eval Result operations ---

    /// Save or update an eval result.
    async fn save_eval_result(&self, result: &EvalResult) -> Result<(), StorageError>;

    /// Get an eval result by ID.
    async fn get_eval_result(&self, id: EvalResultId) -> Result<Option<EvalResult>, StorageError>;

    /// List eval results for a run.
    async fn list_eval_results(&self, run_id: EvalRunId) -> Result<Vec<EvalResult>, StorageError>;

    /// Delete all eval results for a run. Returns count of deleted.
    async fn delete_eval_run_results(&self, run_id: EvalRunId) -> Result<usize, StorageError>;

    // --- Capture Rule operations ---

    /// Save or update a capture rule.
    async fn save_capture_rule(&self, rule: &CaptureRule) -> Result<(), StorageError>;

    /// Get a capture rule by ID.
    async fn get_capture_rule(&self, id: CaptureRuleId) -> Result<Option<CaptureRule>, StorageError>;

    /// List capture rules for a dataset.
    async fn list_capture_rules(&self, dataset_id: DatasetId) -> Result<Vec<CaptureRule>, StorageError>;

    /// Delete a capture rule by ID. Returns true if deleted.
    async fn delete_capture_rule(&self, id: CaptureRuleId) -> Result<bool, StorageError>;

    // --- File operations ---

    /// Save a file version record.
    async fn save_file_version(&self, version: &FileVersion) -> Result<(), StorageError>;

    /// List all file versions.
    async fn list_file_versions(&self) -> Result<Vec<FileVersion>, StorageError>;

    /// Save file content by hash.
    async fn save_file_content(&self, hash: &str, content: &[u8]) -> Result<(), StorageError>;

    /// Load file content by hash.
    async fn load_file_content(&self, hash: &str) -> Result<Vec<u8>, StorageError>;

    // --- Batch operations (for cloud efficiency) ---

    /// Save multiple spans in a batch.
    /// Default implementation calls save_span for each.
    async fn save_spans_batch(&self, spans: &[Span]) -> Result<(), StorageError> {
        for span in spans {
            self.save_span(span).await?;
        }
        Ok(())
    }

    /// Save multiple datapoints in a batch.
    /// Default implementation calls save_datapoint for each.
    async fn save_datapoints_batch(&self, datapoints: &[Datapoint]) -> Result<(), StorageError> {
        for dp in datapoints {
            self.save_datapoint(dp).await?;
        }
        Ok(())
    }

    // --- Load-all operations (for initialization) ---

    /// Load all spans. Used during store initialization.
    async fn load_all_spans(&self) -> Result<Vec<Span>, StorageError> {
        self.list_spans(&SpanFilter::default()).await
    }

    /// Load all traces. Used during store initialization.
    async fn load_all_traces(&self) -> Result<Vec<Trace>, StorageError> {
        self.list_traces(&TraceFilter::default()).await
    }

    /// Load all datasets. Used during store initialization.
    async fn load_all_datasets(&self) -> Result<Vec<Dataset>, StorageError> {
        self.list_datasets().await
    }

    /// Load all datapoints. Used during store initialization.
    async fn load_all_datapoints(&self) -> Result<Vec<Datapoint>, StorageError> {
        self.list_datapoints_all().await
    }

    /// Load all datapoints across all datasets.
    async fn list_datapoints_all(&self) -> Result<Vec<Datapoint>, StorageError>;

    /// Load all queue items. Used during store initialization.
    async fn load_all_queue_items(&self) -> Result<Vec<QueueItem>, StorageError> {
        self.list_queue_items_all().await
    }

    /// Load all queue items across all datasets.
    async fn list_queue_items_all(&self) -> Result<Vec<QueueItem>, StorageError>;

    /// Load all file versions. Used during store initialization.
    async fn load_all_files(&self) -> Result<Vec<FileVersion>, StorageError> {
        self.list_file_versions().await
    }

    /// Load all eval runs. Used during store initialization.
    async fn load_all_eval_runs(&self) -> Result<Vec<EvalRun>, StorageError> {
        self.list_eval_runs_all().await
    }

    /// List all eval runs across all datasets.
    async fn list_eval_runs_all(&self) -> Result<Vec<EvalRun>, StorageError>;

    /// Load all eval results. Used during store initialization.
    async fn load_all_eval_results(&self) -> Result<Vec<EvalResult>, StorageError> {
        self.list_eval_results_all().await
    }

    /// List all eval results across all runs.
    async fn list_eval_results_all(&self) -> Result<Vec<EvalResult>, StorageError>;

    /// Load all capture rules. Used during store initialization.
    async fn load_all_capture_rules(&self) -> Result<Vec<CaptureRule>, StorageError> {
        self.list_capture_rules_all().await
    }

    /// List all capture rules across all datasets.
    async fn list_capture_rules_all(&self) -> Result<Vec<CaptureRule>, StorageError>;

    // --- Provider Connection operations ---

    /// Save or update a provider connection.
    async fn save_provider_connection(&self, conn: &ProviderConnection) -> Result<(), StorageError>;

    /// Get a provider connection by ID.
    async fn get_provider_connection(&self, id: ProviderConnectionId) -> Result<Option<ProviderConnection>, StorageError>;

    /// List all provider connections.
    async fn list_provider_connections(&self) -> Result<Vec<ProviderConnection>, StorageError>;

    /// Delete a provider connection by ID. Returns true if deleted.
    async fn delete_provider_connection(&self, id: ProviderConnectionId) -> Result<bool, StorageError>;

    /// Load all provider connections. Used during store initialization.
    async fn load_all_provider_connections(&self) -> Result<Vec<ProviderConnection>, StorageError> {
        self.list_provider_connections().await
    }

    // --- Metadata ---

    /// Returns the type of this backend (e.g., "sqlite", "turbopuffer").
    fn backend_type(&self) -> &'static str;
}
