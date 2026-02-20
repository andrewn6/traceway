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
    Datapoint, DatapointId, Dataset, DatasetId, FileVersion, QueueItem, QueueItemId, Span, SpanId,
    Trace, TraceId,
};
use tracing::{debug, info, instrument};

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
            .unwrap_or_else(|_| "https://api.turbopuffer.com".to_string());

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
            base_url: "https://api.turbopuffer.com".to_string(),
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
}

/// Row-based upsert request for Turbopuffer v2 API
#[derive(Debug, Serialize)]
struct UpsertRequest {
    upsert_rows: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance_metric: Option<String>,
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
        };

        let _: serde_json::Value = self.post(&path, &req).await?;
        Ok(())
    }

    /// Query documents from a namespace
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

        let resp: QueryResponse = self.post(&path, &req).await?;
        Ok(resp.rows)
    }

    /// Delete documents by ID
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

        let _: serde_json::Value = self.post(&path, &req).await?;
        Ok(count)
    }

    /// Get a single document by ID
    async fn get_by_id(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<Option<serde_json::Value>, TurbopufferError> {
        let filter = serde_json::json!(["id", "Eq", id]);
        let results = self.query(collection, Some(filter), 1).await?;
        Ok(results.into_iter().next())
    }

    /// Extract data field from a row
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

        let limit = filter.limit.unwrap_or(1000);
        let results = self.query("traces", filters, limit).await?;

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

        self.upsert("spans", vec![row]).await?;
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

        let limit = filter.limit.unwrap_or(10000);
        let results = self.query("spans", filters, limit).await?;

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
        let results = self.query("datasets", None, 1000).await?;

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

        self.upsert("datapoints", vec![row]).await?;
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
        let results = self.query("datapoints", Some(filter), 10000).await?;

        let mut datapoints = Vec::new();
        for row in results {
            if let Some(dp) = Self::extract_data::<Datapoint>(&row) {
                datapoints.push(dp);
            }
        }

        Ok(datapoints)
    }

    async fn list_datapoints_all(&self) -> Result<Vec<Datapoint>, StorageError> {
        let results = self.query("datapoints", None, 10000).await?;

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

        self.upsert("queue_items", vec![row]).await?;
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
        let results = self.query("queue_items", Some(filter), 10000).await?;

        let mut items = Vec::new();
        for row in results {
            if let Some(item) = Self::extract_data::<QueueItem>(&row) {
                items.push(item);
            }
        }

        Ok(items)
    }

    async fn list_queue_items_all(&self) -> Result<Vec<QueueItem>, StorageError> {
        let results = self.query("queue_items", None, 10000).await?;

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
        let results = self.query("file_versions", None, 10000).await?;

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

        self.upsert("file_contents", vec![row]).await?;
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

        self.upsert("spans", rows?).await?;
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

        self.upsert("datapoints", rows?).await?;
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
