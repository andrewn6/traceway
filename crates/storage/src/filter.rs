use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use trace::{DatasetId, TraceId};

use crate::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pagination {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub sort_order: Option<SortOrder>,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Desc
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: Option<usize>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CursorInner {
    pub sort_field: String,
    pub last_value: String,
    pub last_id: String,
}

/// Filter for querying traces.
#[derive(Debug, Default, Clone)]
pub struct TraceFilter {
    pub name_contains: Option<String>,
    pub tags: Option<Vec<String>>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

/// Filter for querying spans.
#[derive(Debug, Default, Clone)]
pub struct SpanFilter {
    pub kind: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub status: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub name_contains: Option<String>,
    pub path: Option<String>,
    pub trace_id: Option<TraceId>,
    pub limit: Option<usize>,
    /// Minimum duration in milliseconds (inclusive)
    pub duration_min: Option<i64>,
    /// Maximum duration in milliseconds (inclusive)
    pub duration_max: Option<i64>,
    /// Minimum total tokens (inclusive)
    pub tokens_min: Option<u64>,
    /// Minimum cost in dollars (inclusive)
    pub cost_min: Option<f64>,
    /// Full-text search across span input and output content (case-insensitive)
    pub text_contains: Option<String>,
    /// Full-text search within span input content only (case-insensitive)
    pub input_contains: Option<String>,
    /// Full-text search within span output content only (case-insensitive)
    pub output_contains: Option<String>,
    /// Field to sort by: "started_at", "duration", "tokens", "cost", "name"
    pub sort_by: Option<String>,
    /// Sort direction: "asc" or "desc" (default: "desc")
    pub sort_order: Option<String>,
}

/// Filter for querying files.
#[derive(Debug, Default, Clone)]
pub struct FileFilter {
    pub path_prefix: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub associated_trace: Option<TraceId>,
}

/// Filter for querying datapoints.
#[derive(Debug, Default, Clone)]
pub struct DatapointFilter {
    pub dataset_id: Option<DatasetId>,
    pub source: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

pub fn encode_cursor(inner: &CursorInner) -> String {
    let json = serde_json::to_string(inner).expect("CursorInner is always serializable");
    STANDARD.encode(json.as_bytes())
}

pub fn decode_cursor(cursor: &str) -> Result<CursorInner, StorageError> {
    let bytes = STANDARD
        .decode(cursor)
        .map_err(|e| StorageError::Serialization(format!("invalid cursor base64: {e}")))?;
    let json = String::from_utf8(bytes)
        .map_err(|e| StorageError::Serialization(format!("invalid cursor utf8: {e}")))?;
    serde_json::from_str(&json)
        .map_err(|e| StorageError::Serialization(format!("invalid cursor json: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trip() {
        let inner = CursorInner {
            sort_field: "started_at".into(),
            last_value: "2025-01-01T00:00:00Z".into(),
            last_id: "abc-123".into(),
        };
        let encoded = encode_cursor(&inner);
        let decoded = decode_cursor(&encoded).unwrap();
        assert_eq!(decoded.sort_field, inner.sort_field);
        assert_eq!(decoded.last_value, inner.last_value);
        assert_eq!(decoded.last_id, inner.last_id);
    }

    #[test]
    fn decode_invalid_cursor() {
        assert!(decode_cursor("not-valid-base64!!!").is_err());
        let not_json = STANDARD.encode(b"not json");
        assert!(decode_cursor(&not_json).is_err());
    }
}
