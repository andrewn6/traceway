# Quick Copy Actions on Spans and Traces

**Labels:** `good first issue`, `frontend`
**Difficulty:** Easy
**Priority:** Low

## Summary

Add one-click copy buttons throughout the trace detail UI — copy span ID, trace ID, span input, span output, full trace JSON, and a "copy as curl" for the LLM call. Small DX improvement that saves time when debugging, sharing, or reproducing issues.

## Context

Currently, to copy a span's output, the user has to manually select text in the JSON viewer and copy. Span IDs and trace IDs are shown in the attributes tab but have no copy button. The "Export JSON" button in the trace header downloads a file — there's no "copy to clipboard" shortcut.

## What to do

### 1. Copy buttons on IDs

In `SpanDetail.svelte`, the Attributes tab shows span ID and trace ID as text. Add a small copy icon button next to each that copies the ID to clipboard with a brief "Copied!" tooltip.

### 2. Copy input/output

In the Input and Output tabs of `SpanDetail.svelte`:
- Add a "Copy" button in the tab header area (next to the raw/formatted toggle)
- Copies the raw JSON string to clipboard
- Brief "Copied!" feedback

### 3. Copy trace JSON

In the trace detail page header, alongside the existing "Export JSON" button:
- Add a "Copy JSON" button (or make the export button a dropdown: "Download JSON" / "Copy to clipboard")
- Copies the full trace JSON (same format as the export) to clipboard

### 4. Copy as cURL (LLM spans only)

For LLM call spans, add a "Copy as cURL" action in the span detail header:
- Reconstructs a cURL command from the span's input (chat messages), model, and provider
- Example output:
  ```bash
  curl https://api.openai.com/v1/chat/completions \
    -H "Authorization: Bearer $OPENAI_API_KEY" \
    -H "Content-Type: application/json" \
    -d '{"model":"gpt-4o","messages":[...]}'
  ```
- Only show this for spans where the provider is known (OpenAI, Anthropic)

### 5. Keyboard shortcut

`Cmd+C` / `Ctrl+C` when a span is selected (and no text is manually selected) copies the span's output to clipboard.

## Files to modify

- `ui/src/lib/components/SpanDetail.svelte` — Copy buttons on IDs, input/output, cURL
- `ui/src/routes/traces/[id]/+page.svelte` — Copy trace JSON button

## Acceptance criteria

- [ ] Copy buttons on span ID and trace ID with feedback
- [ ] Copy button on input/output tabs
- [ ] Copy trace JSON to clipboard
- [ ] Copy as cURL works for OpenAI LLM spans
- [ ] All copy actions show brief "Copied!" feedback
- [ ] `npm run build` passes
