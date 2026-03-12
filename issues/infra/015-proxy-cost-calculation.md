# Wire Cost Calculation into Proxy

**Labels:** `enhancement`, `backend`, `PRD-12`
**Difficulty:** Medium
**PRD:** [PRD-12: Cost & Pricing System](../../prds/PRD-12-cost-pricing.md) — Phases 2-3
**Depends on:** #014

## Summary

Update the proxy (`crates/proxy/src/lib.rs`) to calculate cost using the `PricingTable` when completing spans, instead of always setting `cost: None`. Also update the span ingest API (`crates/api/src/lib.rs`) to enrich SDK-submitted spans with server-side cost when the client didn't calculate it.

## Context

The proxy extracts tokens correctly from LLM responses (`extract_tokens` at `crates/proxy/src/lib.rs:53-86`) but then hardcodes `cost: None` in the updated span kind (`crates/proxy/src/lib.rs:221-229`). The ingest API (`crates/api/src/lib.rs:772-816`) stores whatever the SDK sends as-is with no enrichment. After this issue, both paths produce cost data for every span where tokens and pricing are available.

## What to do

### 1. Make PricingTable accessible to the proxy

The proxy needs a reference to a shared `PricingTable`. Options:
- Add `pricing: Arc<PricingTable>` to the proxy's state/config
- Or pass it through whatever context the proxy uses when building spans

The `PricingTable` should be initialized at startup from builtins + any custom rules loaded from storage.

### 2. Update proxy span completion

In `crates/proxy/src/lib.rs`, around line 221-229 where the updated span kind is built after receiving the LLM response:

```rust
// Before (current):
let updated_kind = SpanKind::LlmCall {
    model: model.clone(),
    provider: provider.clone(),
    input_tokens,
    output_tokens,
    cost: None,                          // <-- always None
    input_preview: input_preview.clone(),
    output_preview,
};

// After:
let cost = match (input_tokens, output_tokens) {
    (Some(inp), Some(out)) => {
        let provider_str = provider.as_deref().unwrap_or("unknown");
        pricing.calculate(provider_str, &model, inp, out)
    }
    _ => None,
};
let updated_kind = SpanKind::LlmCall {
    model: model.clone(),
    provider: provider.clone(),
    input_tokens,
    output_tokens,
    cost,                                 // <-- calculated
    input_preview: input_preview.clone(),
    output_preview,
};
```

### 3. Update the ingest API for SDK spans

In `crates/api/src/lib.rs`, the `create_span` handler (line ~772) or the `complete_span` handler (line ~832): if the span's `SpanKind::LlmCall` has `cost: None` but has tokens, calculate cost server-side.

```rust
// In create_span or complete_span, after building/receiving the span:
if span.kind().cost().is_none() {
    if let SpanKind::LlmCall { ref model, ref provider, input_tokens: Some(inp), output_tokens: Some(out), .. } = span.kind() {
        let provider_str = provider.as_deref().unwrap_or("unknown");
        if let Some(cost) = pricing.calculate(provider_str, model, inp, out) {
            span.set_cost(Some(cost));
        }
    }
}
```

Note: `Span` fields are private and accessed via getters. You may need to add a `set_cost` method or modify the span kind through the existing builder/mutation pattern.

### 4. Respect client-provided cost

If the SDK already provided a non-None cost, keep it. The server only fills in cost when it's missing. This allows clients with more accurate rates to override the server's pricing.

## Files to modify

- `crates/proxy/src/lib.rs` — inject `PricingTable`, calculate cost on span completion
- `crates/api/src/lib.rs` — enrich SDK spans with cost in create/complete handlers
- `crates/trace/src/lib.rs` — may need `set_cost()` or a cost mutation method on `Span`/`SpanKind`

## Acceptance criteria

- [ ] Proxy-routed LLM calls to known models have `cost != None` in their completed spans
- [ ] SDK-submitted spans with tokens but no cost get server-calculated cost
- [ ] SDK-submitted spans with explicit cost keep the client's value (not overwritten)
- [ ] Unknown models (no pricing match) still get `cost: None` (no crash, no guess)
- [ ] Dashboard shows real cost data for proxy traffic (not $0.00)
- [ ] `cargo check -p proxy` and `cargo check -p api` succeed
