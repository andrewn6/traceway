# Add Pricing CRUD API Endpoints

**Labels:** `enhancement`, `backend`, `PRD-12`
**Difficulty:** Medium
**PRD:** [PRD-12: Cost & Pricing System](../../prds/PRD-12-cost-pricing.md) — Phase 4
**Depends on:** #013, #017

## Summary

Add REST API endpoints for managing model pricing rules: list all pricing (builtins + custom), create custom rules, update custom rules, delete custom rules, and resolve the effective rate for a given model. These endpoints power both the pricing table UI and programmatic pricing management.

## Context

There is currently no pricing API. The `crates/api/src/lib.rs` router has endpoints for traces, spans, datasets, eval runs, capture rules, and provider connections — but nothing for pricing. This follows the same handler pattern as existing CRUD endpoints (auth → scope check → store lock → persist → return JSON).

## What to do

### 1. Add routes to the API router

In `crates/api/src/lib.rs`, register the new routes:

```rust
.route("/api/pricing", get(list_pricing).post(create_pricing))
.route("/api/pricing/:id", put(update_pricing).delete(delete_pricing))
.route("/api/pricing/resolve", get(resolve_pricing))
```

### 2. Implement handlers

```rust
/// GET /api/pricing — returns all pricing rules (builtins + custom)
async fn list_pricing(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
) -> Result<Json<Vec<ModelPricing>>, (StatusCode, Json<Value>)> {
    require_scope(&ctx, auth::Scope::TracesRead)?;
    // Return builtins + custom rules from PricingTable
    // Mark each with source: Builtin or Custom
}

/// POST /api/pricing — create a custom pricing rule
async fn create_pricing(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Json(req): Json<CreatePricingRequest>,
) -> Result<(StatusCode, Json<ModelPricing>), (StatusCode, Json<Value>)> {
    require_scope(&ctx, auth::Scope::TracesWrite)?;
    // Validate: provider not empty, model_pattern not empty, rates >= 0
    // Create ModelPricing with source: Custom
    // Persist to storage + add to PricingTable
    // Return 201 Created
}

/// PUT /api/pricing/:id — update a custom rule
async fn update_pricing(...) {
    // Can only update Custom rules, not Builtin
    // Return 403 if trying to update a builtin
}

/// DELETE /api/pricing/:id — delete a custom rule
async fn delete_pricing(...) {
    // Can only delete Custom rules
    // Return 403 if trying to delete a builtin
}

/// GET /api/pricing/resolve?provider=X&model=Y — resolve effective rate
async fn resolve_pricing(
    auth::Auth(ctx): auth::Auth,
    State(state): State<AppState>,
    Query(params): Query<ResolvePricingParams>,
) -> Result<Json<Option<ModelPricing>>, (StatusCode, Json<Value>)> {
    require_scope(&ctx, auth::Scope::TracesRead)?;
    // Use PricingTable::resolve(provider, model)
    // Return the matched pricing rule or null
}
```

### 3. Request/response types

```rust
#[derive(Deserialize)]
pub struct CreatePricingRequest {
    pub provider: String,
    pub model_pattern: String,
    pub input_rate: f64,
    pub output_rate: f64,
    pub per_request_cost: Option<f64>,
}

#[derive(Deserialize)]
pub struct ResolvePricingParams {
    pub provider: String,
    pub model: String,
}
```

### 4. PricingTable on AppState

The `PricingTable` needs to live on the `AppState` (or be accessible from org stores). Since pricing is likely global (not per-org), consider storing it as `Arc<RwLock<PricingTable>>` on `AppState` directly.

## Files to modify

- `crates/api/src/lib.rs` — add routes, handlers, request/response types
- Possibly `crates/api/src/org_store.rs` — if pricing is per-org

## Acceptance criteria

- [ ] `GET /api/pricing` returns all builtins + custom rules with `source` field
- [ ] `POST /api/pricing` creates a custom rule, returns 201
- [ ] `PUT /api/pricing/:id` updates a custom rule, returns 403 for builtins
- [ ] `DELETE /api/pricing/:id` deletes a custom rule, returns 403 for builtins
- [ ] `GET /api/pricing/resolve?provider=openai&model=gpt-4o` returns the matching rate
- [ ] Input validation: negative rates rejected, empty provider/model rejected
- [ ] New custom rules are immediately effective for cost calculation (proxy picks them up)
- [ ] `cargo check -p api` succeeds
