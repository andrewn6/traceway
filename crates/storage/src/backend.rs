use async_trait::async_trait;
use thiserror::Error;
use trace::{FileVersion, Span, SpanId, Trace, TraceId};

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
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // Span operations
    async fn load_all_spans(&self) -> Result<Vec<Span>, StorageError>;
    async fn save_span(&self, span: &Span) -> Result<(), StorageError>;
    async fn delete_span(&self, id: SpanId) -> Result<bool, StorageError>;
    async fn delete_trace_spans(&self, trace_id: TraceId) -> Result<usize, StorageError>;
    async fn clear_spans(&self) -> Result<(), StorageError>;

    // Trace operations
    async fn load_all_traces(&self) -> Result<Vec<Trace>, StorageError>;
    async fn save_trace(&self, trace: &Trace) -> Result<(), StorageError>;
    async fn delete_trace(&self, trace_id: TraceId) -> Result<bool, StorageError>;

    // File operations
    async fn load_all_files(&self) -> Result<Vec<FileVersion>, StorageError>;
    async fn save_file_version(&self, version: &FileVersion) -> Result<(), StorageError>;
    async fn save_file_content(&self, hash: &str, content: &[u8]) -> Result<(), StorageError>;
    async fn load_file_content(&self, hash: &str) -> Result<Vec<u8>, StorageError>;
}
