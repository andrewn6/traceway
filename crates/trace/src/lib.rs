use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub type SpanId = Uuid;
pub type TraceId = Uuid;
pub type DatasetId = Uuid;
pub type DatapointId = Uuid;
pub type QueueItemId = Uuid;
pub type OrgId = Uuid;

// --- SpanKind: typed span variants ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SpanKind {
    FsRead {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_version: Option<String>,
        bytes_read: u64,
    },
    FsWrite {
        path: String,
        file_version: String,
        bytes_written: u64,
    },
    LlmCall {
        model: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        cost: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        input_preview: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_preview: Option<String>,
    },
    Custom {
        kind: String,
        #[serde(default)]
        attributes: HashMap<String, serde_json::Value>,
    },
}

impl SpanKind {
    pub fn kind_name(&self) -> &str {
        match self {
            SpanKind::FsRead { .. } => "fs_read",
            SpanKind::FsWrite { .. } => "fs_write",
            SpanKind::LlmCall { .. } => "llm_call",
            SpanKind::Custom { kind, .. } => kind,
        }
    }

    pub fn model(&self) -> Option<&str> {
        match self {
            SpanKind::LlmCall { model, .. } => Some(model),
            _ => None,
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            SpanKind::FsRead { path, .. } | SpanKind::FsWrite { path, .. } => Some(path),
            _ => None,
        }
    }

    pub fn provider(&self) -> Option<&str> {
        match self {
            SpanKind::LlmCall { provider, .. } => provider.as_deref(),
            _ => None,
        }
    }

    pub fn input_tokens(&self) -> Option<u64> {
        match self {
            SpanKind::LlmCall { input_tokens, .. } => *input_tokens,
            _ => None,
        }
    }

    pub fn output_tokens(&self) -> Option<u64> {
        match self {
            SpanKind::LlmCall { output_tokens, .. } => *output_tokens,
            _ => None,
        }
    }

    pub fn total_tokens(&self) -> Option<u64> {
        match (self.input_tokens(), self.output_tokens()) {
            (Some(i), Some(o)) => Some(i + o),
            (Some(i), None) => Some(i),
            (None, Some(o)) => Some(o),
            (None, None) => None,
        }
    }

    pub fn cost(&self) -> Option<f64> {
        match self {
            SpanKind::LlmCall { cost, .. } => *cost,
            _ => None,
        }
    }
}

// --- SpanStatus: simplified (timestamps live on Span) ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    Running,
    Completed,
    Failed { error: String },
}

impl SpanStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SpanStatus::Running => "running",
            SpanStatus::Completed => "completed",
            SpanStatus::Failed { .. } => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, SpanStatus::Completed | SpanStatus::Failed { .. })
    }
}

// --- Span: immutable after completion ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Span {
    #[schema(value_type = String)]
    id: SpanId,
    #[schema(value_type = String)]
    trace_id: TraceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    org_id: Option<OrgId>,
    #[schema(value_type = Option<String>)]
    parent_id: Option<SpanId>,
    name: String,
    kind: SpanKind,
    status: SpanStatus,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<serde_json::Value>,
}

// Read-only accessors
impl Span {
    pub fn id(&self) -> SpanId {
        self.id
    }

    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    pub fn org_id(&self) -> Option<OrgId> {
        self.org_id
    }

    pub fn parent_id(&self) -> Option<SpanId> {
        self.parent_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> &SpanKind {
        &self.kind
    }

    pub fn status(&self) -> &SpanStatus {
        &self.status
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    pub fn input(&self) -> Option<&serde_json::Value> {
        self.input.as_ref()
    }

    pub fn output(&self) -> Option<&serde_json::Value> {
        self.output.as_ref()
    }

    pub fn duration_ms(&self) -> Option<i64> {
        self.ended_at
            .map(|end| (end - self.started_at).num_milliseconds())
    }

    /// Transition from Running to Completed. No-op if already terminal.
    pub fn complete(self, output: Option<serde_json::Value>) -> Self {
        if self.status.is_terminal() {
            return self;
        }
        Span {
            status: SpanStatus::Completed,
            ended_at: Some(Utc::now()),
            output,
            ..self
        }
    }

    /// Transition from Running to Failed. No-op if already terminal.
    pub fn fail(self, error: impl Into<String>) -> Self {
        if self.status.is_terminal() {
            return self;
        }
        Span {
            status: SpanStatus::Failed {
                error: error.into(),
            },
            ended_at: Some(Utc::now()),
            ..self
        }
    }
}

// --- SpanBuilder ---

pub struct SpanBuilder {
    trace_id: TraceId,
    org_id: Option<OrgId>,
    parent_id: Option<SpanId>,
    name: String,
    kind: SpanKind,
    input: Option<serde_json::Value>,
}

impl SpanBuilder {
    pub fn new(trace_id: TraceId, name: impl Into<String>, kind: SpanKind) -> Self {
        Self {
            trace_id,
            org_id: None,
            parent_id: None,
            name: name.into(),
            kind,
            input: None,
        }
    }

    pub fn org(mut self, org_id: OrgId) -> Self {
        self.org_id = Some(org_id);
        self
    }

    pub fn parent(mut self, parent_id: SpanId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }

    pub fn build(self) -> Span {
        Span {
            id: Uuid::now_v7(),
            trace_id: self.trace_id,
            org_id: self.org_id,
            parent_id: self.parent_id,
            name: self.name,
            kind: self.kind,
            status: SpanStatus::Running,
            started_at: Utc::now(),
            ended_at: None,
            input: self.input,
            output: None,
        }
    }
}

// --- Trace: explicit trace-level metadata ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Trace {
    #[schema(value_type = String)]
    pub id: TraceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub org_id: Option<OrgId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_id: Option<String>,
}

impl Trace {
    pub fn new(name: Option<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            org_id: None,
            name,
            tags: Vec::new(),
            started_at: Utc::now(),
            ended_at: None,
            machine_id: None,
        }
    }

