/**
 * TracewayExporter — OpenTelemetry SpanExporter that sends AI SDK
 * telemetry spans to the Traceway API as traces + spans.
 *
 * The AI SDK emits a tree of OTel spans per call:
 *
 *   ai.generateText                    → Traceway trace
 *     ai.generateText.doGenerate       → Traceway span (kind: llm_call)
 *       ai.toolCall                    → Traceway span (kind: custom/tool_call)
 *
 * This exporter batches incoming OTel spans, groups them by root span,
 * and maps them into Traceway API calls.
 */

import type {
  SpanExporter,
  ReadableSpan,
} from '@opentelemetry/sdk-trace-base';
import type { ExportResult } from '@opentelemetry/core';

import {
  isRootAiSpan,
  isLlmCallSpan,
  isToolCallSpan,
  getSpanName,
  toTracewayKind,
  getSpanInput,
  getSpanOutput,
  getMetadata,
  getOperationType,
} from './mapper.js';

// ── Types ───────────────────────────────────────────────────────────

export interface TracewayExporterConfig {
  /** Base URL of the Traceway API. Defaults to TRACEWAY_URL or http://localhost:3000 */
  url?: string;
  /** API key for authentication. Defaults to TRACEWAY_API_KEY env var. */
  apiKey?: string;
  /** Whether to log debug info. Defaults to false. */
  debug?: boolean;
}

interface PendingTrace {
  traceId?: string;       // Traceway trace ID (set after creation)
  rootSpan: ReadableSpan;
  childSpans: ReadableSpan[];
}

// ── Exporter ────────────────────────────────────────────────────────

export class TracewayExporter implements SpanExporter {
  private baseUrl: string;
  private apiKey?: string;
  private debug: boolean;

  constructor(config?: TracewayExporterConfig) {
    const env = typeof process !== 'undefined' ? process.env : ({} as Record<string, string | undefined>);
    this.baseUrl = (config?.url ?? env.TRACEWAY_URL ?? 'http://localhost:3000').replace(/\/$/, '');
    this.apiKey = config?.apiKey ?? env.TRACEWAY_API_KEY;
    this.debug = config?.debug ?? false;
  }

  /**
   * Called by the BatchSpanProcessor with a batch of completed OTel spans.
   */
  export(spans: ReadableSpan[], resultCallback: (result: ExportResult) => void): void {
    this.processSpans(spans)
      .then(() => resultCallback({ code: 0 /* SUCCESS */ }))
      .catch((err) => {
        if (this.debug) console.error('[traceway] export error:', err);
        resultCallback({ code: 1 /* FAILED */ });
      });
  }

  async shutdown(): Promise<void> {
    // Nothing to clean up
  }

  async forceFlush(): Promise<void> {
    // Nothing buffered internally
  }

  // ── Internal ────────────────────────────────────────────────────────

  private async processSpans(otelSpans: ReadableSpan[]): Promise<void> {
    // Group spans by their OTel trace ID (not Traceway trace ID).
    // Each OTel trace ID corresponds to one AI SDK call (generateText/streamText).
    const byOtelTrace = new Map<string, ReadableSpan[]>();

    for (const span of otelSpans) {
      const op = getOperationType(span);
      if (op === 'unknown') continue; // Skip non-AI spans

      const traceId = span.spanContext().traceId;
      let group = byOtelTrace.get(traceId);
      if (!group) {
        group = [];
        byOtelTrace.set(traceId, group);
      }
      group.push(span);
    }

    // Process each group
    const promises: Promise<void>[] = [];
    for (const group of byOtelTrace.values()) {
      promises.push(this.processGroup(group));
    }
    await Promise.all(promises);
  }

