# Session and User ID Tracking on Traces

**Labels:** `enhancement`, `backend`, `frontend`
**Difficulty:** Medium
**Priority:** Medium

## Summary

Add `session_id` and `user_id` as first-class fields on traces. Sessions group multiple traces into a conversation or workflow. User IDs let you filter by which end-user triggered a trace. Both are set via the SDK and filterable in the UI.

## Context

Traceway traces currently have no concept of sessions or users. Every trace is standalone — there's no way to say "these 5 traces are all part of the same chat conversation" or "show me all traces from user X." This makes it impossible to debug multi-turn conversations or do per-user analytics.

The `Trace` struct already has `name`, `tags`, `machine_id`, `started_at`, `ended_at`. Adding `session_id` and `user_id` as optional string fields is straightforward.

## What to do

### 1. Data model

Add to `Trace` in `crates/trace/src/lib.rs`:

```rust
pub session_id: Option<String>,
pub user_id: Option<String>,
```

### 2. Storage backends

- SQLite: Add `session_id TEXT` and `user_id TEXT` columns to traces table (migration)
- Turbopuffer: Include in the trace document attributes
- PersistentStore: Pass through (no cache changes needed)

### 3. API

- `POST /api/traces` (create trace) accepts `session_id` and `user_id` in the request body
- `GET /api/traces` response includes `session_id` and `user_id`
- Add `GET /api/traces?session_id=X` filter parameter
- Add `GET /api/traces?user_id=X` filter parameter

### 4. SDK

Both Python and TypeScript SDKs:

```python
# Python
with client.trace("chat-turn", session_id="conv_123", user_id="user_456") as t:
    ...
```

```typescript
// TypeScript
await client.trace("chat-turn", { sessionId: "conv_123", userId: "user_456" }, async (ctx) => { ... });
```

### 5. Frontend: Session grouping

On the trace list page, add a "Sessions" tab (alongside the default trace list):
- Groups traces by `session_id`
- Each session row shows: session ID, trace count, time range, total tokens, total cost
- Expanding a session shows the individual traces within it, ordered by `started_at`
- Traces without a session_id appear in the default list only

### 6. Frontend: User filter

Add a "User" filter to the trace list filter bar:
- Text input that filters traces by `user_id` (exact or substring match)
- Show `user_id` as a small label in the trace list rows when present

### 7. Query DSL

Add `session:conv_123` and `user:user_456` filter types to the DSL parser.

## Files to modify

- `crates/trace/src/lib.rs` — Add fields to `Trace`
- `crates/storage-sqlite/src/lib.rs` — Migration, update queries
- `crates/storage-turbopuffer/src/lib.rs` — Include in attributes
- `crates/api/src/lib.rs` — Update create/list trace handlers
- `sdk/python/traceway/` — Add session_id, user_id to trace creation
- `sdk/typescript/src/` — Add sessionId, userId to trace creation
- `ui/src/routes/traces/+page.svelte` — Sessions tab, user filter
- `ui/src/lib/api.ts` — Update Trace type
- `ui/src/lib/query-dsl.ts` — Add session/user filter types

## Acceptance criteria

- [ ] `session_id` and `user_id` stored and returned on traces
- [ ] SDKs support setting both fields on trace creation
- [ ] Sessions tab groups traces by session_id
- [ ] User filter works on trace list page
- [ ] `session:` and `user:` work in query DSL
- [ ] SQLite migration adds columns without breaking existing data
- [ ] `cargo check` and `npm run build` pass
