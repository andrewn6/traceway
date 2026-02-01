use std::collections::HashMap;

use trace::{Span, SpanId, TraceId};

/// In-memory span store with dual indexes for fast lookup.
/// Primary: SpanId -> Span
/// Secondary: TraceId -> Vec<SpanId> for listing all spans in a trace
#[derive(Debug, Default)]
pub struct SpanStore {
    spans: HashMap<SpanId, Span>,
    traces: HashMap<TraceId, Vec<SpanId>>,
}

impl SpanStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new span. Returns the span ID.
    pub fn insert(&mut self, span: Span) -> SpanId {
        let id = span.id;
        let trace_id = span.trace_id;

        self.spans.insert(id, span);
        self.traces.entry(trace_id).or_default().push(id);

        id
    }

    /// Get a span by ID.
    pub fn get(&self, id: SpanId) -> Option<&Span> {
        self.spans.get(&id)
    }

    /// Get a mutable reference to a span by ID.
    pub fn get_mut(&mut self, id: SpanId) -> Option<&mut Span> {
        self.spans.get_mut(&id)
    }

    /// List all span IDs for a given trace.
    pub fn spans_for_trace(&self, trace_id: TraceId) -> &[SpanId] {
        self.traces.get(&trace_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// List all trace IDs.
    pub fn trace_ids(&self) -> impl Iterator<Item = &TraceId> {
        self.traces.keys()
    }

    /// List all spans (immutable).
    pub fn all_spans(&self) -> impl Iterator<Item = &Span> {
        self.spans.values()
    }

    /// Complete a span by ID. Returns true if the span was found and updated.
    pub fn complete(&mut self, id: SpanId) -> bool {
        if let Some(span) = self.spans.get_mut(&id) {
            span.complete();
            true
        } else {
            false
        }
    }

    /// Fail a span by ID with an error message. Returns true if the span was found and updated.
    pub fn fail(&mut self, id: SpanId, error: impl Into<String>) -> bool {
        if let Some(span) = self.spans.get_mut(&id) {
            span.fail(error);
            true
        } else {
            false
        }
    }

    /// Total number of spans stored.
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Total number of traces.
    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace::SpanStatus;
    use uuid::Uuid;

    #[test]
    fn insert_and_retrieve() {
        let mut store = SpanStore::new();
        let trace_id = Uuid::new_v4();
        let span = Span::new(trace_id, None, "test-span");
        let id = span.id;

        store.insert(span);

        assert_eq!(store.span_count(), 1);
        assert_eq!(store.trace_count(), 1);

        let retrieved = store.get(id).unwrap();
        assert_eq!(retrieved.name, "test-span");
        assert_eq!(retrieved.trace_id, trace_id);
    }

    #[test]
    fn complete_span() {
        let mut store = SpanStore::new();
        let trace_id = Uuid::new_v4();
        let span = Span::new(trace_id, None, "work");
        let id = span.id;
        store.insert(span);

        assert!(store.complete(id));

        let span = store.get(id).unwrap();
        assert!(matches!(span.status, SpanStatus::Completed { .. }));
    }

    #[test]
    fn fail_span() {
        let mut store = SpanStore::new();
        let trace_id = Uuid::new_v4();
        let span = Span::new(trace_id, None, "risky");
        let id = span.id;
        store.insert(span);

        assert!(store.fail(id, "something went wrong"));

        let span = store.get(id).unwrap();
        match &span.status {
            SpanStatus::Failed { error, .. } => {
                assert_eq!(error, "something went wrong");
            }
            _ => panic!("expected Failed status"),
        }
    }

    #[test]
    fn spans_for_trace() {
        let mut store = SpanStore::new();
        let trace_id = Uuid::new_v4();

        let s1 = Span::new(trace_id, None, "parent");
        let parent_id = s1.id;
        store.insert(s1);

        let s2 = Span::new(trace_id, Some(parent_id), "child");
        store.insert(s2);

        let span_ids = store.spans_for_trace(trace_id);
        assert_eq!(span_ids.len(), 2);
    }
}
