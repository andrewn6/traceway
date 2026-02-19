const API_BASE = '/api';

// ─── Core Types ──────────────────────────────────────────────────────

export type SpanKind =
	| { type: 'fs_read'; path: string; file_version?: string; bytes_read: number }
	| { type: 'fs_write'; path: string; file_version: string; bytes_written: number }
	| { type: 'llm_call'; model: string; provider?: string; input_tokens?: number; output_tokens?: number; cost?: number; input_preview?: string; output_preview?: string }
	| { type: 'custom'; kind: string; attributes: Record<string, unknown> };

// Status as serialized by Rust serde: "running", "completed", or { "failed": { "error": "..." } }
export type SpanStatus = 'running' | 'completed' | { failed: { error: string } };

export interface Span {
	id: string;
	trace_id: string;
	parent_id: string | null;
	name: string;
	kind: SpanKind;
	status: SpanStatus;
	started_at: string;
	ended_at: string | null;
	input?: unknown;
	output?: unknown;
}

export interface Trace {
	id: string;
	name?: string;
	tags: string[];
	started_at: string;
	ended_at?: string | null;
	machine_id?: string;
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

export interface SpanFilter {
	model?: string;
	provider?: string;
	status?: string;
	since?: string;
	until?: string;
	name_contains?: string;
	kind?: string;
	path?: string;
	trace_id?: string;
}

// ─── File Types ──────────────────────────────────────────────────────

export interface FileVersion {
	hash: string;
	path: string;
	size: number;
	created_at: string;
	created_by_span: string | null;
}

export interface FileListResponse {
	files: FileVersion[];
	count: number;
}

export interface FileVersionsResponse {
	path: string;
	versions: FileVersion[];
	count: number;
}

// ─── Health ──────────────────────────────────────────────────────────

export interface HealthStatus {
	status: string;
	uptime_secs: number;
	version: string;
	storage: {
		span_count: number;
		trace_count: number;
	};
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

// ─── Analytics Types ─────────────────────────────────────────────────

export interface AnalyticsSummary {
	total_traces: number;
	total_spans: number;
	total_llm_calls: number;
	total_cost: number;
	total_tokens: number;
	avg_latency_ms: number;
	error_count: number;
	models_used: string[];
	providers_used: string[];
	cost_by_model: { model: string; cost: number; span_count: number }[];
	tokens_by_model: { model: string; input_tokens: number; output_tokens: number; total_tokens: number }[];
}

// ─── Events ──────────────────────────────────────────────────────────

export type SpanEvent =
	| { type: 'span_created'; span: Span }
	| { type: 'span_completed'; span: Span }
	| { type: 'span_failed'; span: Span }
	| { type: 'span_deleted'; span_id: string }
	| { type: 'trace_created'; trace: Trace }
	| { type: 'trace_completed'; trace: Trace }
	| { type: 'trace_deleted'; trace_id: string }
	| { type: 'file_version_created'; file: FileVersion }
	| { type: 'dataset_created'; dataset: Dataset }
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
export const createTrace = (name?: string, tags?: string[]) =>
	post<Trace>('/traces', { name, tags: tags ?? [] });
export const getTrace = (id: string) => get<SpanList>(`/traces/${id}`);
export const getSpans = (filter?: SpanFilter) => get<SpanList>(`/spans${qs((filter ?? {}) as Record<string, string | undefined>)}`);
export const getSpan = (id: string) => get<Span>(`/spans/${id}`);
export const getStats = () => get<Stats>('/stats');

export interface CreateSpanRequest {
	trace_id: string;
	parent_id?: string;
	name: string;
	kind: SpanKind;
	input?: unknown;
}

export const createSpan = (req: CreateSpanRequest) =>
	post<{ id: string; trace_id: string }>('/spans', req);

export const completeSpan = (spanId: string, output?: unknown) =>
	post<unknown>(`/spans/${spanId}/complete`, output !== undefined ? { output } : {});

export const failSpan = (spanId: string, error: string) =>
	post<unknown>(`/spans/${spanId}/fail`, { error });

// ─── File Endpoints ──────────────────────────────────────────────────

export const getFiles = (filter?: { path_prefix?: string }) =>
	get<FileListResponse>(`/files${qs(filter ?? {})}`);

export const getFileVersions = (path: string) =>
	get<FileVersionsResponse>(`/files/${encodeURIComponent(path)}`);

// ─── File Content ────────────────────────────────────────────────────

export async function getFileContent(hash: string): Promise<string> {
	const res = await fetch(`${API_BASE}/files/content/${encodeURIComponent(hash)}`);
	if (!res.ok) throw new Error(`GET /files/content/${hash}: ${res.status}`);
	return res.text();
}

// ─── Health ──────────────────────────────────────────────────────────

export const getHealth = () => get<HealthStatus>('/health');

// ─── Config & Shutdown ───────────────────────────────────────────────

export interface DaemonConfig {
	api: { addr: string };
	proxy: { addr: string; target: string; capture_mode: string };
	storage: { db_path: string | null };
	logging: { level: string };
}

export const getConfig = () => get<DaemonConfig>('/config');
export const updateConfig = (config: DaemonConfig) => put<DaemonConfig>('/config', config);
export const shutdownDaemon = () => post<unknown>('/shutdown', {});

// ─── Analytics ───────────────────────────────────────────────────────

export const getAnalyticsSummary = () => get<AnalyticsSummary>('/analytics/summary');

// ─── Dataset Endpoints ───────────────────────────────────────────────

export const getDatasets = () => get<DatasetList>('/datasets');
export const createDataset = (name: string, description?: string) =>
	post<Dataset>('/datasets', { name, description });
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
	if (span.status === 'running') return 'running';
	if (span.status === 'completed') return 'completed';
	return 'failed';
}

export function spanStartedAt(span: Span): string {
	return span.started_at;
}

export function spanEndedAt(span: Span): string | null {
	return span.ended_at ?? null;
}

export function spanDurationMs(span: Span): number | null {
	if (!span.ended_at) return null;
	return new Date(span.ended_at).getTime() - new Date(span.started_at).getTime();
}

export function spanError(span: Span): string | null {
	if (typeof span.status === 'object' && 'failed' in span.status) {
		return span.status.failed.error;
	}
	return null;
}

export function spanKindLabel(span: Span): string {
	if (span.kind.type === 'custom') return span.kind.kind;
	return span.kind.type;
}

export function spanKindColor(span: Span): string {
	switch (span.kind.type) {
		case 'fs_read': return 'bg-accent';
		case 'fs_write': return 'bg-success';
		case 'llm_call': return 'bg-purple-400';
		default: return 'bg-text-muted';
	}
}

export function spanModel(span: Span): string | null {
	if (span.kind.type === 'llm_call') return span.kind.model;
	return null;
}

export function shortId(id: string): string {
	return id.slice(0, 8);
}
