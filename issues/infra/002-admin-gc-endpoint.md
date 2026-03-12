# Add Admin GC Endpoint (Dry-Run Orphan Detection)

**Labels:** `good first issue`, `backend`, `PRD-09`
**Difficulty:** Easy
**PRD:** [PRD-09: Orphan Cleanup](../../prds/PRD-09-orphan-cleanup.md) — Phase 5

## Summary

Add a `POST /admin/gc?dry_run=true` API endpoint that scans for orphaned records and returns a report without deleting anything. This is the read-only, safe entry point into the GC system.

## Context

Orphaned records occur when a parent entity is deleted but its children survive (e.g., eval results whose eval run was deleted). Today there's no way to detect these. This endpoint gives operators visibility.

## What to do

1. Create a new file `crates/api/src/gc.rs`
2. Implement `detect_orphans()` that checks for:
   - Spans whose `trace_id` doesn't match any existing trace
   - Datapoints whose `dataset_id` doesn't match any existing dataset
   - Queue items whose `dataset_id` doesn't match any existing dataset
   - Eval runs whose `dataset_id` doesn't match any existing dataset
   - Eval results whose `run_id` doesn't match any existing eval run
   - Capture rules whose `dataset_id` doesn't match any existing dataset
3. Return an `OrphanReport` as JSON
4. Register the route in `crates/api/src/lib.rs`

```rust
#[derive(Debug, Serialize)]
pub struct OrphanReport {
    pub orphaned_spans: usize,
    pub orphaned_datapoints: usize,
    pub orphaned_queue_items: usize,
    pub orphaned_eval_runs: usize,
    pub orphaned_eval_results: usize,
    pub orphaned_capture_rules: usize,
    pub total: usize,
}
```

The endpoint should require auth (use existing `auth::Auth(ctx)` pattern).

## Files to modify

- `crates/api/src/gc.rs` — **new file**
- `crates/api/src/lib.rs` — add route, add `mod gc;`

## Acceptance criteria

- [ ] `POST /admin/gc?dry_run=true` returns JSON `OrphanReport`
- [ ] Correctly identifies orphans (write a test that creates an orphaned eval result and verifies detection)
- [ ] Does NOT delete anything when `dry_run=true`
- [ ] Compiles: `cargo check -p api`
