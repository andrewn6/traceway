// In production, set VITE_API_URL to your API server (e.g., https://api.traceway.example.com)
// Falls back to relative /api for embedded mode or local dev with proxy
const API_BASE = (import.meta.env.VITE_API_URL as string) || '/api';

// ─── Generated Types ─────────────────────────────────────────────────
// Re-export types from the auto-generated OpenAPI types
import type { components } from './api-types';

// Core types
export type SpanKind = components['schemas']['SpanKind'];
export type SpanStatus = components['schemas']['SpanStatus'];
export type Span = components['schemas']['Span'];
export type Trace = components['schemas']['Trace'];
export type TraceListResponse = components['schemas']['TraceListResponse'];
export type SpanList = components['schemas']['SpanList'];
export type Stats = components['schemas']['Stats'];
export type ExportData = components['schemas']['ExportData'];

// File types
export type FileVersion = components['schemas']['FileVersion'];
export type FileListResponse = components['schemas']['FileListResponse'];
export type FileVersionsResponse = components['schemas']['FileVersionsResponse'];

// Health types
export type HealthResponse = components['schemas']['HealthResponse'];
export type StorageHealth = components['schemas']['StorageHealth'];

// Dataset types
export type DatapointKind = components['schemas']['DatapointKind'];
export type DatapointSource = components['schemas']['DatapointSource'];
export type Dataset = components['schemas']['Dataset'];
export type DatasetResponse = components['schemas']['DatasetResponse'];
export type DatasetListResponse = components['schemas']['DatasetListResponse'];
export type Datapoint = components['schemas']['Datapoint'];
export type DatapointListResponse = components['schemas']['DatapointListResponse'];
export type QueueItem = components['schemas']['QueueItem'];
export type QueueItemStatus = components['schemas']['QueueItemStatus'];
export type QueueListResponse = components['schemas']['QueueListResponse'];
export type QueueCounts = components['schemas']['QueueCounts'];
export type Message = components['schemas']['Message'];

// Analytics types
export type AnalyticsSummary = components['schemas']['AnalyticsSummary'];
export type AnalyticsQuery = components['schemas']['AnalyticsQuery'];
export type AnalyticsResponse = components['schemas']['AnalyticsResponse'];
export type AnalyticsFilter = components['schemas']['AnalyticsFilter'];
export type AnalyticsMetric = components['schemas']['AnalyticsMetric'];
export type GroupByField = components['schemas']['GroupByField'];
export type MetricValues = components['schemas']['MetricValues'];
export type ModelCost = components['schemas']['ModelCost'];
export type ModelTokens = components['schemas']['ModelTokens'];

// Request types
export type CreateSpanRequest = components['schemas']['CreateSpanRequest'];
export type CompleteSpanRequest = components['schemas']['CompleteSpanRequest'];
export type FailSpanRequest = components['schemas']['FailSpanRequest'];
export type SpanQueryParams = components['schemas']['SpanQueryParams'];
export type CreateTraceRequest = components['schemas']['CreateTraceRequest'];
export type CreateDatasetRequest = components['schemas']['CreateDatasetRequest'];
export type UpdateDatasetRequest = components['schemas']['UpdateDatasetRequest'];
export type CreateDatapointRequest = components['schemas']['CreateDatapointRequest'];

// Legacy type aliases for backward compatibility
export type TraceList = TraceListResponse;
export type DatasetWithCount = DatasetResponse;
export type DatasetList = DatasetListResponse;
export type DatapointList = DatapointListResponse;
export type QueueList = QueueListResponse;
export type HealthStatus = HealthResponse;
export type SpanFilter = SpanQueryParams;

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
	const res = await fetch(`${API_BASE}${path}`, { credentials: 'include' });
	if (!res.ok) throw new Error(`GET ${path}: ${res.status}`);
	return res.json();
}

async function post<T>(path: string, body?: unknown): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		method: 'POST',
		credentials: 'include',
		headers: body ? { 'Content-Type': 'application/json' } : {},
		body: body ? JSON.stringify(body) : undefined
	});
	if (!res.ok) throw new Error(`POST ${path}: ${res.status}`);
	return res.json();
}

async function put<T>(path: string, body?: unknown): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		method: 'PUT',
		credentials: 'include',
		headers: body ? { 'Content-Type': 'application/json' } : {},
		body: body ? JSON.stringify(body) : undefined
	});
	if (!res.ok) throw new Error(`PUT ${path}: ${res.status}`);
	return res.json();
}

async function del<T>(path: string): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, { method: 'DELETE', credentials: 'include' });
	if (!res.ok) throw new Error(`DELETE ${path}: ${res.status}`);
	return res.json();
}

async function postMultipart<T>(path: string, form: FormData): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		method: 'POST',
		credentials: 'include',
		body: form
	});
	if (!res.ok) throw new Error(`POST ${path}: ${res.status}`);
	return res.json();
}

/**
 * POST with raw response access (for checking status codes, reading headers).
 * Used for auth endpoints where we need to handle errors without throwing.
 */
async function postRaw(path: string, body?: unknown): Promise<Response> {
	return fetch(`${API_BASE}${path}`, {
		method: 'POST',
		credentials: 'include',
		headers: body ? { 'Content-Type': 'application/json' } : {},
		body: body ? JSON.stringify(body) : undefined
	});
}

