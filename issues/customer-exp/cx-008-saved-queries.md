# Saved / Pinned Queries

**Labels:** `enhancement`, `frontend`, `backend`
**Difficulty:** Easy
**Priority:** Low

## Summary

Let users save query DSL strings with a name so they can re-run common queries without retyping them. Currently the query page only has localStorage history (last 50 queries) and URL sharing — no way to name, organize, or pin useful queries.

## Context

The query page is already powerful with its DSL, autocomplete, and three view modes. But users who run the same diagnostic queries daily (e.g., "failed LLM calls in the last hour", "expensive gpt-4o calls") have to either remember the DSL syntax, scroll through history, or bookmark URLs. A first-class saved queries feature removes this friction.

## What to do

### 1. Save query action

Add a "Save" button (bookmark/pin icon) next to the search input. Clicking it opens a small popover/modal:
- Name input (required, e.g., "Failed LLM calls today")
- The current DSL query pre-filled and non-editable
- Save button

### 2. Saved queries sidebar or dropdown

Add a "Saved" section to the query page:
- Either a collapsible sidebar panel on the left, or a dropdown button next to the search bar
- Lists saved queries by name with the DSL preview
- Click to load into the search input and execute
- Delete button (with confirm) on each saved query
- Drag to reorder (stretch)

### 3. Storage

**Local mode**: Store in localStorage under `traceway_saved_queries` (array of `{id, name, query, created_at}`)

**Cloud mode**: Store server-side per project:
- Add a `saved_queries` table to the auth Postgres store (id, org_id, project_id, user_id, name, query, created_at)
- `POST /api/saved-queries`, `GET /api/saved-queries`, `DELETE /api/saved-queries/:id`

For the initial implementation, localStorage-only is acceptable — server-side sync can come later.

### 4. Quick filters integration

The existing 6 quick filter presets ("Slow LLM calls", "Failed spans", etc.) should also appear in the saved queries list as non-deletable built-in entries, so users have one unified place for all query shortcuts.

## Files to modify

- `ui/src/routes/query/+page.svelte` — Save button, saved queries list
- `ui/src/lib/api.ts` — Add SavedQuery type (if doing server-side storage)
- Optional: `crates/storage-postgres/src/lib.rs` — saved_queries table

## Acceptance criteria

- [ ] Save button appears next to query input
- [ ] Saved queries persist across page reloads (localStorage at minimum)
- [ ] Clicking a saved query loads and executes it
- [ ] Delete works on saved queries
- [ ] Built-in presets appear in the same list
- [ ] `npm run build` passes
