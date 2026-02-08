use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type SpanId = Uuid;
pub type TraceId = Uuid;

// --- SpanKind: typed span variants ---

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

// --- SpanStatus: simplified (timestamps live on Span) ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    id: SpanId,
    trace_id: TraceId,
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
    parent_id: Option<SpanId>,
    name: String,
    kind: SpanKind,
    input: Option<serde_json::Value>,
}

impl SpanBuilder {
    pub fn new(trace_id: TraceId, name: impl Into<String>, kind: SpanKind) -> Self {
        Self {
            trace_id,
            parent_id: None,
            name: name.into(),
            kind,
            input: None,
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub id: TraceId,
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
            name,
            tags: Vec::new(),
            started_at: Utc::now(),
            ended_at: None,
            machine_id: None,
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFile {
    pub path: String,
    pub current_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub hash: String,
    pub path: String,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_span: Option<SpanId>,
}

/// Compute SHA256 content hash (hex-encoded).
pub fn content_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
