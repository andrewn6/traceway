use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type SpanId = Uuid;
pub type TraceId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub id: SpanId,
    pub trace_id: TraceId,
    pub parent_id: Option<SpanId>,
    pub name: String,
    pub status: SpanStatus,
    pub metadata: SpanMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Running {
        started_at: DateTime<Utc>,
    },
    Completed{
       started_at: DateTime<Utc>,
       ended_at: DateTime<Utc>,
    },
    Failed {
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
        error: String,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpanMetadata {
    pub model: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

impl Span {
    pub fn new(trace_id: TraceId, parent_id: Option<SpanId>, name: impl Into<String>) -> Self {
        Self{
            id: Uuid::new_v4(),
            trace_id,
            parent_id,
            name: name.into(),
            status: SpanStatus::Running {
                started_at: Utc::now(),
            },
            metadata: SpanMetadata::default(),
        }
    }

    pub fn complete(&mut self) {
        if let SpanStatus::Running { started_at } = self.status {
            self.status = SpanStatus::Completed {
                started_at,
                ended_at: Utc::now(),
            };
        }
    }

}
