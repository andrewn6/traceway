use std::collections::HashMap;

use chrono::{DateTime, Utc};
use trace::{Span, SpanId, SpanStatus, TraceId};

/// Filter criteria for querying spans.
#[derive(Debug, Default, Clone)]
pub struct SpanFilter {
    pub model: Option<String>,
    pub status: Option<String>, // "running", "completed", "failed"
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub name_contains: Option<String>,
}

/// In-memory span store with dual indexes for fast lookup.
#[derive(Debug, Default)]
pub struct SpanStore {
    spans: HashMap<SpanId, Span>,
    traces: HashMap<TraceId, Vec<SpanId>>,
}

impl SpanStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, span: Span) -> SpanId {
        let id = span.id;
        let trace_id = span.trace_id;
        self.spans.insert(id, span);
        self.traces.entry(trace_id).or_default().push(id);
        id
    }

    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.spans.get(&id)
    }

    pub fn get_mut(&mut self, id: SpanId) -> Option<&mut Span> {
        self.spans.get_mut(&id)
    }

    pub fn spans_for_trace(&self, trace_id: TraceId) -> &[SpanId] {
        self.traces
            .get(&trace_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn trace_ids(&self) -> impl Iterator<Item = &TraceId> {
        self.traces.keys()
    }

    pub fn all_spans(&self) -> impl Iterator<Item = &Span> {
        self.spans.values()
    }

    pub fn complete(&mut self, id: SpanId) -> bool {
        if let Some(span) = self.spans.get_mut(&id) {
            span.complete();
            true
        } else {
            false
        }
    }

    pub fn fail(&mut self, id: SpanId, error: impl Into<String>) -> bool {
        if let Some(span) = self.spans.get_mut(&id) {
            span.fail(error);
            true
        } else {
            false
        }
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }

    /// Delete a single span by ID. Returns true if the span was deleted.
    pub fn delete_span(&mut self, id: SpanId) -> bool {
        if let Some(span) = self.spans.remove(&id) {
            // Remove from trace index
            if let Some(span_ids) = self.traces.get_mut(&span.trace_id) {
                span_ids.retain(|&sid| sid != id);
                // Clean up empty trace entry
                if span_ids.is_empty() {
                    self.traces.remove(&span.trace_id);
                }
            }
            true
        } else {
            false
        }
    }

    /// Delete all spans for a trace. Returns the number of spans deleted.
    pub fn delete_trace(&mut self, trace_id: TraceId) -> usize {
        if let Some(span_ids) = self.traces.remove(&trace_id) {
            let count = span_ids.len();
            for id in span_ids {
                self.spans.remove(&id);
            }
            count
        } else {
            0
        }
    }

    /// Delete all spans and traces.
    pub fn clear(&mut self) {
        self.spans.clear();
        self.traces.clear();
    }

    /// Filter spans by criteria.
    pub fn filter_spans(&self, filter: &SpanFilter) -> Vec<&Span> {
        self.spans
            .values()
            .filter(|span| {
                // Filter by model
                if let Some(ref model) = filter.model {
                    match &span.metadata.model {
                        Some(m) if m == model => {}
                        _ => return false,
                    }
                }

                // Filter by status
                if let Some(ref status) = filter.status {
                    let span_status = match &span.status {
                        SpanStatus::Running { .. } => "running",
                        SpanStatus::Completed { .. } => "completed",
                        SpanStatus::Failed { .. } => "failed",
                    };
                    if span_status != status {
                        return false;
                    }
                }

                // Get started_at from span status
                let started_at = match &span.status {
                    SpanStatus::Running { started_at } => *started_at,
                    SpanStatus::Completed { started_at, .. } => *started_at,
                    SpanStatus::Failed { started_at, .. } => *started_at,
                };

                // Filter by since
                if let Some(since) = filter.since {
                    if started_at < since {
                        return false;
                    }
                }

                // Filter by until
                if let Some(until) = filter.until {
                    if started_at > until {
                        return false;
                    }
                }

                // Filter by name contains
                if let Some(ref name_contains) = filter.name_contains {
                    if !span.name.contains(name_contains) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}
