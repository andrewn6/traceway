//! Model pricing table for estimating LLM call costs from token counts.
//!
//! Prices are per million tokens (input / output) in USD.
//! When a model isn't found, we try prefix matching (e.g. "gpt-4o-2024-08-06"
//! matches the "gpt-4o" entry). Returns None if no match is found so the caller
//! can decide whether to leave cost as None or use a fallback.

/// Per-million-token pricing for a model.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    /// Price per 1M input tokens in USD
    pub input_per_mtok: f64,
    /// Price per 1M output tokens in USD
    pub output_per_mtok: f64,
}

/// Static pricing table. Sorted longest-prefix-first so we can do prefix matching.
/// Prices sourced from provider pricing pages (as of early 2025).
static PRICING_TABLE: &[(&str, ModelPricing)] = &[
    // ── OpenAI ──────────────────────────────────────────────────────
    // GPT-4o
    (
        "gpt-4o-mini",
        ModelPricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
        },
    ),
    (
        "gpt-4o-audio",
        ModelPricing {
            input_per_mtok: 2.50,
            output_per_mtok: 10.00,
        },
    ),
    (
        "gpt-4o",
        ModelPricing {
            input_per_mtok: 2.50,
            output_per_mtok: 10.00,
        },
    ),
    // GPT-4.1 family
    (
        "gpt-4.1-mini",
        ModelPricing {
            input_per_mtok: 0.40,
            output_per_mtok: 1.60,
        },
    ),
    (
        "gpt-4.1-nano",
        ModelPricing {
            input_per_mtok: 0.10,
            output_per_mtok: 0.40,
        },
    ),
    (
        "gpt-4.1",
        ModelPricing {
            input_per_mtok: 2.00,
            output_per_mtok: 8.00,
        },
    ),
    // GPT-4 / GPT-4 Turbo
    (
        "gpt-4-turbo",
        ModelPricing {
            input_per_mtok: 10.00,
            output_per_mtok: 30.00,
        },
    ),
    (
        "gpt-4-32k",
        ModelPricing {
            input_per_mtok: 60.00,
            output_per_mtok: 120.00,
        },
    ),
    (
        "gpt-4",
        ModelPricing {
            input_per_mtok: 30.00,
            output_per_mtok: 60.00,
        },
    ),
    // GPT-3.5
    (
        "gpt-3.5-turbo",
        ModelPricing {
            input_per_mtok: 0.50,
            output_per_mtok: 1.50,
        },
    ),
    // o1 / o3 / o4 reasoning models
    (
        "o4-mini",
        ModelPricing {
            input_per_mtok: 1.10,
            output_per_mtok: 4.40,
        },
    ),
    (
        "o3-mini",
        ModelPricing {
            input_per_mtok: 1.10,
            output_per_mtok: 4.40,
        },
    ),
    (
        "o3",
        ModelPricing {
            input_per_mtok: 2.00,
            output_per_mtok: 8.00,
        },
    ),
    (
        "o1-mini",
        ModelPricing {
            input_per_mtok: 1.10,
            output_per_mtok: 4.40,
        },
    ),
    (
        "o1-preview",
        ModelPricing {
            input_per_mtok: 15.00,
            output_per_mtok: 60.00,
        },
    ),
    (
        "o1",
        ModelPricing {
            input_per_mtok: 15.00,
            output_per_mtok: 60.00,
        },
    ),
    // ── Anthropic ───────────────────────────────────────────────────
    (
        "claude-opus-4",
        ModelPricing {
            input_per_mtok: 15.00,
            output_per_mtok: 75.00,
        },
    ),
    (
        "claude-sonnet-4",
        ModelPricing {
            input_per_mtok: 3.00,
            output_per_mtok: 15.00,
        },
    ),
    (
        "claude-3.5-sonnet",
        ModelPricing {
            input_per_mtok: 3.00,
            output_per_mtok: 15.00,
        },
    ),
    (
        "claude-3-5-sonnet",
        ModelPricing {
            input_per_mtok: 3.00,
            output_per_mtok: 15.00,
        },
    ),
    (
        "claude-3.5-haiku",
        ModelPricing {
            input_per_mtok: 0.80,
            output_per_mtok: 4.00,
        },
    ),
    (
        "claude-3-5-haiku",
        ModelPricing {
            input_per_mtok: 0.80,
            output_per_mtok: 4.00,
        },
    ),
    (
        "claude-3-opus",
        ModelPricing {
            input_per_mtok: 15.00,
            output_per_mtok: 75.00,
        },
    ),
    (
        "claude-3-sonnet",
        ModelPricing {
            input_per_mtok: 3.00,
            output_per_mtok: 15.00,
        },
    ),
    (
        "claude-3-haiku",
        ModelPricing {
            input_per_mtok: 0.25,
            output_per_mtok: 1.25,
        },
    ),
    // ── Google ──────────────────────────────────────────────────────
    (
        "gemini-2.5-pro",
        ModelPricing {
            input_per_mtok: 1.25,
            output_per_mtok: 10.00,
        },
    ),
    (
        "gemini-2.5-flash",
        ModelPricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
        },
    ),
    (
        "gemini-2.0-flash",
        ModelPricing {
            input_per_mtok: 0.10,
            output_per_mtok: 0.40,
        },
    ),
    (
        "gemini-1.5-pro",
        ModelPricing {
            input_per_mtok: 1.25,
            output_per_mtok: 5.00,
        },
    ),
    (
        "gemini-1.5-flash",
        ModelPricing {
            input_per_mtok: 0.075,
            output_per_mtok: 0.30,
        },
    ),
    (
        "gemini-pro",
        ModelPricing {
            input_per_mtok: 0.50,
            output_per_mtok: 1.50,
        },
    ),
    // ── Mistral ─────────────────────────────────────────────────────
    (
        "mistral-large",
        ModelPricing {
            input_per_mtok: 2.00,
            output_per_mtok: 6.00,
        },
    ),
    (
        "mistral-medium",
        ModelPricing {
            input_per_mtok: 2.70,
            output_per_mtok: 8.10,
        },
    ),
    (
        "mistral-small",
        ModelPricing {
            input_per_mtok: 0.20,
            output_per_mtok: 0.60,
        },
    ),
    (
        "codestral",
        ModelPricing {
            input_per_mtok: 0.30,
            output_per_mtok: 0.90,
        },
    ),
    (
        "mixtral-8x7b",
        ModelPricing {
            input_per_mtok: 0.70,
            output_per_mtok: 0.70,
        },
    ),
    (
        "mixtral-8x22b",
        ModelPricing {
            input_per_mtok: 2.00,
            output_per_mtok: 6.00,
        },
    ),
    // ── Groq (hosted open models — Groq pricing) ───────────────────
    (
        "llama-3.3-70b",
        ModelPricing {
            input_per_mtok: 0.59,
            output_per_mtok: 0.79,
        },
    ),
    (
        "llama-3.1-70b",
        ModelPricing {
            input_per_mtok: 0.59,
            output_per_mtok: 0.79,
        },
    ),
    (
        "llama-3.1-8b",
        ModelPricing {
            input_per_mtok: 0.05,
            output_per_mtok: 0.08,
        },
    ),
    (
        "llama-3-70b",
        ModelPricing {
            input_per_mtok: 0.59,
            output_per_mtok: 0.79,
        },
    ),
    (
        "llama-3-8b",
        ModelPricing {
            input_per_mtok: 0.05,
            output_per_mtok: 0.08,
        },
    ),
    // ── Cohere ──────────────────────────────────────────────────────
    (
        "command-r-plus",
        ModelPricing {
            input_per_mtok: 2.50,
            output_per_mtok: 10.00,
        },
    ),
    (
        "command-r",
        ModelPricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
        },
    ),
    // ── DeepSeek ────────────────────────────────────────────────────
    (
        "deepseek-chat",
        ModelPricing {
            input_per_mtok: 0.14,
            output_per_mtok: 0.28,
        },
    ),
    (
        "deepseek-reasoner",
        ModelPricing {
            input_per_mtok: 0.55,
            output_per_mtok: 2.19,
        },
    ),
    (
        "deepseek-coder",
        ModelPricing {
            input_per_mtok: 0.14,
            output_per_mtok: 0.28,
        },
    ),
];