    pub fn with_org(mut self, org_id: OrgId) -> Self {
        self.org_id = Some(org_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn complete(mut self) -> Self {
        self.ended_at = Some(Utc::now());
        self
    }
}

// --- File model types ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackedFile {
    pub path: String,
    pub current_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileVersion {
    pub hash: String,
    pub path: String,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub created_by_span: Option<SpanId>,
}

/// Compute SHA256 content hash (hex-encoded).
pub fn content_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

// --- Dataset types ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DatapointKind {
    LlmConversation {
        messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected: Option<Message>,
        #[serde(default)]
        metadata: HashMap<String, serde_json::Value>,
    },
    Generic {
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_output: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        actual_output: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        score: Option<f64>,
        #[serde(default)]
        metadata: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DatapointSource {
    Manual,
    SpanExport,
    FileUpload,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Dataset {
    #[schema(value_type = String)]
    pub id: DatasetId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub org_id: Option<OrgId>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Dataset {
    pub fn new(name: impl Into<String>, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            org_id: None,
            name: name.into(),
            description,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_org(mut self, org_id: OrgId) -> Self {
        self.org_id = Some(org_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Datapoint {
    #[schema(value_type = String)]
    pub id: DatapointId,
    #[schema(value_type = String)]
    pub dataset_id: DatasetId,
    pub kind: DatapointKind,
    pub source: DatapointSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub source_span_id: Option<SpanId>,
    pub created_at: DateTime<Utc>,
}

impl Datapoint {
    pub fn new(dataset_id: DatasetId, kind: DatapointKind, source: DatapointSource) -> Self {
        Self {
            id: Uuid::now_v7(),
            dataset_id,
            kind,
            source,
            source_span_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_source_span(mut self, span_id: SpanId) -> Self {
        self.source_span_id = Some(span_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueueItemStatus {
    Pending,
    Claimed,
    Completed,
}

impl QueueItemStatus {
    pub fn as_str(&self) -> &str {
        match self {
            QueueItemStatus::Pending => "pending",
            QueueItemStatus::Claimed => "claimed",
            QueueItemStatus::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueueItem {
    #[schema(value_type = String)]
    pub id: QueueItemId,
    #[schema(value_type = String)]
    pub dataset_id: DatasetId,
    #[schema(value_type = String)]
    pub datapoint_id: DatapointId,
    pub status: QueueItemStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl QueueItem {
    pub fn new(
        dataset_id: DatasetId,
        datapoint_id: DatapointId,
        original_data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            dataset_id,
            datapoint_id,
            status: QueueItemStatus::Pending,
            claimed_by: None,
            claimed_at: None,
            original_data,
            edited_data: None,
            created_at: Utc::now(),
        }
    }

    pub fn claim(mut self, claimed_by: impl Into<String>) -> Self {
        self.status = QueueItemStatus::Claimed;
        self.claimed_by = Some(claimed_by.into());
        self.claimed_at = Some(Utc::now());
        self
    }

    pub fn complete(mut self, edited_data: Option<serde_json::Value>) -> Self {
        self.status = QueueItemStatus::Completed;
        self.edited_data = edited_data;
        self
    }
}

// --- Analytics types ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsQuery {
    pub metrics: Vec<AnalyticsMetric>,
    #[serde(default)]
    pub group_by: Vec<GroupByField>,
    #[serde(default)]
    pub filter: AnalyticsFilter,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsMetric {
    TotalCost,
    TotalInputTokens,
    TotalOutputTokens,
    TotalTokens,
    AvgLatencyMs,
    SpanCount,
    ErrorCount,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupByField {
    Model,
    Provider,
    Kind,
    Status,
    Trace,
    Day,
    Hour,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsFilter {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
    #[serde(default)]
    pub until: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub trace_id: Option<TraceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsResponse {
    pub groups: Vec<AnalyticsGroup>,
    pub totals: MetricValues,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsGroup {
    pub key: HashMap<String, String>,
    pub metrics: MetricValues,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct MetricValues {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsSummary {
    pub total_traces: usize,
    pub total_spans: usize,
    pub total_llm_calls: usize,
    pub total_cost: f64,
    pub total_tokens: u64,
    pub avg_latency_ms: f64,
    pub error_count: usize,
    pub models_used: Vec<String>,
    pub providers_used: Vec<String>,
    pub cost_by_model: Vec<ModelCost>,
    pub tokens_by_model: Vec<ModelTokens>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelCost {
    pub model: String,
    pub cost: f64,
    pub span_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelTokens {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}
