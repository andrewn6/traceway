# Add ModelPricing Types and Builtin Pricing Table

**Labels:** `good first issue`, `backend`, `PRD-12`
**Difficulty:** Easy
**PRD:** [PRD-12: Cost & Pricing System](../../prds/PRD-12-cost-pricing.md) — Phase 1

## Summary

Add the `ModelPricing`, `PricingSource`, and `ModelPricingId` types to `crates/trace/src/lib.rs`, and create a `BUILTIN_PRICING` constant array with rates for popular OpenAI, Anthropic, Google, and Ollama models. These are the foundational data types that all other cost/pricing work depends on.

## Context

Traceway currently has no concept of a pricing table. The only pricing logic is 3 hardcoded rates in the synthetic data generator (`crates/daemon/src/ingest.rs:141-155`), which uses a crude single rate with a 3x output multiplier. The actual proxy never calculates cost — it sets `cost: None` on every span (`crates/proxy/src/lib.rs:226`). This issue adds the types needed to store real per-model input/output rates.

## What to do

1. Add `ModelPricingId` to the ID types in `crates/trace/src/lib.rs` (follow the existing pattern for `DatasetId`, `EvalRunId`, etc.):

   ```rust
   id_type!(ModelPricingId);
   ```

2. Add the `ModelPricing` struct and `PricingSource` enum:

   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ModelPricing {
       pub id: ModelPricingId,
       pub provider: String,             // "openai", "anthropic", "google", "ollama"
       pub model_pattern: String,        // exact match or glob: "gpt-4o", "claude-*"
       pub input_rate: f64,              // cost per 1M input tokens (USD)
       pub output_rate: f64,             // cost per 1M output tokens (USD)
       pub per_request_cost: Option<f64>,// fixed per-request cost (some APIs charge this)
       pub effective_date: DateTime<Utc>,
       pub source: PricingSource,
       pub created_at: DateTime<Utc>,
       pub updated_at: DateTime<Utc>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
   pub enum PricingSource {
       Builtin,
       Custom,
   }
   ```

3. Add the `BUILTIN_PRICING` constant (put this in a new `crates/trace/src/pricing.rs` module or directly in `lib.rs` — contributor's choice):

   ```rust
   /// (provider, model_pattern, input_rate_per_1M, output_rate_per_1M)
   pub const BUILTIN_PRICING: &[(&str, &str, f64, f64)] = &[
       // OpenAI
       ("openai", "gpt-4o",            2.50,  10.00),
       ("openai", "gpt-4o-mini",       0.15,   0.60),
       ("openai", "gpt-4.1",           2.00,   8.00),
       ("openai", "gpt-4.1-mini",      0.40,   1.60),
       ("openai", "gpt-4.1-nano",      0.10,   0.40),
       ("openai", "o1",               15.00,  60.00),
       ("openai", "o1-mini",           1.10,   4.40),
       ("openai", "o3",               10.00,  40.00),
       ("openai", "o3-mini",           1.10,   4.40),
       ("openai", "o4-mini",           1.10,   4.40),
       // Anthropic
       ("anthropic", "claude-sonnet-4-20250514",   3.00,  15.00),
       ("anthropic", "claude-3.5-sonnet*", 3.00,  15.00),
       ("anthropic", "claude-3.5-haiku*",  0.80,   4.00),
       ("anthropic", "claude-opus-4-20250514",    15.00,  75.00),
       // Google
       ("google", "gemini-2.5-pro*",   1.25,  10.00),
       ("google", "gemini-2.5-flash*", 0.15,   0.60),
       ("google", "gemini-2.0-flash*", 0.10,   0.40),
       // Local (free)
       ("ollama", "*",                 0.00,   0.00),
   ];
   ```

## Files to modify

- `crates/trace/src/lib.rs` — add `ModelPricingId`, `ModelPricing`, `PricingSource`
- Optionally `crates/trace/src/pricing.rs` — if putting builtin table in a submodule

## Acceptance criteria

- [ ] `ModelPricing` and `PricingSource` compile with Serialize/Deserialize (`cargo check -p trace`)
- [ ] `ModelPricingId` follows the same UUIDv7 pattern as other ID types
- [ ] `BUILTIN_PRICING` covers at least 18 model entries across 4 providers
- [ ] No changes to existing types (no breaking changes)
