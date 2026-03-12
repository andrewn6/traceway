# Make PersistentStore Cache Conditional (Backend-Dependent)

**Labels:** `enhancement`, `backend`, `PRD-08`, `breaking-change`
**Difficulty:** Hard
**PRD:** [PRD-08: Cache Externalization](../../prds/PRD-08-cache-externalization.md) — Phases 1-3

## Summary

Refactor `PersistentStore` so the in-memory `HashMap` cache is optional. SQLite (local mode) keeps the cache. Turbopuffer (cloud mode) bypasses it and reads directly from the backend. This is prerequisite for running multiple API server instances.

## Why this is hard

- Touches every CRUD method on `PersistentStore` (~40+ methods)
- Requires changing `SharedStore` from `Arc<RwLock<PersistentStore>>` to `Arc<PersistentStore>` with internal per-entity locks
- Every API handler in `crates/api/src/lib.rs` uses `store.read().await` / `store.write().await` — all must be updated
- Must not regress local mode behavior or performance

## What to do

### Step 1: Make cache optional

```rust
pub struct PersistentStore<B: StorageBackend> {
    backend: Arc<B>,
    cache: Option<StoreCache>,
}

struct StoreCache {
    traces: RwLock<HashMap<TraceId, Trace>>,
    spans: RwLock<HashMap<SpanId, Span>>,
    datasets: RwLock<HashMap<DatasetId, Dataset>>,
    datapoints: RwLock<HashMap<DatapointId, Datapoint>>,
    // ... all entity maps, each with its own RwLock
}
```

### Step 2: Branch on cache presence in every method

```rust
pub async fn get_trace(&self, id: TraceId) -> Result<Option<Trace>, StorageError> {
    if let Some(cache) = &self.cache {
        let reader = cache.traces.read().await;
        Ok(reader.get(&id).cloned())
    } else {
        self.backend.get_trace(id).await
    }
}
```

### Step 3: Change SharedStore type

```rust
// Before
pub type SharedStore = Arc<RwLock<PersistentStore<AnyBackend>>>;

// After
pub type SharedStore = Arc<PersistentStore<AnyBackend>>;
```

### Step 4: Update all API handlers

```rust
// Before
let store = state.store_for_org(org_id).await;
let reader = store.read().await;
let trace = reader.get_trace(id).await?;

// After
let store = state.store_for_org(org_id).await;
let trace = store.get_trace(id).await?;
```

### Step 5: Update `open()` to accept cache flag

```rust
pub async fn open(backend: B, use_cache: bool) -> Result<Self, StorageError> {
    if use_cache {
        // Load all entities into cache (existing behavior)
    }
    Ok(Self { backend: Arc::new(backend), cache })
}
```

Wire `use_cache` based on `backend.backend_type()`: `true` for `"sqlite"`, `false` for `"turbopuffer"`.

## Files to modify

- `crates/storage/src/lib.rs` — refactor `PersistentStore`
- `crates/api/src/org_store.rs` — update `SharedStore` type
- `crates/api/src/lib.rs` — update every handler's lock pattern (~30+ handlers)
- `crates/api/src/capture.rs` — update store access
- `crates/api/src/jobs.rs` — update store access (if applicable)

## Acceptance criteria

- [ ] Local mode (SQLite): behavior identical to before, cache populated on startup
- [ ] Cloud mode (Turbopuffer): no cache, all reads go to backend, startup is instant
- [ ] No global `RwLock` on the entire store — per-entity locks (or no locks in cloud mode)
- [ ] All existing API tests pass
- [ ] `cargo check` succeeds for all crates
- [ ] No deadlocks under concurrent request load
