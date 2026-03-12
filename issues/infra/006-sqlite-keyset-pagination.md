# Implement Keyset Pagination for SQLite Backend

**Labels:** `enhancement`, `backend`, `PRD-06`
**Difficulty:** Medium
**PRD:** [PRD-06: Pagination](../../prds/PRD-06-pagination.md) — Phase 4
**Depends on:** #001

## Summary

Implement `list_*_paged()` methods on `SqliteBackend` using keyset pagination (not OFFSET). Keyset pagination uses a `WHERE (col, id) < (?, ?)` clause which maintains constant performance regardless of page depth.

## Context

The `Pagination` and `Page<T>` types from issue #001 need a concrete SQLite implementation. SQLite is the default backend for local mode.

Currently all `list_*` methods do unbounded `SELECT` queries. The new `_paged` variants should use the cursor to perform efficient keyset pagination.

## What to do

1. Add `_paged` method signatures to the `StorageBackend` trait in `crates/storage/src/backend.rs`:
   ```rust
   async fn list_traces_paged(&self, filter: &TraceFilter, page: &Pagination) -> Result<Page<Trace>, StorageError>;
   async fn list_spans_paged(&self, filter: &SpanFilter, page: &Pagination) -> Result<Page<Span>, StorageError>;
   async fn list_datapoints_paged(&self, dataset_id: DatasetId, page: &Pagination) -> Result<Page<Datapoint>, StorageError>;
   async fn list_eval_results_paged(&self, run_id: EvalRunId, page: &Pagination) -> Result<Page<EvalResult>, StorageError>;
   ```
   Provide default implementations that delegate to the existing `list_*` methods (truncated to page size) so Turbopuffer doesn't break.

2. Implement in `crates/storage-sqlite/src/lib.rs` using keyset pagination:
   ```sql
   -- First page
   SELECT data FROM spans WHERE trace_id = ? ORDER BY started_at DESC, id DESC LIMIT ?;
   -- With cursor
   SELECT data FROM spans WHERE trace_id = ? AND (started_at, id) < (?, ?) ORDER BY started_at DESC, id DESC LIMIT ?;
   ```

3. Fetch `limit + 1` rows to detect `has_more`, then truncate to `limit`.

4. Build the `next_cursor` from the last row's sort key + ID using the cursor encoding from #001.

## Files to modify

- `crates/storage/src/backend.rs` — add trait methods with defaults
- `crates/storage-sqlite/src/lib.rs` — implement keyset pagination
- `crates/api/src/any_backend.rs` — add delegate for new methods

## Acceptance criteria

- [ ] First page returns correct items with `has_more: true` when more exist
- [ ] Cursor-based second page returns the next set (no duplicates, no gaps)
- [ ] Empty cursor returns first page
- [ ] Works with all filter combinations (trace_id, status, kind, etc.)
- [ ] `cargo check` succeeds for all storage crates
- [ ] Unit test: paginate through 100 spans in pages of 10, verify all 100 are returned exactly once
