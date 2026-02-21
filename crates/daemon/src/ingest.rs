//! Synthetic span ingest loop for development and smoke-testing.
//!
//! When the daemon is started with `--dev-ingest`, this module generates
//! realistic synthetic traces at a configurable interval. Each burst creates
//! a trace containing a mix of span kinds (LLM calls, file reads/writes,
//! custom spans) and exercises the full SpanStore -> PersistentStore ->
//! SQLite write path, including span completion and failure transitions.
//!
//! This is NOT enabled in normal operation -- it exists purely to verify
//! that storage, logging, and the API can serve live-updating data before
//! real ingest sources (memfs, proxy, SDK) are wired up.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{watch, RwLock};
use tracing::{debug, info, warn};

use api::AnyBackend;
use storage::PersistentStore;
use trace::{SpanBuilder, SpanKind, Trace};

/// Models used in synthetic LLM call spans.
const MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "claude-sonnet-4-20250514",
    "llama3.1:8b",
    "deepseek-r1:14b",
];

/// Providers for synthetic LLM call spans.
const PROVIDERS: &[&str] = &["openai", "anthropic", "ollama", "ollama"];

/// File paths for synthetic fs spans.
const FILE_PATHS: &[&str] = &[
    "/workspace/src/main.py",
    "/workspace/src/utils.py",
    "/workspace/data/input.json",
    "/workspace/data/output.csv",
    "/workspace/config.toml",
    "/workspace/README.md",
];

/// Trace name templates.
const TRACE_NAMES: &[&str] = &[
    "code-review",
    "refactor-session",
    "bug-investigation",
    "feature-impl",
    "test-generation",
    "doc-update",
];

/// Simple pseudo-random using the cycle counter. Not cryptographic, just enough
/// to vary synthetic data without pulling in `rand`.
fn cheap_random(seed: &mut u64) -> u64 {
    // xorshift64
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn pick<'a>(items: &'a [&str], seed: &mut u64) -> &'a str {
    let idx = (cheap_random(seed) as usize) % items.len();
    items[idx]
}

/// Run the synthetic ingest loop until shutdown is signalled.
pub async fn run_synthetic_ingest(
    store: Arc<RwLock<PersistentStore<AnyBackend>>>,
    interval: Duration,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut seed: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let mut burst_count: u64 = 0;

    loop {
        // Wait for interval or shutdown
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = shutdown_rx.changed() => {
                info!("synthetic ingest loop shutting down");
                return;
            }
        }

        burst_count += 1;
        let trace_name = pick(TRACE_NAMES, &mut seed);

        info!(
            burst = burst_count,
            trace_name,
            "generating synthetic trace"
        );

        if let Err(e) = generate_trace(&store, &mut seed, trace_name).await {
            warn!(burst = burst_count, "synthetic ingest error: {}", e);
        }
    }
}

