import { and, eq } from "drizzle-orm";
import { db } from "../core/database";
import { spans, traces } from "../core/schema";
import { estimateCost } from "../tracing/pricing";

type Scope = { org_id: string; project_id: string };

export interface LlmCallReplay {
  span_id: string;
  name: string;
  model: string;
  provider: string | null;
  input: unknown;
  output: unknown;
  input_tokens: number | null;
  output_tokens: number | null;
  cost: number;
  duration_ms: number | null;
  started_at: string;
  ended_at: string | null;
}

export interface ReplayableTrace {
  trace_id: string;
  trace_name: string | null;
  llm_calls: LlmCallReplay[];
  total_cost: number;
  total_tokens: number;
}

export interface ReplayResult {
  span_id: string;
  success: boolean;
  output: unknown;
  error?: string;
  input_tokens?: number;
  output_tokens?: number;
  cost?: number;
  duration_ms?: number;
}

export interface ExecuteReplayRequest {
  trace_id: string;
  overrides: Record<string, { input?: unknown }>;
}

export async function getReplayableTrace(scope: Scope, traceId: string): Promise<ReplayableTrace | null> {
  const [traceRow] = await db
    .select()
    .from(traces)
    .where(and(
      eq(traces.id, traceId),
      eq(traces.orgId, scope.org_id),
      eq(traces.projectId, scope.project_id)
    ))
    .limit(1);

  if (!traceRow) return null;

  const spanRows = await db
    .select()
    .from(spans)
    .where(and(
      eq(spans.traceId, traceId),
      eq(spans.orgId, scope.org_id),
      eq(spans.projectId, scope.project_id)
    ))
    .orderBy(spans.startedAt);

  const llmCalls: LlmCallReplay[] = [];
  let totalCost = 0;
  let totalTokens = 0;

  for (const span of spanRows) {
    const kind = span.kind as Record<string, unknown>;
    if (kind.type !== "llm_call") continue;

    const model = typeof kind.model === "string" ? kind.model : "";
    const provider = typeof kind.provider === "string" ? kind.provider : null;
    const inputTokens = typeof kind.input_tokens === "number" ? kind.input_tokens : null;
    const outputTokens = typeof kind.output_tokens === "number" ? kind.output_tokens : null;
    const existingCost = typeof kind.cost === "number" ? kind.cost : null;

    const tokens = (inputTokens ?? 0) + (outputTokens ?? 0);
    const cost = existingCost ?? estimateCost(model, inputTokens ?? 0, outputTokens ?? 0) ?? 0;

    totalCost += cost;
    totalTokens += tokens;

    const startMs = new Date(span.startedAt).getTime();
    const endMs = span.endedAt ? new Date(span.endedAt).getTime() : Date.now();
    const durationMs = span.endedAt ? endMs - startMs : null;

    llmCalls.push({
      span_id: span.id,
      name: span.name,
      model,
      provider,
      input: span.input ?? null,
      output: span.output ?? null,
      input_tokens: inputTokens,
      output_tokens: outputTokens,
      cost,
      duration_ms: durationMs,
      started_at: span.startedAt.toISOString(),
      ended_at: span.endedAt?.toISOString() ?? null,
    });
  }

  return {
    trace_id: traceId,
    trace_name: traceRow.name ?? null,
    llm_calls: llmCalls,
    total_cost: totalCost,
    total_tokens: totalTokens,
  };
}
