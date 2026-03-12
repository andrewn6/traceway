//! Turbopuffer storage backend for Traceway cloud deployment.
//!
//! This module provides a cloud-native storage backend using Turbopuffer's
//! vector database as the primary storage layer. It supports:
//!
//! - Multi-tenant data isolation via namespace prefixes
//! - Efficient batch operations for high-throughput ingestion
//! - Optional vector embeddings for semantic search capabilities
//!
//! # Turbopuffer Schema Design
//!
//! Each entity type (traces, spans, datasets, etc.) is stored in a separate namespace
//! with the format `{org_namespace}_{collection}`. Documents are stored with:
//!
//! - `id`: Unique document ID (UUID string)
//! - `type`: Entity type for filtering within namespace
//! - `data`: Full JSON-serialized entity data
//! - Additional indexed attributes for filtering (trace_id, status, etc.)

use async_trait::async_trait;
use base64::Engine;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use storage::error::StorageError;
use storage::filter::{SpanFilter, TraceFilter};
use storage::StorageBackend;
use thiserror::Error;
use trace::{
    CaptureRule, CaptureRuleId, Datapoint, DatapointId, Dataset, DatasetId, EvalResult,
    EvalResultId, EvalRun, EvalRunId, FileVersion, ProviderConnection, ProviderConnectionId,
    QueueItem, QueueItemId, Span, SpanId, Trace, TraceId,
};
use tracing::{debug, info, instrument, warn};

const QUERY_PAGE_SIZE: usize = 10_000;

/// Turbopuffer-specific errors
#[derive(Debug, Error)]
pub enum TurbopufferError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<TurbopufferError> for StorageError {
    fn from(e: TurbopufferError) -> Self {
        match e {
            TurbopufferError::NotFound(_) => StorageError::NotFound,
            TurbopufferError::Config(msg) => StorageError::Configuration(msg),
            TurbopufferError::Http(e) => StorageError::Network(e.to_string()),
            _ => StorageError::Backend(e.to_string()),
        }
    }
}

/// Configuration for Turbopuffer backend
#[derive(Debug, Clone)]
pub struct TurbopufferConfig {
    /// Turbopuffer API key
    pub api_key: String,
    /// Base URL for Turbopuffer API (default: https://api.turbopuffer.com)
    pub base_url: String,
    /// Namespace prefix for multi-tenancy (e.g., "traceway_org123")
    pub namespace: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl TurbopufferConfig {
    pub fn from_env() -> Result<Self, TurbopufferError> {
        let api_key = std::env::var("TURBOPUFFER_API_KEY")
            .map_err(|_| TurbopufferError::Config("TURBOPUFFER_API_KEY not set".to_string()))?;

        let namespace =
            std::env::var("TURBOPUFFER_NAMESPACE").unwrap_or_else(|_| "traceway".to_string());

        let base_url = std::env::var("TURBOPUFFER_BASE_URL")
            .unwrap_or_else(|_| "https://gcp-us-central1.turbopuffer.com".to_string());

        let timeout_secs = std::env::var("TURBOPUFFER_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Ok(Self {
            api_key,
            base_url,
            namespace,
            timeout_secs,
        })
    }

    pub fn new(api_key: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://gcp-us-central1.turbopuffer.com".to_string(),
            namespace: namespace.into(),
            timeout_secs: 30,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Create a new config with a different namespace prefix (for per-org isolation)
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Derive a per-org config from this base config.
    /// Produces namespace like `tw_{org_id_short}` (first 8 chars of UUID).
    pub fn for_org(&self, org_id: &str) -> Self {
        let org_short = &org_id[..8.min(org_id.len())];
        Self {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            namespace: format!("tw_{}", org_short),
            timeout_secs: self.timeout_secs,
        }
    }
}

/// Row-based upsert request for Turbopuffer v2 API
#[derive(Debug, Serialize)]
struct UpsertRequest {
    upsert_rows: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
}

/// Query request for Turbopuffer v2 API
#[derive(Debug, Serialize)]
struct QueryRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    rank_by: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<usize>,
    include_attributes: serde_json::Value,
}

