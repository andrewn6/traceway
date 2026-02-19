import type {
  Span,
  Trace,
  TraceList,
  SpanList,
  Stats,
  ExportData,
  SpanFilter,
  SpanKind,
  StartSpanOpts,
  CreatedSpan,
  SpanEvent,
  TrackedFile,
  FileVersion,
  FileTraces,
  HealthStatus,
} from './types.js';

export interface LLMTraceOpts {
  url?: string;
}

export class LLMTrace {
  private baseUrl: string;

  constructor(opts?: LLMTraceOpts) {
    this.baseUrl = (opts?.url ?? 'http://localhost:3000').replace(/\/$/, '');
  }

  // ─── Internal helpers ──────────────────────────────────────────────

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetch(`${this.baseUrl}/api${path}`, {
      method,
      headers: body ? { 'Content-Type': 'application/json' } : {},
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!res.ok) {
      throw new Error(`${method} ${path}: HTTP ${res.status}`);
    }
    const text = await res.text();
    return text ? JSON.parse(text) : (undefined as T);
  }

  private async requestText(method: string, path: string): Promise<string> {
    const res = await fetch(`${this.baseUrl}/api${path}`, { method });
    if (!res.ok) {
      throw new Error(`${method} ${path}: HTTP ${res.status}`);
    }
    return res.text();
  }

  private qs(params: Record<string, string | undefined>): string {
    const entries = Object.entries(params).filter(
      (e): e is [string, string] => e[1] !== undefined,
    );
    if (entries.length === 0) return '';
    return '?' + new URLSearchParams(entries).toString();
  }

  // ─── Trace operations ──────────────────────────────────────────────

  async createTrace(name?: string, tags?: string[]): Promise<Trace> {
    const body: Record<string, unknown> = {};
    if (name !== undefined) body.name = name;
    if (tags !== undefined) body.tags = tags;
    return this.request<Trace>('POST', '/traces', body);
  }

  // ─── Span operations ───────────────────────────────────────────────

  async startSpan(opts: StartSpanOpts): Promise<CreatedSpan> {
    const body: Record<string, unknown> = {
      trace_id: opts.traceId,
      parent_id: opts.parentId,
      name: opts.name,
    };
    if (opts.kind) body.kind = opts.kind;
    if (opts.input !== undefined) body.input = opts.input;
    if (opts.metadata) body.metadata = opts.metadata;
    return this.request<CreatedSpan>('POST', '/spans', body);
  }

  async completeSpan(spanId: string, output?: unknown): Promise<void> {
    const body = output !== undefined ? { output } : undefined;
    await this.request<void>('POST', `/spans/${spanId}/complete`, body);
  }

  async failSpan(spanId: string, error: string): Promise<void> {
    await this.request<void>('POST', `/spans/${spanId}/fail`, { error });
  }

  // ─── Read operations ───────────────────────────────────────────────

  async getTraces(): Promise<TraceList> {
    return this.request<TraceList>('GET', '/traces');
  }

  async getTrace(traceId: string): Promise<SpanList> {
    return this.request<SpanList>('GET', `/traces/${traceId}`);
  }

  async getSpans(filter?: SpanFilter): Promise<SpanList> {
    const qs = filter ? this.qs(filter as Record<string, string | undefined>) : '';
    return this.request<SpanList>('GET', `/spans${qs}`);
  }

  async getStats(): Promise<Stats> {
    return this.request<Stats>('GET', '/stats');
  }

  async getHealth(): Promise<HealthStatus> {
    return this.request<HealthStatus>('GET', '/health');
  }

  // ─── File operations ───────────────────────────────────────────────

  async listFiles(pathPrefix?: string): Promise<TrackedFile[]> {
    const qs = pathPrefix ? this.qs({ path_prefix: pathPrefix }) : '';
    return this.request<TrackedFile[]>('GET', `/files${qs}`);
  }

  async readFile(path: string): Promise<string> {
    return this.requestText('GET', `/files/${encodeURIComponent(path)}`);
  }

  async fileVersions(path: string): Promise<FileVersion[]> {
    return this.request<FileVersion[]>('GET', `/files/${encodeURIComponent(path)}/versions`);
  }

  async fileTraces(path: string): Promise<FileTraces> {
    return this.request<FileTraces>('GET', `/files/${encodeURIComponent(path)}/traces`);
  }

  // ─── Delete operations ─────────────────────────────────────────────

  async deleteTrace(traceId: string): Promise<void> {
    await this.request<void>('DELETE', `/traces/${traceId}`);
  }

  async clearAll(): Promise<void> {
    await this.request<void>('DELETE', '/traces');
  }

  // ─── Export ────────────────────────────────────────────────────────

  async exportJson(traceId?: string): Promise<ExportData> {
    const qs = traceId ? `?trace_id=${traceId}` : '';
    return this.request<ExportData>('GET', `/export/json${qs}`);
  }

  // ─── Live ──────────────────────────────────────────────────────────

  subscribe(callback: (event: SpanEvent) => void): () => void {
    const es = new EventSource(`${this.baseUrl}/api/events`);
    es.onmessage = (e) => {
      try {
        callback(JSON.parse(e.data));
      } catch {
        // ignore parse errors
      }
    };
    return () => es.close();
  }

  // ─── Trace context ─────────────────────────────────────────────────

  async trace<T>(
    name: string,
    fn: (ctx: TraceContext) => Promise<T>,
  ): Promise<T> {
    const trace = await this.createTrace(name || undefined);
    const ctx = new TraceContext(this, trace.id, name);
    return fn(ctx);
  }
}

export class TraceContext {
  constructor(
    private client: LLMTrace,
    public readonly traceId: string,
    public readonly name: string,
  ) {}

  async span<T>(
    name: string,
    fn: (ctx: SpanContext) => Promise<T>,
    opts?: { kind?: SpanKind; parentId?: string; input?: unknown },
  ): Promise<T> {
    const created = await this.client.startSpan({
      traceId: this.traceId,
      parentId: opts?.parentId,
      name,
      kind: opts?.kind,
      input: opts?.input,
    });
    const ctx = new SpanContext(this.client, created.id, this.traceId);
    try {
      const result = await fn(ctx);
      await this.client.completeSpan(created.id, ctx.output);
      return result;
    } catch (e) {
      await this.client.failSpan(created.id, String(e));
      throw e;
    }
  }

  async llmCall<T>(
    name: string,
    opts: { model: string; provider?: string; input?: unknown },
    fn: (ctx: SpanContext) => Promise<T>,
  ): Promise<T> {
    return this.span(name, fn, {
      kind: {
        type: 'llm_call',
        model: opts.model,
        provider: opts.provider,
      } as SpanKind,
      input: opts.input,
    });
  }
}

export class SpanContext {
  output: unknown = undefined;

  constructor(
    private client: LLMTrace,
    public readonly spanId: string,
    public readonly traceId: string,
  ) {}

  setOutput(value: unknown): void {
    this.output = value;
  }
}
