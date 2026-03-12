# Remove sync_from_backend on Every List Request

**Labels:** `bug`, `backend`
**Difficulty:** Medium
**Priority:** P1

## Summary

`PersistentStore` calls `sync_from_backend()` on every list request (e.g., listing traces, spans, datasets). This performs a full table scan of the backend and replaces the entire in-memory cache. On a project with 10K+ records, every API list call triggers a full data reload — O(n) per request.

## Impact

- List endpoints become progressively slower as data grows
- Each list request generates backend I/O proportional to total data size (not page size)
- Multiple concurrent list requests cause redundant full scans
- For Turbopuffer (cloud mode), this means a full remote fetch on every page view

## Where

`crates/storage/src/lib.rs` — the `sync_from_backend()` method and its call sites in `list_*` methods.

## What to do

1. **Remove `sync_from_backend()` from list methods entirely.** The cache should be populated at startup and kept in sync via write-through. List methods should read from cache only.

2. If cache-miss scenarios are a concern (e.g., multi-instance deployment where another instance wrote data), implement a lightweight staleness check:
   - Track a `last_synced_at` timestamp per entity type
   - Only re-sync if the cache hasn't been synced in the last N seconds (e.g., 30s)
   - Or rely on the conditional caching work in #010

3. For cloud mode (Turbopuffer), list methods should query the backend directly with pagination — not load everything into memory.

## Acceptance criteria

- [ ] List endpoints do not trigger full backend scans
- [ ] List performance is O(page_size), not O(total_records)
- [ ] `cargo check` passes
