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

// ─── Dataset Types ───────────────────────────────────────────────────

export type DatapointKind =
	| { type: 'llm_conversation'; messages: { role: string; content: string }[]; expected?: { role: string; content: string }; metadata: Record<string, unknown> }
	| { type: 'generic'; input: unknown; expected_output?: unknown; actual_output?: unknown; score?: number; metadata: Record<string, unknown> };

export type DatapointSource = 'manual' | 'span_export' | 'file_upload';

export interface Dataset {
	id: string;
	name: string;
	description?: string;
	created_at: string;
	updated_at: string;
}

export interface DatasetWithCount extends Dataset {
	datapoint_count: number;
}

export interface Datapoint {
	id: string;
	dataset_id: string;
	kind: DatapointKind;
	source: DatapointSource;
	source_span_id?: string;
	created_at: string;
}

export interface QueueItem {
	id: string;
	dataset_id: string;
	datapoint_id: string;
	status: 'pending' | 'claimed' | 'completed';
	claimed_by?: string;
	claimed_at?: string;
	original_data?: unknown;
	edited_data?: unknown;
	created_at: string;
}

export interface DatasetList {
	datasets: DatasetWithCount[];
	count: number;
}

export interface DatapointList {
	datapoints: Datapoint[];
	count: number;
}

export interface QueueList {
	items: QueueItem[];
	counts: { pending: number; claimed: number; completed: number };
}

// ─── Events ──────────────────────────────────────────────────────────

export type SpanEvent =
	| { type: 'span_created'; span: Span }
	| { type: 'span_updated'; span: Span }
	| { type: 'span_deleted'; span_id: string }
	| { type: 'trace_deleted'; trace_id: string }
	| { type: 'file_version_created'; file: TrackedFile; version: FileVersion }
	| { type: 'dataset_created'; dataset: DatasetWithCount }
	| { type: 'dataset_deleted'; dataset_id: string }
	| { type: 'datapoint_created'; datapoint: Datapoint }
	| { type: 'queue_item_updated'; item: QueueItem }
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

async function put<T>(path: string, body?: unknown): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		method: 'PUT',
		headers: body ? { 'Content-Type': 'application/json' } : {},
		body: body ? JSON.stringify(body) : undefined
	});
	if (!res.ok) throw new Error(`PUT ${path}: ${res.status}`);
	return res.json();
}

async function del<T>(path: string): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, { method: 'DELETE' });
	if (!res.ok) throw new Error(`DELETE ${path}: ${res.status}`);
	return res.json();
}

async function postMultipart<T>(path: string, form: FormData): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		method: 'POST',
		body: form
	});
	if (!res.ok) throw new Error(`POST ${path}: ${res.status}`);
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

// ─── Dataset Endpoints ───────────────────────────────────────────────

export const getDatasets = () => get<DatasetList>('/datasets');
export const createDataset = (name: string, description?: string) =>
	post<DatasetWithCount>('/datasets', { name, description });
export const getDataset = (id: string) => get<DatasetWithCount>(`/datasets/${id}`);
export const updateDataset = (id: string, body: { name?: string; description?: string }) =>
	put<DatasetWithCount>(`/datasets/${id}`, body);
export const deleteDataset = (id: string) => del<unknown>(`/datasets/${id}`);

export const getDatapoints = (datasetId: string) =>
	get<DatapointList>(`/datasets/${datasetId}/datapoints`);
export const createDatapoint = (datasetId: string, kind: DatapointKind) =>
	post<Datapoint>(`/datasets/${datasetId}/datapoints`, { kind });
export const deleteDatapoint = (datasetId: string, dpId: string) =>
	del<unknown>(`/datasets/${datasetId}/datapoints/${dpId}`);

export const exportSpanToDataset = (datasetId: string, spanId: string) =>
	post<Datapoint>(`/datasets/${datasetId}/export-span`, { span_id: spanId });

export function importFile(datasetId: string, file: File): Promise<{ imported: number }> {
	const form = new FormData();
	form.append('file', file);
	return postMultipart(`/datasets/${datasetId}/import`, form);
}

export const getQueue = (datasetId: string) =>
	get<QueueList>(`/datasets/${datasetId}/queue`);
export const enqueueDatapoints = (datasetId: string, datapointIds: string[]) =>
	post<unknown>(`/datasets/${datasetId}/queue`, { datapoint_ids: datapointIds });
export const claimQueueItem = (itemId: string, claimedBy: string) =>
	post<QueueItem>(`/queue/${itemId}/claim`, { claimed_by: claimedBy });
export const submitQueueItem = (itemId: string, editedData?: unknown) =>
	post<QueueItem>(`/queue/${itemId}/submit`, { edited_data: editedData });

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