/// Look up pricing for a model by name. Uses prefix matching:
/// "gpt-4o-2024-08-06" will match "gpt-4o".
pub fn lookup_pricing(model: &str) -> Option<ModelPricing> {
    let model_lower = model.to_lowercase();

    // Exact match first
    for &(prefix, pricing) in PRICING_TABLE {
        if model_lower == prefix {
            return Some(pricing);
        }
    }

    // Prefix match (e.g. "gpt-4o-2024-08-06" matches "gpt-4o")
    // We iterate the table which has longer prefixes first within each family,
    // so "gpt-4o-mini" matches before "gpt-4o".
    for &(prefix, pricing) in PRICING_TABLE {
        if model_lower.starts_with(prefix) {
            return Some(pricing);
        }
    }

    None
}

/// Estimate cost in USD from model name and token counts.
/// Returns None if the model is not in the pricing table or no tokens are provided.
pub fn estimate_cost(
    model: &str,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
) -> Option<f64> {
    let pricing = lookup_pricing(model)?;
    let inp = input_tokens.unwrap_or(0) as f64;
    let out = output_tokens.unwrap_or(0) as f64;
    if inp == 0.0 && out == 0.0 {
        return None;
    }
    Some((inp * pricing.input_per_mtok + out * pricing.output_per_mtok) / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let p = lookup_pricing("gpt-4o").unwrap();
        assert_eq!(p.input_per_mtok, 2.50);
        assert_eq!(p.output_per_mtok, 10.00);
    }

    #[test]
    fn test_prefix_match() {
        let p = lookup_pricing("gpt-4o-2024-08-06").unwrap();
        assert_eq!(p.input_per_mtok, 2.50);
    }

    #[test]
    fn test_mini_before_base() {
        // "gpt-4o-mini" should match the mini pricing, not gpt-4o
        let p = lookup_pricing("gpt-4o-mini").unwrap();
        assert_eq!(p.input_per_mtok, 0.15);
    }

    #[test]
    fn test_claude() {
        let p = lookup_pricing("claude-sonnet-4-20250514").unwrap();
        assert_eq!(p.input_per_mtok, 3.00);
    }

    #[test]
    fn test_unknown_model() {
        assert!(lookup_pricing("my-custom-model").is_none());
    }

    #[test]
    fn test_estimate_cost() {
        // gpt-4o: $2.50 input, $10.00 output per Mtok
        // 1000 input + 500 output
        let cost = estimate_cost("gpt-4o", Some(1000), Some(500)).unwrap();
        let expected = (1000.0 * 2.50 + 500.0 * 10.00) / 1_000_000.0;
        assert!((cost - expected).abs() < 1e-10);
    }

    #[test]
    fn test_estimate_cost_no_tokens() {
        assert!(estimate_cost("gpt-4o", None, None).is_none());
        assert!(estimate_cost("gpt-4o", Some(0), Some(0)).is_none());
    }

    #[test]
    fn test_case_insensitive() {
        assert!(lookup_pricing("GPT-4o").is_some());
        assert!(lookup_pricing("Claude-3-Opus").is_some());
    }
}
