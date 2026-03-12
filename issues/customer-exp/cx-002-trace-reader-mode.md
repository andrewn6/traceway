# Trace Reader Mode

**Labels:** `enhancement`, `frontend`
**Difficulty:** Medium
**Priority:** High

## Summary

Add a "Reader" view mode to the trace detail page that shows a clean, flattened, narrative view of a trace — optimized for quickly scanning LLM conversations and tool calls without drilling through nested spans. By default, hide non-LLM/non-tool spans so the user sees only the high-signal interactions.

## Context

The current tree view shows every span with full nesting. For a typical agent trace with 30+ spans, most are custom wrapper spans — the user cares about the 5-8 LLM calls and their inputs/outputs. The existing inline content preview (200 chars) helps, but it's cramped inside the tree rows.

Reader mode should feel like reading a conversation transcript: each LLM call is a "turn" with the full prompt and completion visible inline, formatted as chat messages when applicable.

## What to do

### 1. Reader view component

Create `ReaderView.svelte` that:

- Filters the span list to show only **LLM call** and **Custom** spans by default (hide `fs_read`/`fs_write` unless toggled)
- Renders each span as a full-width card with:
  - **Header row**: Span name, model badge, duration, token counts, cost
  - **Input section**: If chat messages (array of `{role, content}`), render the full conversation using the existing chat message rendering from `SpanDetail.svelte`. If raw JSON, show formatted JSON with syntax highlighting
  - **Output section**: Same treatment as input
  - **Breadcrumbs**: Show the span's position in the tree (`parent > child > this_span`) so the user knows where it fits
- Cards are ordered chronologically (by `started_at`)
- Expand/collapse per card (default: expanded for LLM calls, collapsed for custom spans)

### 2. Span type filter

- Toggle bar at the top: "LLM Calls" (on by default), "Custom" (on by default), "File I/O" (off by default)
- Filter updates the visible cards instantly

### 3. View mode integration

- Add "Reader" as a fourth option in the view toggle: Tree | Flat | Timeline | Reader
- Clicking a card's header still selects that span in the detail pane (right panel)

### 4. Performance

- Lazy-load input/output content: Only fetch and parse JSON for cards that are scrolled into view
- For traces with 100+ LLM calls, virtualize the card list

## Files to modify

- `ui/src/routes/traces/[id]/+page.svelte` — Add "Reader" to view toggle
- New: `ui/src/lib/components/ReaderView.svelte`
- Reuse: Chat message rendering logic from `SpanDetail.svelte` (extract to a shared component if not already)

## Acceptance criteria

- [ ] Reader mode shows only LLM and custom spans by default
- [ ] Each card shows full input/output with chat message formatting where applicable
- [ ] Span type toggles filter cards in real-time
- [ ] Cards are ordered chronologically
- [ ] Clicking a card selects the span and opens the detail pane
- [ ] Performance is acceptable for traces with 50+ LLM spans
- [ ] `npm run build` passes
