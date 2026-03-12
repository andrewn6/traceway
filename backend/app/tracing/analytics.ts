import { estimateCost } from "./pricing";
import { listSpans } from "./service";

type Scope = { org_id: string; project_id: string };

function asNumber(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) ? v : null;
}

export async function analyticsSummary(scope: Scope) {
  const spans = await listSpans(scope);
  const llmSpans = spans.filter((s) => s.kind?.type === "llm_call");

  let totalInputTokens = 0;
  let totalOutputTokens = 0;
  let totalCost = 0;
  let totalLatency = 0;
  let latencyCount = 0;
  let errorCount = 0;
  const models = new Set<string>();
  const providers = new Set<string>();
  const byModelTokens = new Map<string, { input: number; output: number; spans: number; cost: number }>();

  for (const span of llmSpans) {
    if (typeof span.status === "object" && span.status && "failed" in span.status) {
      errorCount += 1;
    }
    const kind = span.kind as Record<string, unknown>;
    const model = typeof kind.model === "string" ? kind.model : "unknown";
    const provider = typeof kind.provider === "string" ? kind.provider : "unknown";
    models.add(model);
    providers.add(provider);

    const inputTokens = asNumber(kind.input_tokens) ?? 0;
    const outputTokens = asNumber(kind.output_tokens) ?? 0;
    const cost = asNumber(kind.cost) ?? estimateCost(model, inputTokens, outputTokens) ?? 0;

    totalInputTokens += inputTokens;
    totalOutputTokens += outputTokens;
    totalCost += cost;

    if (span.ended_at) {
      totalLatency += new Date(span.ended_at).getTime() - new Date(span.started_at).getTime();
      latencyCount += 1;
    }

    const current = byModelTokens.get(model) ?? { input: 0, output: 0, spans: 0, cost: 0 };
    current.input += inputTokens;
    current.output += outputTokens;
    current.spans += 1;
    current.cost += cost;
    byModelTokens.set(model, current);
  }

  return {
    total_traces: new Set(spans.map((s) => s.trace_id)).size,
    total_spans: spans.length,
    total_llm_calls: llmSpans.length,
    total_tokens: totalInputTokens + totalOutputTokens,
    total_cost: totalCost,
    avg_latency_ms: latencyCount > 0 ? totalLatency / latencyCount : 0,
    error_count: errorCount,
    models_used: [...models],
    providers_used: [...providers],
    tokens_by_model: [...byModelTokens.entries()].map(([model, v]) => ({
      model,
      input_tokens: v.input,
      output_tokens: v.output,
      total_tokens: v.input + v.output,
    })),
    cost_by_model: [...byModelTokens.entries()].map(([model, v]) => ({
      model,
      span_count: v.spans,
      cost: v.cost,
    })),
  };
}

export async function analyticsQuery(scope: Scope, _query: unknown) {
  const summary = await analyticsSummary(scope);
  return {
    totals: {
      total_cost: summary.total_cost,
      total_input_tokens: summary.tokens_by_model.reduce((acc, row) => acc + row.input_tokens, 0),
      total_output_tokens: summary.tokens_by_model.reduce((acc, row) => acc + row.output_tokens, 0),
      total_tokens: summary.total_tokens,
      avg_latency_ms: summary.avg_latency_ms,
      span_count: summary.total_spans,
      error_count: summary.error_count,
    },
    groups: [],
  };
}
