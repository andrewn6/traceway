# Span-Level Annotations and Feedback Scores

**Labels:** `enhancement`, `backend`, `frontend`
**Difficulty:** Medium
**Priority:** Medium

## Summary

Let users attach numeric scores and text notes to individual spans — thumbs up/down, quality ratings (1-5), free-text comments. This is essential for building feedback loops: users rate model outputs, and the scores can be used to filter for good/bad examples, build evaluation datasets, and track quality over time.

## Context

Traceway already has a labeling queue system for human review, but there's no way to annotate a span inline while viewing a trace. If you're reviewing a trace and see a bad LLM output, you should be able to immediately score it without navigating to a separate queue page. The annotation data feeds into the same pipeline — training data, eval baselines, quality dashboards.

## What to do

### 1. Data model

Add an `Annotation` type to `crates/trace/src/lib.rs`:

```rust
pub struct Annotation {
    pub id: AnnotationId,
    pub span_id: SpanId,
    pub trace_id: TraceId,
    pub org_id: OrgId,
    pub score: Option<f64>,        // numeric score (0.0 - 1.0, or custom range)
    pub label: Option<String>,     // e.g., "good", "bad", "hallucination"
    pub comment: Option<String>,   // free-text note
    pub created_by: Option<String>, // user who annotated
    pub created_at: DateTime<Utc>,
}
```

### 2. Storage

Add to `StorageBackend` trait:
- `save_annotation(annotation: &Annotation)`
- `get_annotations_for_span(span_id: &SpanId) -> Vec<Annotation>`
- `get_annotations_for_trace(trace_id: &TraceId) -> Vec<Annotation>`
- `delete_annotation(id: &AnnotationId)`

### 3. API

- `POST /api/spans/:id/annotate` — Create annotation
- `GET /api/spans/:id/annotations` — List annotations for a span
- `DELETE /api/annotations/:id` — Delete annotation
- `POST /api/traces/:id/annotate` — Annotate at trace level (attaches to root span)

### 4. SDK

Add a post-hoc annotation API to both SDKs:

```python
client.annotate(span_id="...", score=0.9, label="good", comment="Correct answer")
```

### 5. Frontend: Inline annotation

In `SpanDetail.svelte`, add an annotation section below the existing tabs:

- **Quick score**: Row of buttons: thumbs up (1.0) / thumbs down (0.0), or a 1-5 star rating
- **Label**: Dropdown with common labels ("good", "bad", "hallucination", "off-topic") + custom input
- **Comment**: Optional text input
- **Submit** button

Show existing annotations below the input as a list with score, label, comment, author, timestamp.

### 6. Query DSL

Add `score:>0.5`, `label:hallucination` filter types so users can find annotated spans.

## Files to modify

- `crates/trace/src/lib.rs` — Annotation type
- `crates/storage/src/backend.rs` — Trait methods
- `crates/storage-sqlite/src/lib.rs` — SQLite implementation
- `crates/storage-turbopuffer/src/lib.rs` — Turbopuffer implementation
- `crates/api/src/lib.rs` — API routes
- `ui/src/lib/components/SpanDetail.svelte` — Annotation UI
- `ui/src/lib/api.ts` — Annotation API functions
- `ui/src/lib/query-dsl.ts` — score/label filters
- SDKs — annotate method

## Acceptance criteria

- [ ] Users can score and annotate spans from the trace detail page
- [ ] Annotations stored and retrieved per span
- [ ] Quick thumbs up/down works with one click
- [ ] Existing annotations display below the input
- [ ] `score:` and `label:` work in query DSL
- [ ] `cargo check` and `npm run build` pass
