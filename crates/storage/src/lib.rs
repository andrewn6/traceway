use std::collections::HashMap;

use trace::{Span, SpanId, TraceId};

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
}
