# Trace List Time Range Filter

**Labels:** `good first issue`, `frontend`
**Difficulty:** Easy
**Priority:** Medium

## Summary

Add a time range filter to the trace list page. Currently there is no way to filter traces by time on this page — you see all traces regardless of when they were created. The analytics page has time range buttons (1h, 6h, 1d, 7d, 30d) but the trace list does not.

## Context

The trace list page (`ui/src/routes/traces/+page.svelte`) has filters for model and status but no time filter. The query page supports `since:1h` and `until:` in the DSL, and the analytics page has time range buttons. The trace list — the most frequently visited page — is the only one without time filtering.

As users accumulate traces, loading all of them becomes slow and the list becomes unwieldy. A time range filter is both a UX and performance improvement.

## What to do

### 1. Time range selector

Add a row of time range buttons to the trace list filter bar (matching the analytics page style):

- **1h** | **6h** | **24h** | **7d** | **30d** | **All**
- Default: **24h** (instead of showing all traces)
- Selected button gets the active style (filled/highlighted)

### 2. Backend filter

The `GET /api/traces` endpoint (or however traces are fetched) needs to accept `since` and `until` query parameters:

- `since`: ISO timestamp or relative duration
- Filter: `WHERE started_at >= since`

If the backend already supports this (check the implementation), just wire the frontend to send the parameter. If not, add the filter to the trace listing query.

### 3. URL state

Sync the selected time range to the URL query string (`?range=24h`) so it persists on page reload and is shareable.

### 4. Auto-refresh interaction

The trace list already receives SSE events for new traces. The time range filter should work with live updates — new traces that fall within the selected range appear immediately.

## Files to modify

- `ui/src/routes/traces/+page.svelte` — Time range buttons, filter logic
- `ui/src/lib/api.ts` — Add `since` parameter to trace fetch function
- `crates/api/src/lib.rs` — Add `since`/`until` query params to trace list endpoint (if not already present)

## Acceptance criteria

- [ ] Time range buttons appear on the trace list page
- [ ] Selecting a range filters traces to that time window
- [ ] Default is 24h (not "all")
- [ ] Selection persists in URL
- [ ] New traces via SSE still appear if they're within the selected range
- [ ] `npm run build` passes