  private async processGroup(spans: ReadableSpan[]): Promise<void> {
    // Find the root span (generateText or streamText)
    const rootSpan = spans.find(isRootAiSpan);
    if (!rootSpan) {
      // No root — process each span individually as a standalone trace
      for (const span of spans) {
        await this.exportSingleSpan(span);
      }
      return;
    }

    // Create a Traceway trace from the root span
    const traceName = getSpanName(rootSpan);
    const trace = await this.apiRequest<{ id: string }>('POST', '/traces', { name: traceName });
    const tracewayTraceId = trace.id;

    if (this.debug) {
      console.log(`[traceway] created trace ${tracewayTraceId} (${traceName})`);
    }

    // Build a mapping from OTel spanId → Traceway spanId
    const otelToTraceway = new Map<string, string>();

    // Process spans in order: root → LLM calls → tool calls
    // First: create a span for the root itself (optional — as a container)
    // Actually, the root AI span is the trace itself, so we skip creating a
    // span for it and just process children.

    // Find LLM call spans (doGenerate / doStream) — these are children of root
    const llmSpans = spans.filter(isLlmCallSpan);
    const toolSpans = spans.filter(isToolCallSpan);
    const otherSpans = spans.filter(
      (s) => s !== rootSpan && !isLlmCallSpan(s) && !isToolCallSpan(s) && getOperationType(s) !== 'unknown'
    );

    // Create LLM call spans
    for (const llmSpan of llmSpans) {
      const kind = toTracewayKind(llmSpan);
      const input = getSpanInput(llmSpan);
      const output = getSpanOutput(llmSpan);
      const name = getSpanName(llmSpan);

      const created = await this.apiRequest<{ id: string; trace_id: string }>(
        'POST',
        '/spans',
        {
          trace_id: tracewayTraceId,
          name,
          kind,
          input,
        },
      );
      otelToTraceway.set(llmSpan.spanContext().spanId, created.id);

      // Immediately complete with output
      await this.apiRequest('POST', `/spans/${created.id}/complete`, {
        output,
        kind,
      });

      if (this.debug) {
        console.log(`[traceway]   span ${created.id} (${name})`);
      }
    }

    // Create tool call spans (children of LLM spans)
    for (const toolSpan of toolSpans) {
      const parentOtelId = toolSpan.parentSpanId;
      const parentTracewayId = parentOtelId ? otelToTraceway.get(parentOtelId) : undefined;

      const kind = toTracewayKind(toolSpan);
      const input = getSpanInput(toolSpan);
      const output = getSpanOutput(toolSpan);
      const name = getSpanName(toolSpan);

      const created = await this.apiRequest<{ id: string; trace_id: string }>(
        'POST',
        '/spans',
        {
          trace_id: tracewayTraceId,
          parent_id: parentTracewayId,
          name,
          kind,
          input,
        },
      );
      otelToTraceway.set(toolSpan.spanContext().spanId, created.id);

      await this.apiRequest('POST', `/spans/${created.id}/complete`, { output });

      if (this.debug) {
        console.log(`[traceway]     tool ${created.id} (${name})`);
      }
    }

    // Other spans (embed, etc.)
    for (const span of otherSpans) {
      const parentOtelId = span.parentSpanId;
      const parentTracewayId = parentOtelId ? otelToTraceway.get(parentOtelId) : undefined;

      const kind = toTracewayKind(span);
      const input = getSpanInput(span);
      const output = getSpanOutput(span);
      const name = getSpanName(span);

      const created = await this.apiRequest<{ id: string; trace_id: string }>(
        'POST',
        '/spans',
        {
          trace_id: tracewayTraceId,
          parent_id: parentTracewayId,
          name,
          kind,
          input,
        },
      );

      await this.apiRequest('POST', `/spans/${created.id}/complete`, { output });
    }
  }

  /**
   * Fallback for spans without a root (shouldn't happen normally).
   */
  private async exportSingleSpan(span: ReadableSpan): Promise<void> {
    const traceName = getSpanName(span);
    const trace = await this.apiRequest<{ id: string }>('POST', '/traces', { name: traceName });

    const kind = toTracewayKind(span);
    const input = getSpanInput(span);
    const output = getSpanOutput(span);

    const created = await this.apiRequest<{ id: string }>(
      'POST',
      '/spans',
      {
        trace_id: trace.id,
        name: traceName,
        kind,
        input,
      },
    );

    await this.apiRequest('POST', `/spans/${created.id}/complete`, { output, kind });
  }

  // ── HTTP ──────────────────────────────────────────────────────────

  private async apiRequest<T>(method: string, path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    const res = await fetch(`${this.baseUrl}/api${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!res.ok) {
      const text = await res.text().catch(() => '');
      throw new Error(`Traceway API ${method} ${path}: ${res.status} ${text}`);
    }

    const text = await res.text();
    return text ? JSON.parse(text) : (undefined as T);
  }
}
