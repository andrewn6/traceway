# Add Retroactive Cost Backfill Endpoint

**Labels:** `good first issue`, `backend`, `PRD-12`
**Difficulty:** Easy
**PRD:** [PRD-12: Cost & Pricing System](../../prds/PRD-12-cost-pricing.md) — Phase 9
**Depends on:** #014, #017

## Summary

Add a `POST /api/pricing/:id/backfill` endpoint that recalculates cost for all existing spans matching a pricing rule's model that currently have `cost: None` but have token counts. This lets users retroactively fill in cost data after adding or updating pricing rules.

## Context

Before the pricing system exists, the proxy stores all spans with `cost: None` (`crates/proxy/src/lib.rs:226`). When a user first configures pricing, all historical spans lack cost data. This one-off backfill endpoint walks through spans, finds matching ones with tokens but no cost, and fills in the calculated cost.

## What to do

### 1. Add the route

In `crates/api/src/lib.rs`:

```rust
.route("/api/pricing/:id/backfill", post(backfill_pricing))
```

### 2. Implement the handler

```rust
async fn backfill_pricing(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Path(pricing_id): Path<ModelPricingId>,
) -> Result<Json<BackfillResponse>, (StatusCode, Json<Value>)> {
    require_scope(&ctx, auth::Scope::TracesWrite)?;

    let store = state.store_for_org(ctx.org_id).await.map_err(store_err_json)?;
    let r = store.read().await;

    // Get the pricing rule to know which provider/model to match
    let pricing = r.get_model_pricing(pricing_id)
        .ok_or((StatusCode::NOT_FOUND, json_err("pricing rule not found")))?;

    // Find all LlmCall spans with matching model, tokens present, cost = None
    let spans = r.list_all_spans(); // or iterate the cache
    let mut updated = 0;
    drop(r);

    let mut w = store.write().await;
    for span in spans {
        if let SpanKind::LlmCall { ref model, ref provider, input_tokens: Some(inp), output_tokens: Some(out), cost: None, .. } = span.kind() {
            let provider_str = provider.as_deref().unwrap_or("unknown");
            if glob_matches(&pricing.model_pattern, model) && pricing.provider == provider_str {
                let calculated = pricing_table.calculate(provider_str, model, inp, out);
                if let Some(cost) = calculated {
                    w.set_span_cost(span.id(), cost).await;
                    updated += 1;
                }
            }
        }
    }
    drop(w);

    Ok(Json(BackfillResponse { updated }))
}

#[derive(Serialize)]
struct BackfillResponse {
    updated: usize,
}
```

### 3. Add helper method

You'll likely need a `set_span_cost()` method on `PersistentStore` that updates just the cost field of a span's `SpanKind::LlmCall` without replacing the entire span. Alternatively, clone the span with the new cost and re-save it.

### 4. Background execution (optional)

For large datasets, this could block for a while. If the job queue from PRD-07 (#008) is available, run it as a background job. If not, run synchronously — the endpoint just takes longer. Add a note in the response if it took > 5 seconds.

## Files to modify

- `crates/api/src/lib.rs` — add route + handler
- `crates/storage/src/lib.rs` — add `set_span_cost()` helper if needed
- `crates/trace/src/lib.rs` — add cost mutation method on `Span`/`SpanKind` if needed

## Acceptance criteria

- [ ] `POST /api/pricing/:id/backfill` returns `{ "updated": N }` with count of updated spans
- [ ] Only spans with `cost: None` and matching `(provider, model)` are updated
- [ ] Spans that already have cost are NOT overwritten
- [ ] Spans without tokens are skipped (can't calculate cost without tokens)
- [ ] Non-existent pricing ID returns 404
- [ ] `cargo check -p api` succeeds
- [ ] Manual test: create spans with tokens but no cost, add a pricing rule, run backfill, verify cost is populated
