//! Auto-capture rule processing.
//!
//! When a span completes, enabled capture rules are evaluated against it.
//! Matching rules (subject to sampling) create new datapoints in the target dataset.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast;

use trace::{CaptureRule, Datapoint, DatapointKind, DatapointSource, Span};

use crate::events::EventLog;
use crate::org_store::SharedStore;
use crate::SystemEvent;

/// Evaluate all enabled capture rules against a completed span.
///
/// For each matching rule that passes the sample rate check, a new datapoint
/// is created in the rule's target dataset. This mirrors the logic in
/// `export_span_to_dataset` for extracting input/output from the span.
pub async fn process_capture_rules(
    store: &SharedStore,
    span: &Span,
    events_tx: &broadcast::Sender<SystemEvent>,
    event_log: &Arc<dyn EventLog>,
    org_id: &str,
) {
    // Collect matching rules under a read lock
    let matching_rules: Vec<CaptureRule> = {
        let r = store.read().await;
        r.all_enabled_capture_rules()
            .into_iter()
            .filter(|rule| rule.matches_span(span))
            .cloned()
            .collect()
    };

    if matching_rules.is_empty() {
        return;
    }

    for rule in matching_rules {
        // Apply sampling
        if rule.sample_rate < 1.0 && rand::random::<f64>() >= rule.sample_rate {
            continue;
        }

        // Check that the target dataset exists
        let dataset_exists = {
            let r = store.read().await;
            r.get_dataset(rule.dataset_id).is_some()
        };
        if !dataset_exists {
            tracing::warn!(
                rule_id = %rule.id,
                dataset_id = %rule.dataset_id,
                "capture rule target dataset not found, skipping"
            );
            continue;
        }

        // Create a datapoint from the span (same logic as export_span_to_dataset)
        let kind = DatapointKind::Generic {
            input: span.input().cloned().unwrap_or(serde_json::Value::Null),
            expected_output: span.output().cloned(),
            actual_output: None,
            score: None,
            metadata: HashMap::new(),
        };

        let dp = Datapoint::new(rule.dataset_id, kind, DatapointSource::SpanExport)
            .with_source_span(span.id());

        // Save datapoint and update captured count
        {
            let mut w = store.write().await;
            if let Err(e) = w.save_datapoint(dp.clone()).await {
                tracing::error!(rule_id = %rule.id, "capture: failed to save datapoint: {e}");
                continue;
            }

            // Increment the rule's captured_count
            if let Some(current_rule) = w.get_capture_rule(rule.id).cloned() {
                let mut updated = current_rule;
                updated.captured_count += 1;
                if let Err(e) = w.save_capture_rule(updated).await {
                    tracing::error!(rule_id = %rule.id, "capture: failed to update capture rule count: {e}");
                }
            }
        }

        tracing::debug!(
            rule_id = %rule.id,
            rule_name = %rule.name,
            span_id = %span.id(),
            dataset_id = %rule.dataset_id,
            "capture rule fired, created datapoint"
        );

        // Emit events (broadcast + durable log)
        let evt1 = SystemEvent::DatapointCreated { datapoint: dp.clone() };
        let _ = events_tx.send(evt1.clone());
        if let Err(e) = event_log.append(org_id, &evt1).await {
            tracing::warn!("failed to log DatapointCreated event: {e}");
        }
        let evt2 = SystemEvent::CaptureRuleFired { rule_id: rule.id, datapoint: dp };
        let _ = events_tx.send(evt2.clone());
        if let Err(e) = event_log.append(org_id, &evt2).await {
            tracing::warn!("failed to log CaptureRuleFired event: {e}");
        }
    }
}
