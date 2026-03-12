# Custom Dashboard Chart Builder

**Labels:** `enhancement`, `frontend`, `backend`
**Difficulty:** Hard
**Priority:** Medium

## Summary

Let users create custom charts on the analytics dashboard beyond the 7 built-in cards. A chart builder UI lets users pick a metric (tokens, cost, duration, count), an aggregation (sum, avg, p95, p99), optional group-by (model, provider, kind), and a chart type (line, bar, horizontal bar). The backend analytics endpoint already supports flexible queries — this is primarily a frontend feature.

## Context

The analytics page already supports reordering, resizing, and toggling built-in cards. The backend `POST /api/analytics` endpoint supports arbitrary metrics (`TotalCost`, `TotalTokens`, `AvgLatencyMs`, `SpanCount`, `ErrorCount`) with group-by dimensions (`Model`, `Provider`, `Kind`, `Status`, `Day`, `Hour`). The infrastructure for custom charts mostly exists — it just needs a user-facing builder.

## What to do

### 1. Chart builder modal

"+ Add Chart" button on the dashboard opens a builder modal with:

- **Chart type**: Line, Bar, Horizontal Bar (radio/segmented control)
- **Metric**: Dropdown — Total Cost, Total Tokens, Input Tokens, Output Tokens, Avg Latency, Span Count, Error Count
- **Aggregation**: Sum, Avg, Min, Max, P90, P95, P99 (where applicable)
- **Group by** (optional): Model, Provider, Kind, Status, or None
- **Filters** (optional): Kind filter, model filter, status filter
- **Name**: Text input for the chart title

Preview the chart live in the modal before saving.

### 2. Persistence

Store custom chart configs alongside the existing layout config in localStorage (`analytics_layout`). Each chart config is a JSON object:

```json
{
  "id": "custom-1",
  "name": "GPT-4o cost over time",
  "type": "line",
  "metric": "TotalCost",
  "aggregation": "sum",
  "groupBy": "Model",
  "filters": { "model": "gpt-4o" },
  "width": "half"
}
```

### 3. Rendering

Custom charts render using the same charting components as built-in cards (sparkline, bar chart). They call `POST /api/analytics` with the appropriate query parameters when the dashboard loads or the time range changes.

### 4. Management

- Edit button on custom charts to modify the config
- Delete button (with confirm)
- Custom charts are reorderable alongside built-in cards

## Files to modify

- `ui/src/routes/analytics/+page.svelte` — Add chart button, render custom charts
- New: `ui/src/lib/components/ChartBuilder.svelte` — Builder modal
- Potentially extend `crates/api/src/lib.rs` analytics endpoint if new aggregation types (p90/p95/p99) need backend support

## Acceptance criteria

- [ ] "+ Add Chart" button opens builder modal
- [ ] User can select metric, chart type, group-by, and filters
- [ ] Chart previews live in the modal
- [ ] Saved charts persist across page reloads
- [ ] Custom charts respond to time range and interval changes
- [ ] Edit and delete work on custom charts
- [ ] `npm run build` passes
