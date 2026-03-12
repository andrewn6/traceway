/**
 * Model pricing table for estimating LLM call costs from token counts.
 *
 * Prices are per million tokens (input / output) in USD.
 * Mirrors the Rust pricing table in crates/trace/src/pricing.rs.
 */

interface ModelPricing {
  input_per_mtok: number;
  output_per_mtok: number;
}

/**
 * Static pricing table. Longer prefixes listed first within each family
 * so prefix matching picks the most specific entry.
 */
const PRICING_TABLE: [string, ModelPricing][] = [
  // ── OpenAI ──────────────────────────────────────────────────────
  ["gpt-4o-mini", { input_per_mtok: 0.15, output_per_mtok: 0.60 }],
  ["gpt-4o-audio", { input_per_mtok: 2.50, output_per_mtok: 10.00 }],
  ["gpt-4o", { input_per_mtok: 2.50, output_per_mtok: 10.00 }],
  ["gpt-4.1-mini", { input_per_mtok: 0.40, output_per_mtok: 1.60 }],
  ["gpt-4.1-nano", { input_per_mtok: 0.10, output_per_mtok: 0.40 }],
  ["gpt-4.1", { input_per_mtok: 2.00, output_per_mtok: 8.00 }],
  ["gpt-4-turbo", { input_per_mtok: 10.00, output_per_mtok: 30.00 }],
  ["gpt-4-32k", { input_per_mtok: 60.00, output_per_mtok: 120.00 }],
  ["gpt-4", { input_per_mtok: 30.00, output_per_mtok: 60.00 }],
  ["gpt-3.5-turbo", { input_per_mtok: 0.50, output_per_mtok: 1.50 }],
  // o1 / o3 / o4 reasoning models
  ["o4-mini", { input_per_mtok: 1.10, output_per_mtok: 4.40 }],
  ["o3-mini", { input_per_mtok: 1.10, output_per_mtok: 4.40 }],
  ["o3", { input_per_mtok: 2.00, output_per_mtok: 8.00 }],
  ["o1-mini", { input_per_mtok: 1.10, output_per_mtok: 4.40 }],
  ["o1-preview", { input_per_mtok: 15.00, output_per_mtok: 60.00 }],
  ["o1", { input_per_mtok: 15.00, output_per_mtok: 60.00 }],
  // ── Anthropic ───────────────────────────────────────────────────
  ["claude-opus-4", { input_per_mtok: 15.00, output_per_mtok: 75.00 }],
  ["claude-sonnet-4", { input_per_mtok: 3.00, output_per_mtok: 15.00 }],
  ["claude-3.5-sonnet", { input_per_mtok: 3.00, output_per_mtok: 15.00 }],
  ["claude-3-5-sonnet", { input_per_mtok: 3.00, output_per_mtok: 15.00 }],
  ["claude-3.5-haiku", { input_per_mtok: 0.80, output_per_mtok: 4.00 }],
  ["claude-3-5-haiku", { input_per_mtok: 0.80, output_per_mtok: 4.00 }],
  ["claude-3-opus", { input_per_mtok: 15.00, output_per_mtok: 75.00 }],
  ["claude-3-sonnet", { input_per_mtok: 3.00, output_per_mtok: 15.00 }],
  ["claude-3-haiku", { input_per_mtok: 0.25, output_per_mtok: 1.25 }],
  // ── Google ──────────────────────────────────────────────────────
  ["gemini-2.5-pro", { input_per_mtok: 1.25, output_per_mtok: 10.00 }],
  ["gemini-2.5-flash", { input_per_mtok: 0.15, output_per_mtok: 0.60 }],
  ["gemini-2.0-flash", { input_per_mtok: 0.10, output_per_mtok: 0.40 }],
  ["gemini-1.5-pro", { input_per_mtok: 1.25, output_per_mtok: 5.00 }],
  ["gemini-1.5-flash", { input_per_mtok: 0.075, output_per_mtok: 0.30 }],
  ["gemini-pro", { input_per_mtok: 0.50, output_per_mtok: 1.50 }],
  // ── Mistral ─────────────────────────────────────────────────────
  ["mistral-large", { input_per_mtok: 2.00, output_per_mtok: 6.00 }],
  ["mistral-medium", { input_per_mtok: 2.70, output_per_mtok: 8.10 }],
  ["mistral-small", { input_per_mtok: 0.20, output_per_mtok: 0.60 }],
  ["codestral", { input_per_mtok: 0.30, output_per_mtok: 0.90 }],
  ["mixtral-8x7b", { input_per_mtok: 0.70, output_per_mtok: 0.70 }],
  ["mixtral-8x22b", { input_per_mtok: 2.00, output_per_mtok: 6.00 }],
  // ── Groq (hosted open models) ───────────────────────────────────
  ["llama-3.3-70b", { input_per_mtok: 0.59, output_per_mtok: 0.79 }],
  ["llama-3.1-70b", { input_per_mtok: 0.59, output_per_mtok: 0.79 }],
  ["llama-3.1-8b", { input_per_mtok: 0.05, output_per_mtok: 0.08 }],
  ["llama-3-70b", { input_per_mtok: 0.59, output_per_mtok: 0.79 }],
  ["llama-3-8b", { input_per_mtok: 0.05, output_per_mtok: 0.08 }],
  // ── Cohere ──────────────────────────────────────────────────────
  ["command-r-plus", { input_per_mtok: 2.50, output_per_mtok: 10.00 }],
  ["command-r", { input_per_mtok: 0.15, output_per_mtok: 0.60 }],
  // ── DeepSeek ────────────────────────────────────────────────────
  ["deepseek-chat", { input_per_mtok: 0.14, output_per_mtok: 0.28 }],
  ["deepseek-reasoner", { input_per_mtok: 0.55, output_per_mtok: 2.19 }],
  ["deepseek-coder", { input_per_mtok: 0.14, output_per_mtok: 0.28 }],
];

/** Look up pricing for a model by name using exact then prefix matching. */
function lookupPricing(model: string): ModelPricing | null {
  const lower = model.toLowerCase();
  for (const [prefix, pricing] of PRICING_TABLE) {
    if (lower === prefix) return pricing;
  }
  for (const [prefix, pricing] of PRICING_TABLE) {
    if (lower.startsWith(prefix)) return pricing;
  }
  return null;
}

/** Estimate cost in USD from model name and token counts. */
export function estimateCost(model: string, inputTokens: number, outputTokens: number): number | null {
  const pricing = lookupPricing(model);
  if (!pricing) return null;
  if (inputTokens === 0 && outputTokens === 0) return null;
  return (inputTokens * pricing.input_per_mtok + outputTokens * pricing.output_per_mtok) / 1_000_000;
}

/**
 * If the kind is an llm_call with tokens but no cost, estimate the cost
 * and return an enriched copy. Otherwise return the kind unchanged.
 */
export function enrichKindWithCost(kind: Record<string, unknown>): Record<string, unknown> {
  if (kind.type !== "llm_call") return kind;
  if (typeof kind.cost === "number" && kind.cost > 0) return kind;

  const model = typeof kind.model === "string" ? kind.model : null;
  if (!model) return kind;

  const inputTokens = typeof kind.input_tokens === "number" ? kind.input_tokens : 0;
  const outputTokens = typeof kind.output_tokens === "number" ? kind.output_tokens : 0;
  const estimated = estimateCost(model, inputTokens, outputTokens);
  if (estimated === null) return kind;

  return { ...kind, cost: estimated };
}
