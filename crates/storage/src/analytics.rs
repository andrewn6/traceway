use std::collections::HashMap;

use trace::{
    AnalyticsGroup, AnalyticsMetric, AnalyticsQuery, AnalyticsResponse, AnalyticsSummary,
    GroupByField, MetricValues, ModelCost, ModelTokens, Span, SpanStatus,
};

/// Compute analytics from a set of spans according to the query.
pub fn compute_analytics(spans: &[&Span], query: &AnalyticsQuery) -> AnalyticsResponse {
    // Accumulator per group
    struct Acc {
        cost: f64,
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
        latency_sum_ms: f64,
        latency_count: u64,
        span_count: u64,
        error_count: u64,
    }

    impl Acc {
        fn new() -> Self {
            Self {
                cost: 0.0,
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                latency_sum_ms: 0.0,
                latency_count: 0,
                span_count: 0,
                error_count: 0,
            }
        }

        fn accumulate(&mut self, span: &Span) {
            self.span_count += 1;
            if matches!(span.status(), SpanStatus::Failed { .. }) {
                self.error_count += 1;
            }
            if let Some(ms) = span.duration_ms() {
                self.latency_sum_ms += ms as f64;
                self.latency_count += 1;
            }
            if let Some(c) = span.kind().cost() {
                self.cost += c;
            }
            if let Some(t) = span.kind().input_tokens() {
                self.input_tokens += t;
            }
            if let Some(t) = span.kind().output_tokens() {
                self.output_tokens += t;
            }
            if let Some(t) = span.kind().total_tokens() {
                self.total_tokens += t;
            }
        }

        fn to_metrics(&self, requested: &[AnalyticsMetric]) -> MetricValues {
            let mut mv = MetricValues::default();
            for m in requested {
                match m {
                    AnalyticsMetric::TotalCost => mv.total_cost = Some(self.cost),
                    AnalyticsMetric::TotalInputTokens => {
                        mv.total_input_tokens = Some(self.input_tokens)
                    }
                    AnalyticsMetric::TotalOutputTokens => {
                        mv.total_output_tokens = Some(self.output_tokens)
                    }
                    AnalyticsMetric::TotalTokens => mv.total_tokens = Some(self.total_tokens),
                    AnalyticsMetric::AvgLatencyMs => {
                        mv.avg_latency_ms = if self.latency_count > 0 {
                            Some(self.latency_sum_ms / self.latency_count as f64)
                        } else {
                            Some(0.0)
                        };
                    }
                    AnalyticsMetric::SpanCount => mv.span_count = Some(self.span_count),
                    AnalyticsMetric::ErrorCount => mv.error_count = Some(self.error_count),
                }
            }
            mv
        }
    }

    fn group_key(span: &Span, fields: &[GroupByField]) -> HashMap<String, String> {
        let mut key = HashMap::new();
        for field in fields {
            let val = match field {
                GroupByField::Model => span
                    .kind()
                    .model()
                    .unwrap_or("unknown")
                    .to_string(),
                GroupByField::Provider => span
                    .kind()
                    .provider()
                    .unwrap_or("unknown")
                    .to_string(),
                GroupByField::Kind => span.kind().kind_name().to_string(),
                GroupByField::Status => span.status().as_str().to_string(),
                GroupByField::Trace => span.trace_id().to_string(),
                GroupByField::Day => span.started_at().format("%Y-%m-%d").to_string(),
                GroupByField::Hour => span.started_at().format("%Y-%m-%dT%H:00").to_string(),
            };
            key.insert(format!("{:?}", field).to_lowercase(), val);
        }
        key
    }

    // Single pass: accumulate into groups + totals
    let mut groups: HashMap<Vec<(String, String)>, Acc> = HashMap::new();
    let mut totals = Acc::new();

    for span in spans {
        totals.accumulate(span);

        if !query.group_by.is_empty() {
            let key_map = group_key(span, &query.group_by);
            let mut sorted_key: Vec<(String, String)> = key_map.into_iter().collect();
            sorted_key.sort_by(|a, b| a.0.cmp(&b.0));
            groups
                .entry(sorted_key)
                .or_insert_with(Acc::new)
                .accumulate(span);
        }
    }

    let result_groups: Vec<AnalyticsGroup> = groups
        .into_iter()
        .map(|(sorted_key, acc)| AnalyticsGroup {
            key: sorted_key.into_iter().collect(),
            metrics: acc.to_metrics(&query.metrics),
        })
        .collect();

    AnalyticsResponse {
        groups: result_groups,
        totals: totals.to_metrics(&query.metrics),
    }
}

/// Compute a summary suitable for a quick dashboard view.
pub fn compute_summary(spans: &[&Span], trace_count: usize) -> AnalyticsSummary {
    let mut total_cost = 0.0_f64;
    let mut total_tokens = 0_u64;
    let mut total_llm_calls = 0_usize;
    let mut error_count = 0_usize;
    let mut latency_sum = 0.0_f64;
    let mut latency_count = 0_usize;

    let mut models: HashMap<String, (f64, u64, u64, usize)> = HashMap::new(); // model -> (cost, in_tok, out_tok, count)
    let mut providers: std::collections::HashSet<String> = std::collections::HashSet::new();

    for span in spans {
        if matches!(span.status(), SpanStatus::Failed { .. }) {
            error_count += 1;
        }
        if let Some(ms) = span.duration_ms() {
            latency_sum += ms as f64;
            latency_count += 1;
        }

        if span.kind().kind_name() == "llm_call" {
            total_llm_calls += 1;
            let model_name = span.kind().model().unwrap_or("unknown").to_string();
            if let Some(p) = span.kind().provider() {
                providers.insert(p.to_string());
            }
            let cost = span.kind().cost().unwrap_or(0.0);
            let in_tok = span.kind().input_tokens().unwrap_or(0);
            let out_tok = span.kind().output_tokens().unwrap_or(0);
            total_cost += cost;
            total_tokens += in_tok + out_tok;

            let entry = models.entry(model_name).or_insert((0.0, 0, 0, 0));
            entry.0 += cost;
            entry.1 += in_tok;
            entry.2 += out_tok;
            entry.3 += 1;
        }
    }

    let models_used: Vec<String> = models.keys().cloned().collect();
    let providers_used: Vec<String> = providers.into_iter().collect();

    let cost_by_model: Vec<ModelCost> = models
        .iter()
        .map(|(model, (cost, _, _, count))| ModelCost {
            model: model.clone(),
            cost: *cost,
            span_count: *count,
        })
        .collect();

    let tokens_by_model: Vec<ModelTokens> = models
        .iter()
        .map(|(model, (_, in_tok, out_tok, _))| ModelTokens {
            model: model.clone(),
            input_tokens: *in_tok,
            output_tokens: *out_tok,
            total_tokens: *in_tok + *out_tok,
        })
        .collect();

    let avg_latency_ms = if latency_count > 0 {
        latency_sum / latency_count as f64
    } else {
        0.0
    };

    AnalyticsSummary {
        total_traces: trace_count,
        total_spans: spans.len(),
        total_llm_calls,
        total_cost,
        total_tokens,
        avg_latency_ms,
        error_count,
        models_used,
        providers_used,
        cost_by_model,
        tokens_by_model,
    }
}
