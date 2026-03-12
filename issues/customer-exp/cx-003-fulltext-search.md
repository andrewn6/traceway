# Full-Text Search Across Span Inputs and Outputs

**Labels:** `enhancement`, `backend`, `frontend`
**Difficulty:** Hard
**Priority:** High

## Summary

Add full-text search that lets users search across span input/output content and span names to jump directly to the trace containing a specific prompt, completion, error message, or tool call. This is the fastest path from "I know what the model said" to "show me the full trace."

## Context

Traceway's query DSL supports filtering by kind, model, status, duration, cost, and name substring — but it cannot search inside span input/output content. If a user wants to find the trace where the model said "I don't have access to that file," they have no way to do it today short of clicking through traces one by one.

The query page already has a powerful DSL and autocomplete. Extending it with a `content:` or `text:` filter is the natural approach. No need for a separate search page.

## What to do

### 1. Backend: Search endpoint

Add a search capability to the API. Two approaches, depending on backend:

**SQLite backend (local mode):**
- Use SQLite FTS5 (full-text search) on a virtual table that indexes `span.input` and `span.output` as text columns
- Add migration to create `spans_fts` virtual table
- Query: `SELECT span_id, trace_id FROM spans_fts WHERE spans_fts MATCH ? ORDER BY rank`
- Join back to spans table for metadata

**Turbopuffer backend (cloud mode):**
- Turbopuffer already supports text search via BM25. Leverage existing vector search capabilities
- Alternatively, add a `POST /api/search` endpoint that does a LIKE/contains scan with pagination (acceptable at cloud scale with proper indexing)

### 2. Query DSL extension

Add two new filter types to the DSL parser (`ui/src/lib/query-dsl.ts`):

- `text:"search phrase"` — searches across both input and output content
- `input:"search phrase"` — searches input only
- `output:"search phrase"` — searches output only

The parser should support quoted strings for multi-word searches.

### 3. Backend: Wire into existing query flow

The existing `POST /api/analytics` or the span listing endpoints need to accept a `text_search` parameter. When present:

1. Run the FTS query to get matching span IDs
2. Intersect with other filters (kind, model, status, etc.)
3. Return results with a **snippet** showing the matched text with surrounding context (highlight the match)

### 4. Frontend: Search results

In the query page results (table/grouped view):
- Show a "Match" column or inline snippet showing where the search text was found
- Highlight the matched text in the snippet
- Add `text:` to the autocomplete suggestions when the user types a new filter token

### 5. UI: Global search shortcut

- `Cmd+K` / `Ctrl+K` opens a search modal that focuses on text search
- Results show span name, trace name, matched snippet, timestamp
- Clicking a result navigates to the trace with that span selected

## Files to modify

- `crates/storage-sqlite/src/lib.rs` — Add FTS5 virtual table migration, implement search query
- `crates/storage/src/backend.rs` — Add `search_spans(query, filters, limit)` to `StorageBackend` trait
- `crates/api/src/lib.rs` — Wire search into API (new endpoint or extend existing listing)
- `ui/src/lib/query-dsl.ts` — Add `text:`, `input:`, `output:` filter types
- `ui/src/routes/query/+page.svelte` — Show match snippets in results
- New: `ui/src/lib/components/SearchModal.svelte` — Global Cmd+K search

## Acceptance criteria

- [ ] `text:"search phrase"` in query DSL returns spans containing that text in input or output
- [ ] SQLite FTS5 index is created via migration (local mode)
- [ ] Search results include a text snippet with the match highlighted
- [ ] `Cmd+K` opens a quick search modal
- [ ] Existing DSL filters (kind, model, status) compose with text search
- [ ] Performance: searches over 10K spans return in <500ms (local mode)
- [ ] `cargo check` and `npm run build` pass
