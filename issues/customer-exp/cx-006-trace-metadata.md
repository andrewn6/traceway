# Trace-Level Metadata (Key-Value Context)

**Labels:** `enhancement`, `backend`, `frontend`
**Difficulty:** Easy
**Priority:** Medium

## Summary

Add a `metadata: HashMap<String, Value>` field to traces for attaching arbitrary key-value context — environment, deployment version, feature flags, A/B test variant, region, etc. Metadata is set via the SDK and displayed in the trace detail page.

## Context

Tags are string labels for categorization. Metadata is structured key-value data for context. A trace tagged `["production", "v2"]` tells you *what* it is; metadata like `{"environment": "production", "version": "2.1.3", "region": "us-west", "experiment": "new-prompt-v2"}` tells you *where and how* it ran.

Currently, the only way to attach context is through tags (flat strings) or by encoding it into the trace name. A dedicated metadata field lets users store structured context without abusing other fields.

## What to do

### 1. Data model

Add to `Trace` in `crates/trace/src/lib.rs`:

```rust
pub metadata: Option<HashMap<String, serde_json::Value>>,
```

### 2. Storage

- SQLite: Add `metadata TEXT` column (JSON-serialized) to traces table
- Turbopuffer: Include as a trace attribute

### 3. API

- `POST /api/traces` accepts `metadata` dict in request body
- Returned in `GET /api/traces` and trace detail responses

### 4. SDK

```python
# Python
with client.trace("run", metadata={"env": "prod", "version": "2.1.3"}) as t:
    ...
```

```typescript
// TypeScript
await client.trace("run", { metadata: { env: "prod", version: "2.1.3" } }, async (ctx) => { ... });
```

### 5. Frontend

- **Trace detail page**: Show a "Metadata" section in the trace header (collapsible JSON viewer). Only visible when metadata is non-empty
- **Trace list**: Optionally show 1-2 key metadata values as small labels (e.g., `env: prod`) — configurable

### 6. Query DSL (stretch)

Add `meta.key:value` filter syntax: `meta.environment:production` filters traces where `metadata.environment == "production"`.

## Files to modify

- `crates/trace/src/lib.rs` — Add `metadata` field to `Trace`
- `crates/storage-sqlite/src/lib.rs` — Migration, update queries
- `crates/api/src/lib.rs` — Update trace handlers
- `sdk/python/traceway/` and `sdk/typescript/src/` — Add metadata parameter
- `ui/src/routes/traces/[id]/+page.svelte` — Show metadata section
- `ui/src/lib/api.ts` — Update Trace type

## Acceptance criteria

- [ ] Metadata stored as JSON on traces
- [ ] SDKs support setting metadata on trace creation
- [ ] Trace detail page shows metadata in a collapsible JSON viewer
- [ ] `cargo check` and `npm run build` pass