/// Query response from Turbopuffer
#[derive(Debug, Deserialize)]
struct QueryResponse {
    #[serde(default)]
    rows: Vec<serde_json::Value>,
}

/// Delete request for Turbopuffer v2 API
#[derive(Debug, Serialize)]
struct DeleteRequest {
    deletes: Vec<String>,
}

/// Turbopuffer storage backend implementation
pub struct TurbopufferBackend {
    client: Client,
    config: Arc<TurbopufferConfig>,
}

impl TurbopufferBackend {
    /// Create a new Turbopuffer backend with the given configuration
    pub fn new(config: TurbopufferConfig) -> Result<Self, TurbopufferError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .gzip(true)
            .build()?;

        info!(namespace = %config.namespace, "Initialized Turbopuffer backend");

        Ok(Self {
            client,
            config: Arc::new(config),
        })
    }

    /// Create a backend from environment variables
    pub fn from_env() -> Result<Self, TurbopufferError> {
        let config = TurbopufferConfig::from_env()?;
        Self::new(config)
    }

    /// Get the full namespace name for a collection type
    fn namespace(&self, collection: &str) -> String {
        format!("{}_{}", self.config.namespace, collection)
    }

    /// Make an authenticated POST request to Turbopuffer
    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, TurbopufferError> {
        let url = format!("{}{}", self.config.base_url, path);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(TurbopufferError::Api { status, message });
        }

        Ok(resp.json().await?)
    }

    /// Upsert documents to a namespace
    #[instrument(skip(self, rows), fields(count = rows.len()))]
    async fn upsert(
        &self,
        collection: &str,
        rows: Vec<serde_json::Value>,
    ) -> Result<(), TurbopufferError> {
        if rows.is_empty() {
            return Ok(());
        }

        let ns = self.namespace(collection);
        let path = format!("/v2/namespaces/{}", ns);

        debug!(namespace = %ns, count = rows.len(), "Upserting documents");

        // For non-vector namespaces, we don't need distance_metric
        let req = UpsertRequest {
            upsert_rows: rows,
            distance_metric: None,
            schema: None,
        };

        let _: serde_json::Value = self.post(&path, &req).await?;
        Ok(())
    }

    /// Upsert documents with an explicit schema (e.g. to mark attributes as non-filterable)
    #[instrument(skip(self, rows, schema), fields(count = rows.len()))]
    async fn upsert_with_schema(
        &self,
        collection: &str,
        rows: Vec<serde_json::Value>,
        schema: serde_json::Value,
    ) -> Result<(), TurbopufferError> {
        if rows.is_empty() {
            return Ok(());
        }

        let ns = self.namespace(collection);
        let path = format!("/v2/namespaces/{}", ns);

        debug!(namespace = %ns, count = rows.len(), "Upserting documents with schema");

        let req = UpsertRequest {
            upsert_rows: rows,
            distance_metric: None,
            schema: Some(schema),
        };

        let _: serde_json::Value = self.post(&path, &req).await?;
        Ok(())
    }

    /// Query documents from a namespace.
    /// Returns an empty vec if the namespace does not exist yet (404).
    #[instrument(skip(self, filters))]
    async fn query(
        &self,
        collection: &str,
        filters: Option<serde_json::Value>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, TurbopufferError> {
        let ns = self.namespace(collection);
        let path = format!("/v2/namespaces/{}/query", ns);

        // Order by id for consistent ordering when not using vectors
        let req = QueryRequest {
            rank_by: Some(serde_json::json!(["id", "asc"])),
            filters,
            top_k: Some(limit),
            include_attributes: serde_json::json!(true),
        };

        debug!(namespace = %ns, limit, "Querying documents");

        match self.post(&path, &req).await {
            Ok(resp) => {
                let resp: QueryResponse = resp;
                Ok(resp.rows)
            }
            Err(TurbopufferError::Api { status: 404, .. }) => {
                // Namespace doesn't exist yet — treat as empty collection
                debug!(namespace = %ns, "Namespace not found, returning empty result");
                Ok(vec![])
            }
            Err(e) => Err(e),
        }
    }

    /// Query all documents from a namespace with keyset pagination on `id`.
    ///
    /// This avoids silent truncation when collections exceed Turbopuffer `top_k` limits.
    async fn query_all(
        &self,
        collection: &str,
        filters: Option<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, TurbopufferError> {
        let mut rows = Vec::new();
        let mut last_id: Option<String> = None;

        loop {
            let page_filters = match (&filters, &last_id) {
                (None, None) => None,
                (Some(base), None) => Some(base.clone()),
                (None, Some(id)) => Some(serde_json::json!(["id", "Gt", id])),
                (Some(base), Some(id)) => {
                    Some(serde_json::json!(["And", [base.clone(), ["id", "Gt", id]]]))
                }
            };

            let page = self
                .query(collection, page_filters, QUERY_PAGE_SIZE)
                .await?;

            if page.is_empty() {
                break;
            }

            let page_len = page.len();
            let next_last_id = page
                .last()
                .and_then(|row| row.get("id"))
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);

            rows.extend(page);

            if page_len < QUERY_PAGE_SIZE {
                break;
            }

            warn!(collection, page_size = QUERY_PAGE_SIZE, "query page reached top_k; continuing pagination");

            match next_last_id {
                Some(id) => {
                    if last_id.as_deref() == Some(id.as_str()) {
                        warn!(collection, "pagination cursor did not advance; stopping to avoid loop");
                        break;
                    }
                    last_id = Some(id);
                }
                None => {
                    warn!(collection, "missing id in query row; stopping pagination early");
                    break;
                }
            }
        }

        Ok(rows)
    }

    /// Delete documents by ID.
    /// Returns 0 if the namespace does not exist yet (404).
    #[instrument(skip(self, ids))]
    async fn delete_ids(
        &self,
        collection: &str,
        ids: Vec<String>,
    ) -> Result<usize, TurbopufferError> {
        if ids.is_empty() {
            return Ok(0);
        }

        let ns = self.namespace(collection);
        let path = format!("/v2/namespaces/{}", ns);
        let count = ids.len();

        let req = DeleteRequest { deletes: ids };

        debug!(namespace = %ns, count, "Deleting documents");

        match self.post::<_, serde_json::Value>(&path, &req).await {
            Ok(_) => Ok(count),
            Err(TurbopufferError::Api { status: 404, .. }) => {
                debug!(namespace = %ns, "Namespace not found on delete, returning 0");
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    /// Get a single document by ID.
    /// Returns None if the namespace does not exist yet.
    async fn get_by_id(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<Option<serde_json::Value>, TurbopufferError> {
        let filter = serde_json::json!(["id", "Eq", id]);
        let results = self.query(collection, Some(filter), 1).await?;
        Ok(results.into_iter().next())
    }

    /// Extract data field from a query result row.
    ///
    /// With `include_attributes: true`, turbopuffer returns attributes flat
    /// at the top level of each row: `{"id": ..., "data": "...", ...}`.
    fn extract_data<T: for<'de> Deserialize<'de>>(row: &serde_json::Value) -> Option<T> {
        row.get("data")
            .and_then(|d| {
                if d.is_string() {
                    // Data stored as JSON string
                    d.as_str().and_then(|s| serde_json::from_str(s).ok())
                } else {
                    // Data stored as object
                    serde_json::from_value(d.clone()).ok()
                }
            })
    }
}

#[async_trait]
impl StorageBackend for TurbopufferBackend {
    fn backend_type(&self) -> &'static str {
        "turbopuffer"
    }

    // --- Trace operations ---

    async fn save_trace(&self, trace: &Trace) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": trace.id.to_string(),
            "data": serde_json::to_string(trace)?,
            "name": trace.name,
            "started_at": trace.started_at.to_rfc3339(),
            "ended_at": trace.ended_at.map(|t| t.to_rfc3339()),
        });

        self.upsert("traces", vec![row]).await?;
        Ok(())
    }

    async fn get_trace(&self, id: TraceId) -> Result<Option<Trace>, StorageError> {
        match self.get_by_id("traces", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_traces(&self, filter: &TraceFilter) -> Result<Vec<Trace>, StorageError> {
        let mut conditions = Vec::new();

        if let Some(ref name) = filter.name_contains {
            // Use Glob for partial matching
            conditions.push(serde_json::json!(["name", "Glob", format!("*{}*", name)]));
        }
        if let Some(since) = filter.since {
            conditions.push(serde_json::json!(["started_at", "Gte", since.to_rfc3339()]));
        }
        if let Some(until) = filter.until {
            conditions.push(serde_json::json!(["started_at", "Lte", until.to_rfc3339()]));
        }

        let filters = if conditions.is_empty() {
            None
        } else if conditions.len() == 1 {
            Some(conditions.remove(0))
        } else {
            Some(serde_json::json!(["And", conditions]))
        };

        let results = if let Some(limit) = filter.limit {
            self.query("traces", filters, limit).await?
        } else {
            self.query_all("traces", filters).await?
        };

        let mut traces = Vec::new();
        for row in results {
            if let Some(trace) = Self::extract_data::<Trace>(&row) {
                traces.push(trace);
            }
        }

        Ok(traces)
    }

    async fn delete_trace(&self, id: TraceId) -> Result<bool, StorageError> {
        let count = self.delete_ids("traces", vec![id.to_string()]).await?;
        Ok(count > 0)
    }

    // --- Span operations ---

    async fn save_span(&self, span: &Span) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": span.id().to_string(),
            "data": serde_json::to_string(span)?,
            "trace_id": span.trace_id().to_string(),
            "name": span.name(),
            "kind": span.kind().kind_name(),
            "status": span.status().as_str(),
            "model": span.kind().model(),
            "provider": span.kind().provider(),
            "started_at": span.started_at().to_rfc3339(),
            "ended_at": span.ended_at().map(|t| t.to_rfc3339()),
        });

        // Mark `data` as non-filterable since it can be large (LLM outputs)
        // and we only read it back, never filter on it. This also gives a 50% storage discount.
        let schema = serde_json::json!({
            "data": {"type": "string", "filterable": false}
        });
        self.upsert_with_schema("spans", vec![row], schema).await?;
        Ok(())
    }

    async fn get_span(&self, id: SpanId) -> Result<Option<Span>, StorageError> {
        match self.get_by_id("spans", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_spans(&self, filter: &SpanFilter) -> Result<Vec<Span>, StorageError> {
        let mut conditions = Vec::new();

        if let Some(ref trace_id) = filter.trace_id {
            conditions.push(serde_json::json!(["trace_id", "Eq", trace_id.to_string()]));
        }
        if let Some(ref status) = filter.status {
            conditions.push(serde_json::json!(["status", "Eq", status]));
        }
        if let Some(ref kind) = filter.kind {
            conditions.push(serde_json::json!(["kind", "Eq", kind]));
        }
        if let Some(ref model) = filter.model {
            conditions.push(serde_json::json!(["model", "Eq", model]));
        }
        if let Some(ref provider) = filter.provider {
            conditions.push(serde_json::json!(["provider", "Eq", provider]));
        }
        if let Some(ref name) = filter.name_contains {
            conditions.push(serde_json::json!(["name", "Glob", format!("*{}*", name)]));
        }
        if let Some(since) = filter.since {
            conditions.push(serde_json::json!(["started_at", "Gte", since.to_rfc3339()]));
        }
        if let Some(until) = filter.until {
            conditions.push(serde_json::json!(["started_at", "Lte", until.to_rfc3339()]));
        }

        let filters = if conditions.is_empty() {
            None
        } else if conditions.len() == 1 {
            Some(conditions.remove(0))
        } else {
            Some(serde_json::json!(["And", conditions]))
        };

        let results = if let Some(limit) = filter.limit {
            self.query("spans", filters, limit).await?
        } else {
            self.query_all("spans", filters).await?
        };

        let mut spans = Vec::new();
        for row in results {
            if let Some(span) = Self::extract_data::<Span>(&row) {
                spans.push(span);
            }
        }

        Ok(spans)
    }

    async fn delete_span(&self, id: SpanId) -> Result<bool, StorageError> {
        let count = self.delete_ids("spans", vec![id.to_string()]).await?;
        Ok(count > 0)
    }

    async fn delete_trace_spans(&self, trace_id: TraceId) -> Result<usize, StorageError> {
        let filter = SpanFilter {
            trace_id: Some(trace_id),
            ..Default::default()
        };
        let spans = self.list_spans(&filter).await?;
        let ids: Vec<String> = spans.iter().map(|s| s.id().to_string()).collect();
        let count = ids.len();

        if !ids.is_empty() {
            self.delete_ids("spans", ids).await?;
        }

        Ok(count)
    }

    async fn clear_spans(&self) -> Result<(), StorageError> {
        // Query all spans and delete them
        let spans = self.list_spans(&SpanFilter::default()).await?;
        let ids: Vec<String> = spans.iter().map(|s| s.id().to_string()).collect();

        if !ids.is_empty() {
            // Delete in batches to avoid request size limits
            for chunk in ids.chunks(1000) {
                self.delete_ids("spans", chunk.to_vec()).await?;
            }
        }

        Ok(())
    }

    // --- Dataset operations ---

    async fn save_dataset(&self, dataset: &Dataset) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": dataset.id.to_string(),
            "data": serde_json::to_string(dataset)?,
            "name": dataset.name,
            "created_at": dataset.created_at.to_rfc3339(),
            "updated_at": dataset.updated_at.to_rfc3339(),
        });

        self.upsert("datasets", vec![row]).await?;
        Ok(())
    }

    async fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>, StorageError> {
        match self.get_by_id("datasets", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_datasets(&self) -> Result<Vec<Dataset>, StorageError> {
        let results = self.query_all("datasets", None).await?;

        let mut datasets = Vec::new();
        for row in results {
            if let Some(dataset) = Self::extract_data::<Dataset>(&row) {
                datasets.push(dataset);
            }
        }

        Ok(datasets)
    }

    async fn delete_dataset(&self, id: DatasetId) -> Result<bool, StorageError> {
        // Delete associated datapoints first
        self.delete_dataset_datapoints(id).await?;

        let count = self.delete_ids("datasets", vec![id.to_string()]).await?;
        Ok(count > 0)
    }

    // --- Datapoint operations ---

    async fn save_datapoint(&self, dp: &Datapoint) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": dp.id.to_string(),
            "data": serde_json::to_string(dp)?,
            "dataset_id": dp.dataset_id.to_string(),
            "source": format!("{:?}", dp.source),
            "created_at": dp.created_at.to_rfc3339(),
        });

        let schema = serde_json::json!({"data": {"type": "string", "filterable": false}});
        self.upsert_with_schema("datapoints", vec![row], schema).await?;
        Ok(())
    }

    async fn get_datapoint(&self, id: DatapointId) -> Result<Option<Datapoint>, StorageError> {
        match self.get_by_id("datapoints", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_datapoints(&self, dataset_id: DatasetId) -> Result<Vec<Datapoint>, StorageError> {
        let filter = serde_json::json!(["dataset_id", "Eq", dataset_id.to_string()]);
        let results = self.query_all("datapoints", Some(filter)).await?;

        let mut datapoints = Vec::new();
        for row in results {
            if let Some(dp) = Self::extract_data::<Datapoint>(&row) {
                datapoints.push(dp);
            }
        }

        Ok(datapoints)
    }

    async fn list_datapoints_all(&self) -> Result<Vec<Datapoint>, StorageError> {
        let results = self.query_all("datapoints", None).await?;

        let mut datapoints = Vec::new();
        for row in results {
            if let Some(dp) = Self::extract_data::<Datapoint>(&row) {
                datapoints.push(dp);
            }
        }

        Ok(datapoints)
    }

    async fn delete_datapoint(&self, id: DatapointId) -> Result<bool, StorageError> {
        let count = self.delete_ids("datapoints", vec![id.to_string()]).await?;
        Ok(count > 0)
    }

    async fn delete_dataset_datapoints(
        &self,
        dataset_id: DatasetId,
    ) -> Result<usize, StorageError> {
        let datapoints = self.list_datapoints(dataset_id).await?;
        let ids: Vec<String> = datapoints.iter().map(|dp| dp.id.to_string()).collect();
        let count = ids.len();

        if !ids.is_empty() {
            self.delete_ids("datapoints", ids).await?;
        }

        Ok(count)
    }

    // --- Queue operations ---

    async fn save_queue_item(&self, item: &QueueItem) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": item.id.to_string(),
            "data": serde_json::to_string(item)?,
            "dataset_id": item.dataset_id.to_string(),
            "datapoint_id": item.datapoint_id.to_string(),
            "status": item.status.as_str(),
            "claimed_by": item.claimed_by,
            "created_at": item.created_at.to_rfc3339(),
        });

        let schema = serde_json::json!({"data": {"type": "string", "filterable": false}});
        self.upsert_with_schema("queue_items", vec![row], schema).await?;
        Ok(())
    }

    async fn get_queue_item(&self, id: QueueItemId) -> Result<Option<QueueItem>, StorageError> {
        match self.get_by_id("queue_items", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_queue_items(
        &self,
        dataset_id: DatasetId,
    ) -> Result<Vec<QueueItem>, StorageError> {
        let filter = serde_json::json!(["dataset_id", "Eq", dataset_id.to_string()]);
        let results = self.query_all("queue_items", Some(filter)).await?;

        let mut items = Vec::new();
        for row in results {
            if let Some(item) = Self::extract_data::<QueueItem>(&row) {
                items.push(item);
            }
        }

        Ok(items)
    }

    async fn list_queue_items_all(&self) -> Result<Vec<QueueItem>, StorageError> {
        let results = self.query_all("queue_items", None).await?;

        let mut items = Vec::new();
        for row in results {
            if let Some(item) = Self::extract_data::<QueueItem>(&row) {
                items.push(item);
            }
        }

        Ok(items)
    }

    async fn delete_queue_item(&self, id: QueueItemId) -> Result<bool, StorageError> {
        let count = self
            .delete_ids("queue_items", vec![id.to_string()])
            .await?;
        Ok(count > 0)
    }

    // --- Eval Run operations ---

    async fn save_eval_run(&self, run: &EvalRun) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": run.id.to_string(),
            "data": serde_json::to_string(run)?,
            "dataset_id": run.dataset_id.to_string(),
            "status": run.status.as_str(),
            "created_at": run.created_at.to_rfc3339(),
        });
        self.upsert("eval_runs", vec![row]).await?;
        Ok(())
    }

    async fn get_eval_run(&self, id: EvalRunId) -> Result<Option<EvalRun>, StorageError> {
        match self.get_by_id("eval_runs", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_eval_runs(&self, dataset_id: DatasetId) -> Result<Vec<EvalRun>, StorageError> {
        let filter = serde_json::json!(["dataset_id", "Eq", dataset_id.to_string()]);
        let results = self.query_all("eval_runs", Some(filter)).await?;
        let mut runs = Vec::new();
        for row in results {
            if let Some(run) = Self::extract_data::<EvalRun>(&row) {
                runs.push(run);
            }
        }
        Ok(runs)
    }

    async fn list_eval_runs_all(&self) -> Result<Vec<EvalRun>, StorageError> {
        let results = self.query_all("eval_runs", None).await?;
        let mut runs = Vec::new();
        for row in results {
            if let Some(run) = Self::extract_data::<EvalRun>(&row) {
                runs.push(run);
            }
        }
        Ok(runs)
    }

    async fn delete_eval_run(&self, id: EvalRunId) -> Result<bool, StorageError> {
        // Delete associated results first
        self.delete_eval_run_results(id).await?;
        let count = self.delete_ids("eval_runs", vec![id.to_string()]).await?;
        Ok(count > 0)
    }

    // --- Eval Result operations ---

    async fn save_eval_result(&self, result: &EvalResult) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": result.id.to_string(),
            "data": serde_json::to_string(result)?,
            "run_id": result.run_id.to_string(),
            "datapoint_id": result.datapoint_id.to_string(),
            "status": result.status.as_str(),
        });
        let schema = serde_json::json!({"data": {"type": "string", "filterable": false}});
        self.upsert_with_schema("eval_results", vec![row], schema).await?;
        Ok(())
    }

    async fn get_eval_result(&self, id: EvalResultId) -> Result<Option<EvalResult>, StorageError> {
        match self.get_by_id("eval_results", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_eval_results(&self, run_id: EvalRunId) -> Result<Vec<EvalResult>, StorageError> {
        let filter = serde_json::json!(["run_id", "Eq", run_id.to_string()]);
        let results = self.query_all("eval_results", Some(filter)).await?;
        let mut eval_results = Vec::new();
        for row in results {
            if let Some(r) = Self::extract_data::<EvalResult>(&row) {
                eval_results.push(r);
            }
        }
        Ok(eval_results)
    }

    async fn list_eval_results_all(&self) -> Result<Vec<EvalResult>, StorageError> {
        let results = self.query_all("eval_results", None).await?;
        let mut eval_results = Vec::new();
        for row in results {
            if let Some(r) = Self::extract_data::<EvalResult>(&row) {
                eval_results.push(r);
            }
        }
        Ok(eval_results)
    }

    async fn delete_eval_run_results(&self, run_id: EvalRunId) -> Result<usize, StorageError> {
        let results = self.list_eval_results(run_id).await?;
        let ids: Vec<String> = results.iter().map(|r| r.id.to_string()).collect();
        let count = ids.len();
        if !ids.is_empty() {
            self.delete_ids("eval_results", ids).await?;
        }
        Ok(count)
    }

    // --- Capture Rule operations ---

    async fn save_capture_rule(&self, rule: &CaptureRule) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": rule.id.to_string(),
            "data": serde_json::to_string(rule)?,
            "dataset_id": rule.dataset_id.to_string(),
            "enabled": rule.enabled,
            "created_at": rule.created_at.to_rfc3339(),
        });
        self.upsert("capture_rules", vec![row]).await?;
        Ok(())
    }

    async fn get_capture_rule(&self, id: CaptureRuleId) -> Result<Option<CaptureRule>, StorageError> {
        match self.get_by_id("capture_rules", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_capture_rules(&self, dataset_id: DatasetId) -> Result<Vec<CaptureRule>, StorageError> {
        let filter = serde_json::json!(["dataset_id", "Eq", dataset_id.to_string()]);
        let results = self.query_all("capture_rules", Some(filter)).await?;
        let mut rules = Vec::new();
        for row in results {
            if let Some(rule) = Self::extract_data::<CaptureRule>(&row) {
                rules.push(rule);
            }
        }
        Ok(rules)
    }

    async fn list_capture_rules_all(&self) -> Result<Vec<CaptureRule>, StorageError> {
        let results = self.query_all("capture_rules", None).await?;
        let mut rules = Vec::new();
        for row in results {
            if let Some(rule) = Self::extract_data::<CaptureRule>(&row) {
                rules.push(rule);
            }
        }
        Ok(rules)
    }

    async fn delete_capture_rule(&self, id: CaptureRuleId) -> Result<bool, StorageError> {
        let count = self.delete_ids("capture_rules", vec![id.to_string()]).await?;
        Ok(count > 0)
    }

    // --- Provider Connection operations ---

    async fn save_provider_connection(&self, conn: &ProviderConnection) -> Result<(), StorageError> {
        let row = serde_json::json!({
            "id": conn.id.to_string(),
            "data": serde_json::to_string(conn)?,
            "name": conn.name,
            "provider": conn.provider,
            "created_at": conn.created_at.to_rfc3339(),
            "updated_at": conn.updated_at.to_rfc3339(),
        });
        self.upsert("provider_connections", vec![row]).await?;
        Ok(())
    }

    async fn get_provider_connection(&self, id: ProviderConnectionId) -> Result<Option<ProviderConnection>, StorageError> {
        match self.get_by_id("provider_connections", &id.to_string()).await? {
            Some(row) => Ok(Self::extract_data(&row)),
            None => Ok(None),
        }
    }

    async fn list_provider_connections(&self) -> Result<Vec<ProviderConnection>, StorageError> {
        let results = self.query_all("provider_connections", None).await?;
        let mut conns = Vec::new();
        for row in results {
            if let Some(conn) = Self::extract_data::<ProviderConnection>(&row) {
                conns.push(conn);
            }
        }
        Ok(conns)
    }

    async fn delete_provider_connection(&self, id: ProviderConnectionId) -> Result<bool, StorageError> {
        let count = self.delete_ids("provider_connections", vec![id.to_string()]).await?;
        Ok(count > 0)
    }

    // --- File operations ---

    async fn save_file_version(&self, version: &FileVersion) -> Result<(), StorageError> {
        // Use path+hash as unique ID
        let id = format!("{}:{}", version.path, version.hash);
        let row = serde_json::json!({
            "id": id,
            "data": serde_json::to_string(version)?,
            "path": version.path,
            "hash": version.hash,
            "size": version.size,
            "created_at": version.created_at.to_rfc3339(),
        });

        self.upsert("file_versions", vec![row]).await?;
        Ok(())
    }

    async fn list_file_versions(&self) -> Result<Vec<FileVersion>, StorageError> {
        let results = self.query_all("file_versions", None).await?;

        let mut versions = Vec::new();
        for row in results {
            if let Some(version) = Self::extract_data::<FileVersion>(&row) {
                versions.push(version);
            }
        }

        Ok(versions)
    }

    async fn save_file_content(&self, hash: &str, content: &[u8]) -> Result<(), StorageError> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(content);
        let row = serde_json::json!({
            "id": hash,
            "content_base64": encoded,
            "size": content.len(),
        });

        let schema = serde_json::json!({"content_base64": {"type": "string", "filterable": false}});
        self.upsert_with_schema("file_contents", vec![row], schema).await?;
        Ok(())
    }

    async fn load_file_content(&self, hash: &str) -> Result<Vec<u8>, StorageError> {
        match self.get_by_id("file_contents", hash).await? {
            Some(row) => {
                let encoded = row
                    .get("content_base64")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| StorageError::Backend("Missing content_base64".to_string()))?;

                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|e| StorageError::Backend(format!("Base64 decode error: {}", e)))
            }
            None => Err(StorageError::NotFound),
        }
    }

    // --- Batch operations (optimized for cloud) ---

    async fn save_spans_batch(&self, spans: &[Span]) -> Result<(), StorageError> {
        if spans.is_empty() {
            return Ok(());
        }

        let rows: Result<Vec<_>, _> = spans
            .iter()
            .map(|span| {
                Ok(serde_json::json!({
                    "id": span.id().to_string(),
                    "data": serde_json::to_string(span)?,
                    "trace_id": span.trace_id().to_string(),
                    "name": span.name(),
                    "kind": span.kind().kind_name(),
                    "status": span.status().as_str(),
                    "model": span.kind().model(),
                    "provider": span.kind().provider(),
                    "started_at": span.started_at().to_rfc3339(),
                    "ended_at": span.ended_at().map(|t| t.to_rfc3339()),
                }))
            })
            .collect::<Result<Vec<_>, serde_json::Error>>();

        let schema = serde_json::json!({"data": {"type": "string", "filterable": false}});
        self.upsert_with_schema("spans", rows?, schema).await?;
        Ok(())
    }

    async fn save_datapoints_batch(&self, datapoints: &[Datapoint]) -> Result<(), StorageError> {
        if datapoints.is_empty() {
            return Ok(());
        }

        let rows: Result<Vec<_>, _> = datapoints
            .iter()
            .map(|dp| {
                Ok(serde_json::json!({
                    "id": dp.id.to_string(),
                    "data": serde_json::to_string(dp)?,
                    "dataset_id": dp.dataset_id.to_string(),
                    "source": format!("{:?}", dp.source),
                    "created_at": dp.created_at.to_rfc3339(),
                }))
            })
            .collect::<Result<Vec<_>, serde_json::Error>>();

        let schema = serde_json::json!({"data": {"type": "string", "filterable": false}});
        self.upsert_with_schema("datapoints", rows?, schema).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace() {
        let config = TurbopufferConfig::new("test_key", "org123");
        let backend = TurbopufferBackend::new(config).unwrap();
        assert_eq!(backend.namespace("spans"), "org123_spans");
        assert_eq!(backend.namespace("traces"), "org123_traces");
    }

    #[test]
    fn test_config_builder() {
        let config = TurbopufferConfig::new("key", "ns")
            .with_base_url("http://localhost:8080")
            .with_timeout(60);

        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.timeout_secs, 60);
    }
}