async fn generate_trace(
    store: &Arc<RwLock<PersistentStore<AnyBackend>>>,
    seed: &mut u64,
    trace_name: &str,
) -> Result<(), String> {
    // Create a trace
    let trace = Trace::new(Some(trace_name.to_string()))
        .with_tags(vec!["synthetic".to_string(), "dev".to_string()]);
    let trace_id = trace.id;

    {
        let mut s = store.write().await;
        s.save_trace(trace).await;
    }

    debug!(%trace_id, name = trace_name, "created synthetic trace");

    // Generate 2-6 spans per trace
    let span_count = 2 + (cheap_random(seed) % 5) as usize;

    let mut parent_span_id = None;

    for i in 0..span_count {
        // Vary the span kind
        let kind_roll = cheap_random(seed) % 100;

        let (name, kind) = if kind_roll < 50 {
            // LLM call (50%)
            let model = pick(MODELS, seed);
            let provider_idx = (cheap_random(seed) as usize) % PROVIDERS.len();
            let provider = PROVIDERS[provider_idx];
            let input_tokens = 100 + (cheap_random(seed) % 4000);
            let output_tokens = 50 + (cheap_random(seed) % 2000);
            let cost_per_mtok = match model {
                "gpt-4o" => 5.0,
                "gpt-4o-mini" => 0.15,
                m if m.starts_with("claude") => 3.0,
                _ => 0.0, // local models
            };
            let cost = if cost_per_mtok > 0.0 {
                Some(
                    (input_tokens as f64 * cost_per_mtok
                        + output_tokens as f64 * cost_per_mtok * 3.0)
                        / 1_000_000.0,
                )
            } else {
                None
            };

            (
                format!("llm-call-{}", i),
                SpanKind::LlmCall {
                    model: model.to_string(),
                    provider: Some(provider.to_string()),
                    input_tokens: Some(input_tokens),
                    output_tokens: Some(output_tokens),
                    cost,
                    input_preview: Some("What is the meaning of life?".to_string()),
                    output_preview: Some("The meaning of life is...".to_string()),
                },
            )
        } else if kind_roll < 75 {
            // File read (25%)
            let path = pick(FILE_PATHS, seed);
            let bytes = 256 + (cheap_random(seed) % 65536);
            (
                format!("read-{}", path.rsplit('/').next().unwrap_or("file")),
                SpanKind::FsRead {
                    path: path.to_string(),
                    file_version: None,
                    bytes_read: bytes,
                },
            )
        } else if kind_roll < 90 {
            // File write (15%)
            let path = pick(FILE_PATHS, seed);
            let bytes = 128 + (cheap_random(seed) % 32768);
            let hash = format!("{:016x}", cheap_random(seed));
            (
                format!("write-{}", path.rsplit('/').next().unwrap_or("file")),
                SpanKind::FsWrite {
                    path: path.to_string(),
                    file_version: hash,
                    bytes_written: bytes,
                },
            )
        } else {
            // Custom span (10%)
            let mut attrs = std::collections::HashMap::new();
            attrs.insert(
                "iteration".to_string(),
                serde_json::Value::Number(serde_json::Number::from(i as u64)),
            );
            (
                format!("custom-step-{}", i),
                SpanKind::Custom {
                    kind: "tool_call".to_string(),
                    attributes: attrs,
                },
            )
        };

        // Build the span, optionally parented to the first span
        let mut builder = SpanBuilder::new(trace_id, &name, kind);
        if let Some(pid) = parent_span_id {
            builder = builder.parent(pid);
        }
        builder = builder.input(serde_json::json!({"synthetic": true, "step": i}));
        let span = builder.build();
        let span_id = span.id();

        {
            let mut s = store.write().await;
            s.insert(span).await;
        }

        debug!(%trace_id, %span_id, span_name = name, "inserted synthetic span");

        // First span becomes the parent for the rest (flat hierarchy under root)
        if i == 0 {
            parent_span_id = Some(span_id);
        }

        // Simulate some work time
        tokio::time::sleep(Duration::from_millis(10 + (cheap_random(seed) % 50))).await;

        // Complete or fail the span
        let fail_roll = cheap_random(seed) % 100;
        {
            let mut s = store.write().await;
            if fail_roll < 10 {
                // 10% failure rate
                s.fail_span(span_id, "synthetic error: something went wrong")
                    .await;
                debug!(%span_id, "failed synthetic span");
            } else {
                s.complete_span(
                    span_id,
                    Some(serde_json::json!({"synthetic": true, "result": "ok"})),
                )
                .await;
                debug!(%span_id, "completed synthetic span");
            }
        }
    }

    // Complete the trace
    {
        let mut s = store.write().await;
        if let Some(trace) = s.get_trace(trace_id).cloned() {
            s.save_trace(trace.complete()).await;
        }
    }

    // Log summary
    {
        let s = store.read().await;
        info!(
            %trace_id,
            name = trace_name,
            spans = span_count,
            total_spans = s.span_count(),
            total_traces = s.trace_count(),
            "synthetic trace complete"
        );
    }

    Ok(())
}
