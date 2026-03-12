# Trace Timeline / Waterfall View

**Labels:** `enhancement`, `frontend`
**Difficulty:** Medium
**Priority:** High

## Summary

Add a timeline (waterfall/Gantt) view mode to the trace detail page. Each span renders as a horizontal bar on a shared time axis, making it immediately obvious where time is spent, what runs in parallel, and where gaps exist. This is the single most requested visualization for latency debugging in LLM observability tools.

## Context

The trace detail page currently supports two modes: **tree view** (hierarchical DFS) and **flat view** (chronological list). Both show duration as text (`+120ms`, `320ms`) but neither visualizes timing spatially. When a trace has 20+ spans with varying durations and parallelism, text-based views make it hard to answer "where did the time go?"

Traceway already has a good query DSL — the timeline view should integrate with it, so you can filter spans within a trace (`kind:llm_call duration:>500ms`) and the timeline highlights matching bars.

## What to do

### 1. Timeline layout engine

Add a `TimelineView.svelte` component in `ui/src/lib/components/` that:

- Takes the same span tree data the existing `TraceTimeline` component uses
- Computes horizontal bar positions relative to the trace's `started_at` (time 0)
- Each bar: `left = (span.started_at - trace.started_at) / total_duration * 100%`, `width = span.duration / total_duration * 100%`
- Rows are stacked vertically, indented by depth in the span tree
- Minimum bar width of 2px so very short spans remain visible

### 2. Visual design

- Bar color by span kind: LLM calls = accent blue, custom = neutral gray, fs_read/fs_write = muted green
- Failed spans get a red bar or red border
- Running spans get a pulsing/striped animation
- Span name label on the bar when space allows, otherwise on hover tooltip
- Duration label on or next to the bar
- For LLM spans, show token count badge on the bar when space allows
- Time axis at the top with tick marks (auto-scale: ms, s, min)

### 3. Interaction

- Click a bar to select that span (opens span detail in right pane, same as tree view)
- Hover shows tooltip: span name, kind, model (if LLM), duration, tokens, cost
- Zoom: scroll wheel or +/- buttons to zoom the time axis. Pan with click-drag on the axis
- The selected span bar should have a highlight border/glow

### 4. View mode integration

- Add "Timeline" as a third option in the existing view mode toggle (Tree | Flat | Timeline)
- Persist the selected mode in localStorage alongside the existing preference
- View mode syncs to URL param so shared links open in the right mode

### 5. DSL filter integration (stretch)

If the trace has many spans, the query DSL search bar (already present in the trace detail header) should dim/fade non-matching bars in timeline mode, keeping matching bars fully opaque. This lets you visually spot "all LLM calls over 500ms" in the waterfall.

## Files to modify

- `ui/src/lib/components/TraceTimeline.svelte` — Add view mode prop and conditionally render new timeline component
- `ui/src/routes/traces/[id]/+page.svelte` — Add "Timeline" to view toggle, pass view mode down
- New: `ui/src/lib/components/TimelineView.svelte` — The waterfall renderer

## Acceptance criteria

- [ ] Timeline view renders horizontal bars on a time axis for every span in the trace
- [ ] Bars are positioned correctly relative to trace start time
- [ ] Clicking a bar selects the span and opens the detail pane
- [ ] Zoom in/out works smoothly
- [ ] LLM spans show token count badges
- [ ] Failed spans are visually distinct (red)
- [ ] View mode persists across page loads
- [ ] `npm run build` passes
