use async_trait::async_trait;
use thiserror::Error;
use trace::{Span, SpanId, TraceId};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("not found")]
    NotFound,
}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}

/// Trait for pluggable storage backends.
///
/// Implementations can store spans in SQLite, Turso, S3, or any other backend.
/// All methods are async to support both local and remote storage.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Load all spans from storage.
    async fn load_all(&self) -> Result<Vec<Span>, StorageError>;

    /// Save a span (insert or update).
    async fn save_span(&self, span: &Span) -> Result<(), StorageError>;

    /// Delete a span by ID. Returns true if the span existed.
    async fn delete_span(&self, id: SpanId) -> Result<bool, StorageError>;

    /// Delete all spans for a trace. Returns number of spans deleted.
    async fn delete_trace(&self, trace_id: TraceId) -> Result<usize, StorageError>;

    /// Delete all spans.
    async fn clear(&self) -> Result<(), StorageError>;
}
