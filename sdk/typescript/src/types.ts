// ─── SpanKind ────────────────────────────────────────────────────────

export type SpanKind =
  | { FsRead: { path: string; file_version?: string; bytes_read: number } }
  | { FsWrite: { path: string; file_version: string; bytes_written: number } }
  | {
      LlmCall: {
        model: string;
        provider?: string;
        input_tokens?: number;
        output_tokens?: number;
        input_preview?: string;
        output_preview?: string;
      };
    }
  | { Custom: { kind: string; attributes: Record<string, unknown> } };

// ─── Legacy SpanMetadata (backward compat) ───────────────────────────

export interface SpanMetadata {
  model: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
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
  started_at?: string;
  ended_at?: string | null;
  status:
    | { Running: { started_at: string } }
    | { Completed: { started_at: string; ended_at: string } }
    | { Failed: { started_at: string; ended_at: string; error: string } };
  metadata: SpanMetadata;
}

// ─── Collections ─────────────────────────────────────────────────────

export interface TraceList {
  traces: string[];
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
  uptime_seconds?: number;
  version?: string;
  components?: Record<string, string>;
  storage?: {
    span_count: number;
    trace_count: number;
    file_count: number;
    db_size_bytes: number;
  };
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
  | { type: 'span_updated'; span: Span }
  | { type: 'span_deleted'; span_id: string }
  | { type: 'trace_deleted'; trace_id: string }
  | { type: 'file_version_created'; file: TrackedFile; version: FileVersion }
  | { type: 'cleared' };
