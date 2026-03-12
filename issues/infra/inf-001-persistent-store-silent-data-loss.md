# Fix PersistentStore Silent Data Loss on Backend Write Failure

**Labels:** `bug`, `backend`
**Difficulty:** Medium
**Priority:** P0 — Critical

## Summary

`PersistentStore` uses a write-through pattern: write to the in-memory HashMap cache, then async-persist to the backend (SQLite/Turbopuffer). If the backend write fails, the error is logged but **the cache still contains the data**, so the API returns success to the client. On restart, the data is gone — it was never persisted.

## Impact

Users think their data was saved (API returned 200), but it silently vanished on the next server restart. This is a data loss bug.

## Where

`crates/storage/src/lib.rs` — every `save_*` method follows this pattern:

```rust
// 1. Write to in-memory cache
self.traces.insert(trace.id.clone(), trace.clone());
// 2. Try to persist — error is swallowed
if let Err(e) = self.backend.save_trace(&trace).await {
    tracing::error!("Failed to persist trace: {e}");
    // No rollback of cache! No error returned to caller!
}
```

## What to do

1. **Fail the entire operation if backend write fails.** Remove the `if let Err(e)` pattern and propagate the error with `?`. The cache write should only happen *after* the backend write succeeds (write-behind → write-through with rollback).

2. Alternatively, reverse the order: write to backend first, then insert into cache. If backend fails, return error to caller, cache is never updated.

3. Audit every `save_*`, `delete_*`, and `update_*` method in `PersistentStore` for this pattern. There are approximately 20+ methods affected.

## Acceptance criteria

- [ ] Backend write failure returns an error to the API handler
- [ ] API handler returns 500 to the client (not 200)
- [ ] Cache is not updated if backend write fails
- [ ] Existing tests still pass
- [ ] `cargo check` passes
