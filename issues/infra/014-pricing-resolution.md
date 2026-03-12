# Implement Pricing Resolution and Cost Calculation

**Labels:** `good first issue`, `backend`, `PRD-12`
**Difficulty:** Easy
**PRD:** [PRD-12: Cost & Pricing System](../../prds/PRD-12-cost-pricing.md) — Phase 1
**Depends on:** #013

## Summary

Implement a `PricingTable` struct with `resolve()` and `calculate()` methods that look up pricing rates for a given `(provider, model)` pair and compute cost from token counts. This is the core pricing logic that will be used by the proxy, the ingest API, and the backfill endpoint.

## Context

The only cost calculation in the codebase today is in `crates/daemon/src/ingest.rs:141-155` — a crude match statement with 3 hardcoded rates and a single `cost_per_mtok` with a 3x output multiplier. Real pricing needs separate input/output rates and glob-based model matching. This issue builds the proper resolution engine.

## What to do

1. Create `crates/trace/src/pricing.rs` (or extend the module from #013) with a `PricingTable`:

   ```rust
   pub struct PricingTable {
       /// Custom rules take priority over builtins
       custom: Vec<ModelPricing>,
       builtin: Vec<ModelPricing>,
   }

   impl PricingTable {
       /// Create a new table with builtins pre-loaded
       pub fn new() -> Self { ... }

       /// Add a custom pricing rule (overrides builtins)
       pub fn add_custom(&mut self, pricing: ModelPricing) { ... }

       /// Remove a custom pricing rule by ID
       pub fn remove_custom(&mut self, id: ModelPricingId) -> bool { ... }

       /// Resolve the effective rate for a (provider, model) pair
       pub fn resolve(&self, provider: &str, model: &str) -> Option<&ModelPricing> { ... }

       /// Calculate cost given token counts
       pub fn calculate(
           &self,
           provider: &str,
           model: &str,
           input_tokens: u64,
           output_tokens: u64,
       ) -> Option<f64> {
           let rate = self.resolve(provider, model)?;
           let input_cost = (input_tokens as f64 * rate.input_rate) / 1_000_000.0;
           let output_cost = (output_tokens as f64 * rate.output_rate) / 1_000_000.0;
           let request_cost = rate.per_request_cost.unwrap_or(0.0);
           Some(input_cost + output_cost + request_cost)
       }
   }
   ```

2. Implement the resolution priority (from PRD-12):
   1. **Exact match** in custom rules: `(provider, model)` matches exactly → use it
   2. **Glob match** in custom rules: `model_pattern` contains `*`, match with simple glob → use it
   3. **Exact match** in builtins → use it
   4. **Glob match** in builtins → use it
   5. **No match** → return `None`

3. Implement simple glob matching (just `*` suffix/prefix, not full glob):
   ```rust
   fn glob_matches(pattern: &str, value: &str) -> bool {
       if pattern == "*" { return true; }
       if let Some(prefix) = pattern.strip_suffix('*') {
           return value.starts_with(prefix);
       }
       if let Some(suffix) = pattern.strip_prefix('*') {
           return value.ends_with(suffix);
       }
       pattern == value
   }
   ```

## Files to modify

- `crates/trace/src/pricing.rs` — implement `PricingTable`, `resolve()`, `calculate()`, `glob_matches()`
- `crates/trace/src/lib.rs` — `pub mod pricing;` if not already exported

## Acceptance criteria

- [ ] `PricingTable::new()` loads all builtin rates
- [ ] Exact match: `resolve("openai", "gpt-4o")` returns the gpt-4o rate
- [ ] Glob match: `resolve("anthropic", "claude-3.5-sonnet-20241022")` matches `claude-3.5-sonnet*`
- [ ] Provider fallback: `resolve("ollama", "llama3.2")` matches `("ollama", "*")`
- [ ] Custom override: adding a custom rule for `("openai", "gpt-4o")` overrides the builtin
- [ ] Unknown model: `resolve("unknown", "mystery-model")` returns `None`
- [ ] `calculate()` returns correct cost: `calculate("openai", "gpt-4o", 1_000_000, 1_000_000)` = `Some(12.50)` ($2.50 input + $10.00 output)
- [ ] Unit tests for all resolution cases (`cargo test -p trace`)
- [ ] `cargo check -p trace` succeeds
