# Trace Comparison / Diff View

**Labels:** `enhancement`, `frontend`
**Difficulty:** Hard
**Priority:** Medium — Edge Feature

## Summary

Add a side-by-side trace comparison view that highlights differences between two traces. This is especially useful for debugging regressions ("this worked yesterday, why does it fail today?"), comparing A/B test variants, and evaluating prompt changes. No major competitor has this as a first-class feature.

## Context

A common debugging workflow: the user runs the same agent twice — once it succeeds, once it fails. They want to see exactly where the execution diverged. Today they'd need to open two browser tabs, manually scroll through both traces, and visually compare spans. A dedicated diff view automates this.

## What to do

### 1. Comparison entry point

Two ways to enter comparison mode:

- **From trace list**: Checkbox to select two traces, then "Compare" button appears
- **From trace detail**: "Compare with..." button in the trace header, opens a trace picker modal

### 2. Side-by-side layout

Split the viewport into left and right panels, each showing a trace:

- **Header**: Trace A name/timestamp on the left, Trace B on the right
- **Summary diff**: A table at the top showing key metrics side-by-side:
  - Total duration (with delta: +120ms / -50ms, colored green/red)
  - Total tokens (with delta)
  - Total cost (with delta)
  - Span count (with delta)
  - Status

### 3. Span alignment

Attempt to match spans between the two traces by name + position in the tree:

- **Matched spans**: Show side-by-side with differences highlighted
  - Duration difference (bar visualization showing relative difference)
  - Token count difference
  - If input/output differs, show a text diff (added/removed lines, like a code diff)
- **Unmatched spans**: Spans that exist in trace A but not B (or vice versa) are highlighted with "only in A" / "only in B" labels
- **Order**: Show spans in chronological order, with matched spans aligned on the same row

### 4. Input/output diff

When the user clicks a matched span pair, the detail pane shows a unified diff of the input and output:

- For JSON: structural diff (added/removed/changed keys)
- For chat messages: per-message diff
- For plain text: line-level diff with red/green highlighting

### 5. URL routing

`/traces/compare?a=<trace_id>&b=<trace_id>` — shareable comparison links.

## Files to create/modify

- New: `ui/src/routes/traces/compare/+page.svelte` — Comparison page
- New: `ui/src/lib/components/TraceDiff.svelte` — Diff visualization
- `ui/src/routes/traces/+page.svelte` — Add multi-select checkbox and compare button
- `ui/src/routes/traces/[id]/+page.svelte` — Add "Compare with..." action

## Acceptance criteria

- [ ] Can select two traces and open comparison view
- [ ] Summary metrics shown side-by-side with deltas
- [ ] Spans are aligned by name and tree position
- [ ] Input/output text diff works for JSON and chat messages
- [ ] Unmatched spans are clearly labeled
- [ ] Comparison URL is shareable
- [ ] `npm run build` passes
