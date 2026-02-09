const API_BASE = 'http://localhost:3000';

// ─── Core Types (existing) ───────────────────────────────────────────

export interface SpanMetadata {
	model: string | null;
	input_tokens: number | null;
	output_tokens: number | null;
}

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

// ─── New Types (SpanKind, Files, Health, Analysis) ───────────────────

export type SpanKind =
	| { FsRead: { path: string; file_version?: string; bytes_read: number } }
	| { FsWrite: { path: string; file_version: string; bytes_written: number } }
	| { LlmCall: { model: string; provider?: string; input_tokens?: number; output_tokens?: number; input_preview?: string; output_preview?: string } }
	| { Custom: { kind: string; attributes: Record<string, unknown> } };

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

export interface TraceDiff {
	only_in_a: Span[];
	only_in_b: Span[];
	in_both: { a: Span; b: Span }[];
	files_diff: { path: string; version_a?: string; version_b?: string }[];
}

// ─── Events ──────────────────────────────────────────────────────────

export type SpanEvent =
	| { type: 'span_created'; span: Span }
	| { type: 'span_updated'; span: Span }
	| { type: 'span_deleted'; span_id: string }
	| { type: 'trace_deleted'; trace_id: string }
	| { type: 'file_version_created'; file: TrackedFile; version: FileVersion }
	| { type: 'cleared' };

// ─── HTTP Helpers ────────────────────────────────────────────────────

function qs(params: Record<string, string | undefined>): string {
	const entries = Object.entries(params).filter(
		(e): e is [string, string] => e[1] !== undefined
	);
	if (entries.length === 0) return '';
	return '?' + new URLSearchParams(entries).toString();
}

async function get<T>(path: string): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`);
	if (!res.ok) throw new Error(`GET ${path}: ${res.status}`);
	return res.json();
}

async function getText(path: string): Promise<string> {
	const res = await fetch(`${API_BASE}${path}`);
	if (!res.ok) throw new Error(`GET ${path}: ${res.status}`);
	return res.text();
}

async function post<T>(path: string, body?: unknown): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		method: 'POST',
		headers: body ? { 'Content-Type': 'application/json' } : {},
		body: body ? JSON.stringify(body) : undefined
	});
	if (!res.ok) throw new Error(`POST ${path}: ${res.status}`);
	return res.json();
}

async function del<T>(path: string): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`DELETE ${path}: ${res.status}`);
	return res.json();
}

// ─── Trace / Span Endpoints ─────────────────────────────────────────

export const getTraces = () => get<TraceList>('/traces');
export const getTrace = (id: string) => get<SpanList>(`/traces/${id}`);
export const getSpans = (filter?: SpanFilter) => get<SpanList>(`/spans${qs((filter ?? {}) as Record<string, string | undefined>)}`);
export const getSpan = (id: string) => get<Span>(`/spans/${id}`);
export const getStats = () => get<Stats>('/stats');

// ─── File Endpoints ──────────────────────────────────────────────────

export const getFiles = (filter?: { path_prefix?: string }) =>
	get<TrackedFile[]>(`/files${qs(filter ?? {})}`);

export const getFileContent = (path: string) =>
	getText(`/files/${encodeURIComponent(path)}`);

export const getFileVersions = (path: string) =>
	get<FileVersion[]>(`/files/${encodeURIComponent(path)}/versions`);

export const getFileTraces = (path: string) =>
	get<FileTraces>(`/files/${encodeURIComponent(path)}/traces`);

// ─── Health ──────────────────────────────────────────────────────────

export const getHealth = () => get<HealthStatus>('/health');

// ─── Analysis ────────────────────────────────────────────────────────

export const getTraceDiff = (traceA: string, traceB: string) =>
	get<TraceDiff>(`/analysis/diff${qs({ trace_a: traceA, trace_b: traceB })}`);

export const getFileImpact = (path: string) =>
	get<{ traces: { trace_id: string; read_at: string }[] }>(`/analysis/file-impact${qs({ path })}`);

export const getUnusedContext = (traceId: string) =>
	get<{ unused_files: { path: string; bytes_read: number }[] }>(`/analysis/unused-context${qs({ trace_id: traceId })}`);

// ─── Export ──────────────────────────────────────────────────────────

export const exportJson = (traceId?: string) =>
	get<ExportData>(`/export/json${traceId ? `?trace_id=${traceId}` : ''}`);

// ─── Delete ──────────────────────────────────────────────────────────

export const deleteTrace = (id: string) => del<unknown>(`/traces/${id}`);
export const deleteSpan = (id: string) => del<unknown>(`/spans/${id}`);
export const clearAll = () => del<unknown>('/traces');

// ─── SSE ─────────────────────────────────────────────────────────────

export function subscribeEvents(callback: (event: SpanEvent) => void): () => void {
	const es = new EventSource(`${API_BASE}/events`);
	es.onmessage = (e) => {
		try {
			callback(JSON.parse(e.data));
		} catch {
			// ignore parse errors
		}
	};
	return () => es.close();
}

// ─── Span Helpers ────────────────────────────────────────────────────

export function spanStatus(span: Span): 'running' | 'completed' | 'failed' {
	if ('Running' in span.status) return 'running';
	if ('Completed' in span.status) return 'completed';
	return 'failed';
}

export function spanStartedAt(span: Span): string {
	if (span.started_at) return span.started_at;
	if ('Running' in span.status) return span.status.Running.started_at;
	if ('Completed' in span.status) return span.status.Completed.started_at;
	return span.status.Failed.started_at;
}

export function spanEndedAt(span: Span): string | null {
	if (span.ended_at) return span.ended_at;
	if ('Completed' in span.status) return span.status.Completed.ended_at;
	if ('Failed' in span.status) return span.status.Failed.ended_at;
	return null;
}

export function spanDurationMs(span: Span): number | null {
	const end = spanEndedAt(span);
	if (!end) return null;
	return new Date(end).getTime() - new Date(spanStartedAt(span)).getTime();
}

export function spanError(span: Span): string | null {
	if ('Failed' in span.status) return span.status.Failed.error;
	return null;
}

export function spanKindLabel(span: Span): string | null {
	if (!span.kind) return null;
	if ('FsRead' in span.kind) return 'fs_read';
	if ('FsWrite' in span.kind) return 'fs_write';
	if ('LlmCall' in span.kind) return 'llm_call';
	if ('Custom' in span.kind) return span.kind.Custom.kind;
	return null;
}

export function spanKindColor(span: Span): string {
	if (!span.kind) return 'bg-text-muted';
	if ('FsRead' in span.kind) return 'bg-accent';
	if ('FsWrite' in span.kind) return 'bg-success';
	if ('LlmCall' in span.kind) return 'bg-purple-400';
	return 'bg-text-muted';
}

export function shortId(id: string): string {
	return id.slice(0, 8);
}
