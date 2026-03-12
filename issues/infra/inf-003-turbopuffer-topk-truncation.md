# Fix Turbopuffer top_k Truncation Silently Dropping Data

**Labels:** `bug`, `backend`
**Difficulty:** Medium
**Priority:** P0 — Critical

## Summary

The Turbopuffer backend uses `top_k: 10000` in queries to fetch all records from a namespace. If a namespace has more than 10,000 records (e.g., 10K+ spans in a project), the query silently returns only the first 10,000 — the rest are invisible. There is no warning, no pagination, and no error.

## Impact

Cloud-mode users with more than 10K spans, traces, or datapoints per project silently lose visibility into older data. The UI appears to work but is missing records. This is a silent data truncation bug.

## Where

`crates/storage-turbopuffer/src/lib.rs` — every `list_*` method passes `top_k: 10000` to the Turbopuffer query API.

## What to do

1. **Implement cursor-based pagination for Turbopuffer queries.** Use Turbopuffer's pagination support (if available) to fetch results in batches until exhausted.

2. If Turbopuffer doesn't support cursor pagination natively, use a keyset approach: order by a sortable field (e.g., `started_at` or `id`), fetch `top_k: 10000`, then re-query with a `WHERE id > last_seen_id` filter to get the next batch.

3. For the `sync_from_backend()` method specifically, this is even more critical since it's meant to load ALL data into cache.

4. Add a warning log if a query returns exactly `top_k` results (likely truncated):

```rust
if results.len() == TOP_K {
    tracing::warn!("Turbopuffer query returned exactly {TOP_K} results — data may be truncated");
}
```

## Acceptance criteria

- [ ] Queries that exceed 10K results fetch all pages
- [ ] Warning logged when potential truncation detected
- [ ] `sync_from_backend()` loads all records regardless of count
- [ ] `cargo check -p storage-turbopuffer` passes
