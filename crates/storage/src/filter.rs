use chrono::{DateTime, Utc};
use trace::{DatasetId, TraceId};

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
