import { IncomingMessage, ServerResponse } from "node:http";

import { api } from "encore.dev/api";

import { defaultScopeForLocal } from "../auth/service";
import { handlePreflight, json, readJsonBody, requireScope, setCors } from "../shared/http";
import { getSpan, getTraceSpans, listSpans, listTraces, type SpanItem, type TraceItem } from "../tracing/service";

type Scope = { org_id: string; project_id: string };

type JsonRpcRequest = {
  jsonrpc?: string;
  id?: string | number | null;
  method?: string;
  params?: unknown;
};

type ToolDef = {
  name: string;
  description: string;
  inputSchema: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
    additionalProperties?: boolean;
  };
};

const TOOL_DEFS: ToolDef[] = [
  {
    name: "search_traces",
    description: "Search traces using Traceway query terms (status, model, kind, since, name, tag)",
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string", description: "Traceway query string (example: kind:llm_call status:failed since:1h)" },
        limit: { type: "number", description: "Maximum traces to return (default 20, max 100)" },
      },
      required: ["query"],
      additionalProperties: false,
    },
  },
  {
    name: "list_recent_traces",
    description: "List the most recent traces with summary metadata",
    inputSchema: {
      type: "object",
      properties: {
        limit: { type: "number", description: "Maximum traces to return (default 10, max 100)" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "get_trace",
    description: "Get a full trace rendered as an LLM-friendly span tree summary",
    inputSchema: {
      type: "object",
      properties: {
        trace_id: { type: "string", description: "Trace ID" },
      },
      required: ["trace_id"],
      additionalProperties: false,
    },
  },
  {
    name: "get_span",
    description: "Get a span with full input/output details",
    inputSchema: {
      type: "object",
      properties: {
        span_id: { type: "string", description: "Span ID" },
      },
      required: ["span_id"],
      additionalProperties: false,
    },
  },
];

function isLocalHost(hostHeader: string | undefined): boolean {
  if (!hostHeader) return false;
  const host = hostHeader.split(":")[0]?.trim().toLowerCase() ?? "";
  return host === "localhost" || host === "127.0.0.1" || host === "::1" || host === "[::1]";
}

async function resolveMcpScope(req: IncomingMessage, res: ServerResponse): Promise<Scope | null> {
  const hasAuth = Boolean(req.headers.authorization || req.headers.cookie || req.headers["x-traceway-control-token"]);
  if (hasAuth) {
    const scoped = await requireScope(req, res);
    if (!scoped) return null;
    return { org_id: scoped.org_id, project_id: scoped.project_id };
  }

  const hostHeader = typeof req.headers.host === "string" ? req.headers.host : Array.isArray(req.headers.host) ? req.headers.host[0] : undefined;
  if (!isLocalHost(hostHeader)) {
    json(res, 401, { error: "Unauthorized" });
    return null;
  }

  const fallback = await defaultScopeForLocal();
  if (!fallback) {
    json(res, 401, { error: "No local scope available" });
    return null;
  }
  return fallback;
}

function textContent(text: string) {
  return [{ type: "text", text }];
}

function rpcResult(id: string | number | null | undefined, result: unknown) {
  return { jsonrpc: "2.0", id: id ?? null, result };
}

function rpcError(id: string | number | null | undefined, code: number, message: string) {
  return { jsonrpc: "2.0", id: id ?? null, error: { code, message } };
}

function durationMs(startedAt: string, endedAt?: string | null): number | null {
  if (!endedAt) return null;
  const d = new Date(endedAt).getTime() - new Date(startedAt).getTime();
  return Number.isFinite(d) && d >= 0 ? d : null;
}

function roundCost(n: number): number {
  return Math.round(n * 1_000_000) / 1_000_000;
}

function safeString(v: unknown): string {
  if (typeof v === "string") return v;
  if (v === null || v === undefined) return "";
  try {
    return JSON.stringify(v, null, 2);
  } catch {
    return String(v);
  }
}

function truncateText(v: unknown, limit = 2000): string {
  const text = safeString(v);
  if (text.length <= limit) return text;
  return `${text.slice(0, limit)}\n... [truncated, use get_span for full content]`;
}

function parseSinceMs(value: string): number | null {
  const raw = value.trim().toLowerCase();
  const m = raw.match(/^(\d+)([smhd])$/);
  if (!m) return null;
  const amount = Number(m[1]);
  const unit = m[2];
  if (!Number.isFinite(amount) || amount <= 0) return null;
  const map: Record<string, number> = { s: 1000, m: 60_000, h: 3_600_000, d: 86_400_000 };
  return amount * map[unit];
}

function parseQueryDsl(query: string): { terms: string[]; filters: Record<string, string> } {
  const terms: string[] = [];
  const filters: Record<string, string> = {};
  for (const token of query.split(/\s+/).map((t) => t.trim()).filter(Boolean)) {
    const idx = token.indexOf(":");
    if (idx <= 0) {
      terms.push(token.toLowerCase());
      continue;
    }
    const key = token.slice(0, idx).toLowerCase();
    const value = token.slice(idx + 1);
    if (value) filters[key] = value;
  }
  return { terms, filters };
}

type TraceSummary = {
  id: string;
  name?: string | null;
  status: "failed" | "running" | "completed";
  duration_ms: number | null;
  total_tokens: number;
  total_cost: number;
  started_at: string;
  span_count: number;
  tags?: string[];
};

function summarizeTrace(trace: TraceItem, spansForTrace: SpanItem[]): TraceSummary {
  let totalTokens = 0;
  let totalCost = 0;
  let hasFailed = false;
  let hasRunning = false;
  for (const s of spansForTrace) {
    if (typeof s.status === "object" && s.status !== null && "failed" in (s.status as Record<string, unknown>)) hasFailed = true;
    if (s.status === "running") hasRunning = true;
    const kind = s.kind as Record<string, unknown>;
    if (kind.type === "llm_call") {
      const inTok = typeof kind.input_tokens === "number" ? kind.input_tokens : 0;
      const outTok = typeof kind.output_tokens === "number" ? kind.output_tokens : 0;
      totalTokens += inTok + outTok;
      const cost = typeof kind.cost === "number" ? kind.cost : 0;
      totalCost += cost;
    }
  }
  return {
    id: trace.id,
    name: trace.name,
    status: hasFailed ? "failed" : hasRunning ? "running" : "completed",
    duration_ms: durationMs(trace.started_at, trace.ended_at),
    total_tokens: totalTokens,
    total_cost: roundCost(totalCost),
    started_at: trace.started_at,
    span_count: spansForTrace.length,
    tags: trace.tags ?? [],
  };
}

function buildTraceText(summary: TraceSummary, spansForTrace: SpanItem[]): string {
  const byParent = new Map<string | null, SpanItem[]>();
  for (const s of spansForTrace) {
    const parent = s.parent_id ?? null;
    const arr = byParent.get(parent) ?? [];
    arr.push(s);
    byParent.set(parent, arr);
  }
  for (const arr of byParent.values()) {
    arr.sort((a, b) => new Date(a.started_at).getTime() - new Date(b.started_at).getTime());
  }

  const lines: string[] = [];
  lines.push(`Trace ${summary.id}`);
  lines.push(`Name: ${summary.name || "(unnamed)"}`);
  lines.push(`Status: ${summary.status}`);
  lines.push(`Duration: ${summary.duration_ms ?? "ongoing"} ms`);
  lines.push(`Tokens: ${summary.total_tokens} | Cost: $${summary.total_cost}`);
  lines.push(`Started: ${summary.started_at}`);
  if (summary.tags && summary.tags.length) lines.push(`Tags: ${summary.tags.join(", ")}`);
  lines.push("");
  lines.push("Span tree:");

  const walk = (parentId: string | null, depth: number) => {
    const nodes = byParent.get(parentId) ?? [];
    for (const s of nodes) {
      const kind = (s.kind as Record<string, unknown>)?.type;
      const status = typeof s.status === "string" ? s.status : "failed";
      const dms = durationMs(s.started_at, s.ended_at);
      const indent = "  ".repeat(depth);
      lines.push(`${indent}- ${s.name} [${kind ?? "unknown"}] ${status} ${dms ?? "ongoing"}ms id=${s.id}`);

      const failedObj = typeof s.status === "object" && s.status && "failed" in (s.status as Record<string, unknown>)
        ? ((s.status as { failed?: { error?: string } }).failed ?? null)
        : null;
      if (failedObj?.error) lines.push(`${indent}  error: ${truncateText(failedObj.error, 500)}`);

      if (s.input !== undefined) lines.push(`${indent}  input: ${truncateText(s.input)}`);
      if (s.output !== undefined) lines.push(`${indent}  output: ${truncateText(s.output)}`);

      walk(s.id, depth + 1);
    }
  };

  walk(null, 0);
  return lines.join("\n");
}

async function toolListRecent(scope: Scope, args: Record<string, unknown>) {
  const traces = await listTraces(scope);
  const spans = await listSpans(scope);
  const byTrace = new Map<string, SpanItem[]>();
  for (const s of spans) {
    const arr = byTrace.get(s.trace_id) ?? [];
    arr.push(s);
    byTrace.set(s.trace_id, arr);
  }
  const limitRaw = typeof args.limit === "number" ? args.limit : 10;
  const limit = Math.max(1, Math.min(100, Math.floor(limitRaw)));
  const summaries = traces.slice(0, limit).map((t) => summarizeTrace(t, byTrace.get(t.id) ?? []));
  return {
    content: textContent(summaries.map((s) => `${s.id} ${s.status} ${s.duration_ms ?? "ongoing"}ms tokens=${s.total_tokens} cost=$${s.total_cost}`).join("\n")),
    structuredContent: { traces: summaries, count: summaries.length },
  };
}

async function toolSearchTraces(scope: Scope, args: Record<string, unknown>) {
  const query = typeof args.query === "string" ? args.query.trim() : "";
  if (!query) throw new Error("query is required");

  const traces = await listTraces(scope);
  const spans = await listSpans(scope);
  const byTrace = new Map<string, SpanItem[]>();
  for (const s of spans) {
    const arr = byTrace.get(s.trace_id) ?? [];
    arr.push(s);
    byTrace.set(s.trace_id, arr);
  }

  const { filters, terms } = parseQueryDsl(query);
  const sinceMs = filters.since ? parseSinceMs(filters.since) : null;
  const now = Date.now();
  const limitRaw = typeof args.limit === "number" ? args.limit : 20;
  const limit = Math.max(1, Math.min(100, Math.floor(limitRaw)));

  const out: TraceSummary[] = [];
  for (const t of traces) {
    if (sinceMs !== null && now - new Date(t.started_at).getTime() > sinceMs) continue;
    const spansForTrace = byTrace.get(t.id) ?? [];
    const summary = summarizeTrace(t, spansForTrace);

    if (filters.status && summary.status !== filters.status) continue;
    if (filters.tag && !(summary.tags ?? []).some((tag) => tag.toLowerCase().includes(filters.tag.toLowerCase()))) continue;
    if (filters.name && !(summary.name ?? "").toLowerCase().includes(filters.name.toLowerCase())) continue;
    if (filters.model) {
      const hasModel = spansForTrace.some((s) => {
        const model = (s.kind as Record<string, unknown>).model;
        return typeof model === "string" && model.toLowerCase().includes(filters.model.toLowerCase());
      });
      if (!hasModel) continue;
    }
    if (filters.kind) {
      const hasKind = spansForTrace.some((s) => ((s.kind as Record<string, unknown>).type ?? "") === filters.kind);
      if (!hasKind) continue;
    }

    if (terms.length) {
      const hay = [
        t.id,
        t.name ?? "",
        ...(t.tags ?? []),
        ...spansForTrace.flatMap((s) => [s.id, s.name, safeString(s.input), safeString(s.output)]),
      ].join(" ").toLowerCase();
      if (!terms.every((term) => hay.includes(term))) continue;
    }

    out.push(summary);
    if (out.length >= limit) break;
  }

  return {
    content: textContent(out.map((s) => `${s.id} ${s.status} ${s.duration_ms ?? "ongoing"}ms tokens=${s.total_tokens} cost=$${s.total_cost}`).join("\n") || "No matching traces"),
    structuredContent: { traces: out, count: out.length, query },
  };
}

async function toolGetTrace(scope: Scope, args: Record<string, unknown>) {
  const traceId = typeof args.trace_id === "string" ? args.trace_id : "";
  if (!traceId) throw new Error("trace_id is required");
  const traces = await listTraces(scope);
  const trace = traces.find((t) => t.id === traceId);
  if (!trace) throw new Error("trace not found");
  const spansForTrace = await getTraceSpans(scope, traceId);
  const summary = summarizeTrace(trace, spansForTrace);
  const text = buildTraceText(summary, spansForTrace);
  return {
    content: textContent(text),
    structuredContent: { trace: summary, spans: spansForTrace },
  };
}

async function toolGetSpan(scope: Scope, args: Record<string, unknown>) {
  const spanId = typeof args.span_id === "string" ? args.span_id : "";
  if (!spanId) throw new Error("span_id is required");
  const span = await getSpan(scope, spanId);
  if (!span) throw new Error("span not found");
  const text = [
    `Span ${span.id}`,
    `Trace: ${span.trace_id}`,
    `Name: ${span.name}`,
    `Kind: ${safeString((span.kind as Record<string, unknown>).type ?? "unknown")}`,
    `Status: ${typeof span.status === "string" ? span.status : "failed"}`,
    `Started: ${span.started_at}`,
    `Ended: ${span.ended_at ?? "(running)"}`,
    "",
    "Input:",
    safeString(span.input),
    "",
    "Output:",
    safeString(span.output),
  ].join("\n");
  return {
    content: textContent(text),
    structuredContent: { span },
  };
}

async function callTool(scope: Scope, name: string, args: Record<string, unknown>) {
  if (name === "list_recent_traces") return toolListRecent(scope, args);
  if (name === "search_traces") return toolSearchTraces(scope, args);
  if (name === "get_trace") return toolGetTrace(scope, args);
  if (name === "get_span") return toolGetSpan(scope, args);
  throw new Error(`unknown tool: ${name}`);
}

export const mcpEndpoint = api.raw(
  { expose: true, method: "POST", path: "/v1/mcp" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    setCors(req, res);

    const body = await readJsonBody<JsonRpcRequest>(req);
    if (body.jsonrpc !== "2.0" || !body.method) {
      json(res, 400, rpcError(body.id, -32600, "Invalid Request"));
      return;
    }

    if (body.method === "tools/list") {
      json(res, 200, rpcResult(body.id, { tools: TOOL_DEFS }));
      return;
    }

    if (body.method !== "tools/call") {
      json(res, 200, rpcError(body.id, -32601, "Method not found"));
      return;
    }

    const scope = await resolveMcpScope(req, res);
    if (!scope) return;

    const params = (body.params ?? {}) as Record<string, unknown>;
    const name = typeof params.name === "string" ? params.name : "";
    const args = (params.arguments ?? {}) as Record<string, unknown>;
    if (!name) {
      json(res, 200, rpcError(body.id, -32602, "name is required"));
      return;
    }

    try {
      const result = await callTool(scope, name, args);
      json(res, 200, rpcResult(body.id, result));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Tool call failed";
      json(res, 200, rpcError(body.id, -32000, message));
    }
  }
);
