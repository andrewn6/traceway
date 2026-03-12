# Keyboard Shortcuts for Trace Navigation

**Labels:** `good first issue`, `frontend`
**Difficulty:** Easy
**Priority:** Low

## Summary

Add keyboard shortcuts for navigating the trace detail page — moving between spans, expanding/collapsing nodes, switching view modes, and toggling the detail pane. Power users who review many traces daily will benefit from not having to reach for the mouse.

## Context

The trace detail page currently only has one keyboard shortcut: `/` to focus the search bar. Everything else requires mouse interaction. For an observability tool that's used daily, keyboard-driven navigation significantly improves review speed.

## What to do

### 1. Span navigation

- `j` / `Down arrow` — Select next span in the list
- `k` / `Up arrow` — Select previous span
- `Enter` — Open selected span's detail pane (if not already open)
- `Escape` — Close detail pane / deselect span

### 2. Tree manipulation (tree view only)

- `l` / `Right arrow` — Expand selected node (if collapsed)
- `h` / `Left arrow` — Collapse selected node (if expanded), or jump to parent
- `e` — Expand all
- `shift+e` — Collapse all

### 3. View mode switching

- `1` — Switch to Tree view
- `2` — Switch to Flat view
- `3` — Switch to Timeline view (once cx-001 is implemented)
- `4` — Switch to Reader view (once cx-002 is implemented)

### 4. Detail pane tabs

When the detail pane is open:
- `Tab` / `Shift+Tab` — Cycle through detail pane tabs (Input, Output, Attributes, etc.)

### 5. Global

- `/` — Focus search bar (already exists)
- `?` — Show keyboard shortcuts help modal (a small overlay listing all shortcuts)

### 6. Implementation

- Use a top-level `keydown` event listener on the trace detail page
- Don't capture shortcuts when an input/textarea is focused
- Show a subtle hint in the UI: "Press ? for keyboard shortcuts"

## Files to modify

- `ui/src/routes/traces/[id]/+page.svelte` — Keydown handler, shortcut logic
- New: `ui/src/lib/components/ShortcutsHelp.svelte` — Help overlay modal

## Acceptance criteria

- [ ] `j`/`k` navigate between spans
- [ ] `h`/`l` expand/collapse tree nodes
- [ ] `1`-`4` switch view modes
- [ ] `?` opens a shortcuts help modal
- [ ] Shortcuts don't fire when typing in an input field
- [ ] `npm run build` passes
