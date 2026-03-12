# Fix SDK Pagination Response Type Mismatch

**Labels:** `bug`, `backend`, `sdk`
**Difficulty:** Easy
**Priority:** P1

## Summary

Both the Python and TypeScript SDKs expect list endpoints to return fields like `traces`, `spans`, `count`, but the API actually returns a `Page<T>` structure with `items`, `total`, `next_cursor`, `has_more`. This mismatch means SDK pagination is broken — clients either crash on missing fields or silently get no data.

## Where

**Python SDK** (`sdk/python/traceway/`):
- Expects response keys like `traces`, `spans` in the JSON response
- API returns `{ items: [...], total: N, ... }`

**TypeScript SDK** (`sdk/typescript/src/`):
- Same mismatch — looks for `traces`/`spans` keys

**API** (`crates/api/src/lib.rs`):
- List endpoints return `Page<T>` with `items` field

## What to do

1. **Update both SDKs** to match the actual API response format:
   - Access `response.items` instead of `response.traces` / `response.spans`
   - Parse `total`, `next_cursor`, `has_more` for pagination state
   - Update TypeScript types and Python dataclasses accordingly

2. **Or update the API** to use the field names the SDKs expect. Less preferred since `Page<T>` with `items` is more generic.

3. Add integration tests that verify SDK list methods return actual data.

## Files to modify

- `sdk/python/traceway/client.py` (or equivalent) — Fix response parsing
- `sdk/typescript/src/client.ts` (or equivalent) — Fix response parsing
- Both SDK type definitions

## Acceptance criteria

- [ ] Python SDK `list_traces()` returns actual traces
- [ ] TypeScript SDK `listTraces()` returns actual traces
- [ ] Pagination cursor works for fetching subsequent pages
- [ ] Both SDKs handle empty results correctly