// ─── Trace / Span Endpoints ─────────────────────────────────────────

export const getTraces = () => get<TraceList>('/traces');
export const createTrace = (name?: string, tags?: string[]) =>
	post<Trace>('/traces', { name, tags: tags ?? [] });
export const getTrace = (id: string) => get<SpanList>(`/traces/${id}`);
export const getSpans = (filter?: SpanFilter) => get<SpanList>(`/spans${qs((filter ?? {}) as Record<string, string | undefined>)}`);
export const getSpan = (id: string) => get<Span>(`/spans/${id}`);
export const getStats = () => get<Stats>('/stats');

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
export const queryAnalytics = (query: AnalyticsQuery) => post<AnalyticsResponse>('/analytics', query);

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

// ─── Auth Types ──────────────────────────────────────────────────────

export type Scope =
	| 'traces_read'
	| 'traces_write'
	| 'datasets_read'
	| 'datasets_write'
	| 'analytics_read'
	| 'admin';

export interface AuthConfig {
	mode: 'local' | 'cloud';
	features: string[];
}

export interface AuthMe {
	org_id: string;
	user_id: string | null;
	scopes: Scope[];
	is_local_mode: boolean;
}

export interface ApiKeyInfo {
	id: string;
	name: string;
	key_prefix: string;
	scopes: Scope[];
	created_at: string;
	last_used_at: string | null;
}

export interface ApiKeyCreated {
	id: string;
	key: string;
	name: string;
	key_prefix: string;
	scopes: Scope[];
}

export interface OrgInfo {
	id: string;
	name: string;
	slug: string;
	plan: string;
}

export interface OrgMember {
	id: string;
	email: string;
	name: string | null;
	role: string;
}

// ─── Auth Response Types ─────────────────────────────────────────────

export interface AuthResponse {
	user_id: string;
	org_id: string;
	email: string;
	name: string | null;
	role: string;
}

export interface AuthResult {
	ok: boolean;
	data?: AuthResponse;
	error?: string;
}

// ─── Auth Endpoints ──────────────────────────────────────────────────

export const getAuthConfig = () => get<AuthConfig>('/auth/config');
export const getAuthMe = () => get<AuthMe>('/auth/me');
export const getOrg = () => get<OrgInfo>('/org');
export const getApiKeys = () => get<ApiKeyInfo[]>('/org/api-keys');
export const createApiKey = (name: string, scopes?: Scope[]) =>
	post<ApiKeyCreated>('/org/api-keys', { name, scopes: scopes ?? [] });
export const deleteApiKey = (id: string) => del<unknown>(`/org/api-keys/${id}`);
export const getOrgMembers = () => get<OrgMember[]>('/org/members');

export async function signup(
	email: string,
	password: string,
	name?: string,
	orgName?: string
): Promise<AuthResult> {
	const res = await postRaw('/auth/signup', {
		email,
		password,
		name: name || undefined,
		org_name: orgName || undefined
	});
	if (res.ok) {
		const data: AuthResponse = await res.json();
		return { ok: true, data };
	}
	const text = await res.text();
	return { ok: false, error: text || `Signup failed (${res.status})` };
}

export async function login(email: string, password: string): Promise<AuthResult> {
	const res = await postRaw('/auth/login', { email, password });
	if (res.ok) {
		const data: AuthResponse = await res.json();
		return { ok: true, data };
	}
	const text = await res.text();
	return { ok: false, error: text || `Login failed (${res.status})` };
}

export async function logout(): Promise<void> {
	await postRaw('/auth/logout');
}

// ─── Invite Types ────────────────────────────────────────────────────

export interface InviteInfo {
	id: string;
	email: string;
	role: string;
	invited_by: string;
	expires_at: string;
	created_at: string;
}

// ─── Invite Endpoints ────────────────────────────────────────────────

export const getInvites = () => get<InviteInfo[]>('/org/invites');
export const createInvite = (email: string, role?: string) =>
	post<InviteInfo>('/org/invites', { email, role: role ?? 'member' });
export const deleteInvite = (id: string) => del<unknown>(`/org/invites/${id}`);

export async function acceptInvite(
	token: string,
	password: string,
	name?: string
): Promise<AuthResult> {
	const res = await postRaw('/auth/accept-invite', {
		token,
		password,
		name: name || undefined
	});
	if (res.ok) {
		const data: AuthResponse = await res.json();
		return { ok: true, data };
	}
	const text = await res.text();
	return { ok: false, error: text || `Accept invite failed (${res.status})` };
}

// ─── Password Reset Endpoints ────────────────────────────────────────

export async function forgotPassword(email: string): Promise<{ ok: boolean; message?: string }> {
	const res = await postRaw('/auth/forgot-password', { email });
	if (res.ok) {
		return res.json();
	}
	const text = await res.text();
	return { ok: false, message: text || `Request failed (${res.status})` };
}

export async function resetPassword(token: string, password: string): Promise<{ ok: boolean; message?: string }> {
	const res = await postRaw('/auth/reset-password', { token, password });
	if (res.ok) {
		return res.json();
	}
	const text = await res.text();
	return { ok: false, message: text || `Reset failed (${res.status})` };
}
