// ─── SpanKind (internally tagged: {"type": "llm_call", ...}) ─────────

export type SpanKind =
  | { type: 'fs_read'; path: string; file_version?: string; bytes_read: number }
  | { type: 'fs_write'; path: string; file_version: string; bytes_written: number }
  | {
      type: 'llm_call';
      model: string;
      provider?: string;
      input_tokens?: number;
      output_tokens?: number;
      cost?: number;
      input_preview?: string;
      output_preview?: string;
    }
  | { type: 'custom'; kind: string; attributes: Record<string, unknown> };

// ─── Legacy SpanMetadata (backward compat) ───────────────────────────

export interface SpanMetadata {
  model: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
}

// ─── SpanStatus ─────────────────────────────────────────────────────
// Backend serializes as: "running" | "completed" | {"failed":{"error":"..."}}

export type SpanStatus =
  | 'running'
  | 'completed'
  | { failed: { error: string } };

export function statusKind(s: SpanStatus): 'running' | 'completed' | 'failed' {
  if (typeof s === 'string') return s;
  return 'failed';
}

export function statusError(s: SpanStatus): string | null {
  if (typeof s === 'object' && 'failed' in s) return s.failed.error;
  return null;
}

// ─── Span ────────────────────────────────────────────────────────────

export interface Span {
  id: string;
  trace_id: string;
  parent_id: string | null;
  name: string;
  kind?: SpanKind;
  input?: unknown;
  output?: unknown;
  started_at: string;
  ended_at?: string | null;
  status: SpanStatus;
  metadata: SpanMetadata;
}

// ─── Collections ─────────────────────────────────────────────────────

export interface Trace {
  id: string;
  name?: string;
  tags?: string[];
  started_at?: string;
  ended_at?: string | null;
}

export interface TraceList {
  traces: Trace[];
  count: number;
}

export interface SpanList {
  spans: Span[];
  count: number;
}

export interface Stats {
  trace_count: number;
  span_count: number;
}

export interface ExportData {
  traces: Record<string, Span[]>;
}

// ─── Filters ─────────────────────────────────────────────────────────

export interface SpanFilter {
  model?: string;
  status?: string;
  since?: string;
  until?: string;
  name_contains?: string;
  kind?: string;
  path?: string;
  trace_id?: string;
  provider?: string;
}

// ─── File Types ──────────────────────────────────────────────────────

export interface TrackedFile {
  path: string;
  current_hash: string;
  version_count: number;
  created_at: string;
  updated_at: string;
}

export interface FileVersion {
  hash: string;
  path: string;
  size: number;
  created_at: string;
  created_by_span: string | null;
  created_by_trace: string | null;
}

export interface FileTraces {
  reads: { trace_id: string; span_name: string; at: string }[];
  writes: { trace_id: string; span_name: string; at: string }[];
}

// ─── Health ──────────────────────────────────────────────────────────

export interface HealthStatus {
  status: string;
  uptime_secs?: number;
}

// ─── Span creation ───────────────────────────────────────────────────

export interface StartSpanOpts {
  traceId: string;
  parentId?: string;
  name: string;
  kind?: SpanKind;
  input?: unknown;
  metadata?: {
    model?: string;
    input_tokens?: number;
    output_tokens?: number;
  };
}

export interface CreatedSpan {
  id: string;
  trace_id: string;
}

// ─── Events ──────────────────────────────────────────────────────────

export type SpanEvent =
  | { type: 'span_created'; span: Span }
  | { type: 'span_completed'; span: Span }
  | { type: 'span_failed'; span: Span }
  | { type: 'span_deleted'; span_id: string }
  | { type: 'trace_created'; trace: Trace }
  | { type: 'trace_deleted'; trace_id: string }
  | { type: 'file_version_created'; file: FileVersion }
  | { type: 'cleared' };
