# Span Tag Filtering in Trace List and Query Page

**Labels:** `enhancement`, `frontend`, `backend`
**Difficulty:** Easy
**Priority:** Medium

## Summary

Tags already exist on the `Trace` data model (`tags: Vec<String>`) and in the SDK (`create_trace(tags=["prod", "v2"])`), but the trace list page and query page have no way to filter by tags. Add tag filtering to both pages and tag display to the trace list rows.

## Context

The `Trace` struct in `crates/trace/src/lib.rs` has a `tags: Vec<String>` field. Both the Python and TypeScript SDKs support setting tags on trace creation. However:

- The trace list page (`ui/src/routes/traces/+page.svelte`) doesn't show tags or allow filtering by them
- The query DSL (`ui/src/lib/query-dsl.ts`) doesn't support a `tag:` filter
- There's no UI for adding/removing tags on an existing trace post-creation

## What to do

### 1. Trace list: Show tags

In the trace list table, add a "Tags" column (or show tags as small badges/pills after the trace name). Use muted colored pills that are compact enough to not bloat the row height.

### 2. Trace list: Tag filter

Add a tag filter dropdown to the trace list filter bar (next to the existing model and status filters):

- Multi-select dropdown showing all tags that exist across the current trace set
- Selecting tags filters to traces that have **any** of the selected tags (OR logic)
- Tags in the dropdown should show a count of matching traces

To populate the dropdown, add a `GET /api/tags` endpoint that returns all unique tags with counts, or include tag aggregation in the existing summary/analytics endpoint.

### 3. Query DSL: `tag:` filter

Add `tag:production` as a filter type in the DSL parser. Multiple `tag:` filters should AND together (trace must have all specified tags).

Autocomplete for `tag:` should fetch available tags from the API.

### 4. Post-creation tag editing (stretch)

In the trace detail page header, show tags as editable pills. Allow adding new tags and removing existing ones via a simple input. This requires a `PATCH /api/traces/:id/tags` endpoint.

## Files to modify

- `ui/src/routes/traces/+page.svelte` — Show tags in trace rows, add tag filter
- `ui/src/lib/query-dsl.ts` — Add `tag:` filter type
- `ui/src/routes/query/+page.svelte` — Wire tag filter into query execution
- `crates/api/src/lib.rs` — Add `GET /api/tags` endpoint (or extend summary)
- Optional: `PATCH /api/traces/:id/tags` for post-creation editing

## Acceptance criteria

- [ ] Tags appear as badges on trace list rows
- [ ] Tag filter dropdown works on the trace list page
- [ ] `tag:production` works in the query DSL
- [ ] Tag autocomplete suggests existing tags
- [ ] `npm run build` passes
