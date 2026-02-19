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
