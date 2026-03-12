# Memory-Bounded PersistentStore with LRU Eviction

**Labels:** `enhancement`, `backend`
**Difficulty:** Hard
**Priority:** P1

## Summary

`PersistentStore` loads all data into in-memory HashMaps with no eviction policy. For a project with 100K+ spans, this can consume gigabytes of RAM and eventually OOM the process. Add a memory-bounded cache with LRU eviction.

## Context

This is related to but distinct from #010 (conditional caching). Issue #010 makes the cache optional for cloud mode. This issue adds eviction for local mode where the cache is still needed for performance.

## What to do

1. Replace `HashMap<SpanId, Span>` (and similar) with an LRU cache (e.g., `lru::LruCache` or `moka::sync::Cache`)

2. Set a configurable max entries per entity type:
   - Traces: 10,000 (default)
   - Spans: 50,000 (default)
   - Datasets/Datapoints: 5,000 (default)
   - Configurable via environment variable: `TRACEWAY_CACHE_MAX_TRACES=10000`

3. On cache miss during reads, fetch from backend and insert into cache (cache-aside pattern)

4. Eviction happens automatically when the cache exceeds max size — least-recently-used entries are dropped

5. Write-through pattern remains: writes go to both cache and backend

## Files to modify

- `crates/storage/src/lib.rs` — Replace HashMap with LRU cache
- `Cargo.toml` — Add `lru` or `moka` dependency

## Acceptance criteria

- [ ] Cache has a configurable max size per entity type
- [ ] LRU eviction works correctly
- [ ] Cache misses transparently fetch from backend
- [ ] Memory usage stays bounded under sustained load
- [ ] `cargo check` passes
