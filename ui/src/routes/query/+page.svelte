<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		getSpans,
		getAnalyticsSummary,
		spanStatus,
		spanStartedAt,
		spanEndedAt,
		spanDurationMs,
		spanKindLabel,
		shortId,
		type Span,
		type SpanFilter,
		type AnalyticsSummary
	} from '$lib/api';
	import { parseDsl, filterToDsl, parseRelativeTime } from '$lib/query-dsl';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import SpanKindIcon from '$lib/components/SpanKindIcon.svelte';
	import { onMount } from 'svelte';

	// ─── State ──────────────────────────────────────────────────────────

	let dslInput = $state('');
	let filter: SpanFilter = $state({});
	let results: Span[] = $state([]);
	let resultCount = $state(0);
	let queryTimeMs = $state(0);
	let loading = $state(false);
	let hasQueried = $state(false);

	// Analytics summary for autocomplete
	let summary: AnalyticsSummary | null = $state(null);

	// Search autocomplete
	let searchFocused = $state(false);
	let searchInputEl: HTMLInputElement | undefined = $state(undefined);
	let selectedSuggestionIdx = $state(-1);

	// View mode
	type ViewMode = 'table' | 'grouped' | 'scatter';
	let viewMode: ViewMode = $state('table');

	// History
	interface HistoryEntry {
		dsl: string;
		timestamp: number;
		resultCount: number;
	}

	let history: HistoryEntry[] = $state([]);
	const HISTORY_KEY = 'traceway:query-history';
	const MAX_HISTORY = 50;

	// ─── Sorting ────────────────────────────────────────────────────────

	type SortField = 'name' | 'kind' | 'model' | 'status' | 'duration' | 'tokens' | 'cost' | 'started_at';
	let sortField: SortField = $state('started_at');
	let sortAsc = $state(false);

	function toggleSort(field: SortField) {
		if (sortField === field) {
			sortAsc = !sortAsc;
		} else {
			sortField = field;
			sortAsc = false;
		}
		filter.sort_by = field;
		filter.sort_order = sortAsc ? 'asc' : 'desc';
		syncDslFromFilter();
	}

	function sortIcon(field: SortField): string {
		if (sortField !== field) return '';
		return sortAsc ? '\u2191' : '\u2193';
	}

	// ─── Autocomplete suggestions ──────────────────────────────────────

	interface Suggestion {
		label: string;
		insert: string;
		category: string;
		description?: string;
	}

	const suggestions = $derived.by((): Suggestion[] => {
		const input = dslInput;
		const cursorAtEnd = true; // We always work from end of input for simplicity

		// Find the last token being typed
		const lastSpace = input.lastIndexOf(' ');
		const currentToken = lastSpace >= 0 ? input.slice(lastSpace + 1) : input;

		if (!currentToken) {
			// Show syntax hints when empty or after a space
			if (!input.trim()) return [];
			return [];
		}

		const results: Suggestion[] = [];
		const colonIdx = currentToken.indexOf(':');

		if (colonIdx > 0) {
			// User typed "key:" — suggest values
			const key = currentToken.slice(0, colonIdx).toLowerCase();
			const valuePart = currentToken.slice(colonIdx + 1).toLowerCase();
			const prefix = input.slice(0, lastSpace >= 0 ? lastSpace + 1 : 0);

			if (key === 'model' && summary) {
				for (const m of summary.models_used) {
					if (!valuePart || m.toLowerCase().includes(valuePart)) {
						results.push({ label: m, insert: `${prefix}model:${m}`, category: 'model', description: 'Model' });
					}
				}
			} else if (key === 'provider' && summary) {
				for (const p of summary.providers_used) {
					if (!valuePart || p.toLowerCase().includes(valuePart)) {
						results.push({ label: p, insert: `${prefix}provider:${p}`, category: 'provider', description: 'Provider' });
					}
				}
			} else if (key === 'kind') {
				const kinds = [
					{ id: 'llm_call', label: 'LLM Call' },
					{ id: 'fs_read', label: 'File Read' },
					{ id: 'fs_write', label: 'File Write' },
					{ id: 'custom', label: 'Custom' }
				];
				for (const k of kinds) {
					if (!valuePart || k.id.includes(valuePart) || k.label.toLowerCase().includes(valuePart)) {
						results.push({ label: k.id, insert: `${prefix}kind:${k.id}`, category: 'kind', description: k.label });
					}
				}
			} else if (key === 'status') {
				const statuses = ['running', 'completed', 'failed'];
				for (const s of statuses) {
					if (!valuePart || s.includes(valuePart)) {
						results.push({ label: s, insert: `${prefix}status:${s}`, category: 'status' });
					}
				}
			} else if (key === 'since') {
				const presets = ['5m', '15m', '1h', '6h', '24h', '7d', '30d'];
				for (const p of presets) {
					if (!valuePart || p.includes(valuePart)) {
						results.push({ label: p, insert: `${prefix}since:${p}`, category: 'time' });
					}
				}
			} else if (key === 'duration') {
				const hints = ['>100ms', '>500ms', '>1s', '>5s', '>10s', '<100ms'];
				for (const h of hints) {
					if (!valuePart || h.includes(valuePart)) {
						results.push({ label: h, insert: `${prefix}duration:${h}`, category: 'duration' });
					}
				}
			} else if (key === 'tokens') {
				const hints = ['>100', '>500', '>1000', '>5000', '>10000'];
				for (const h of hints) {
					if (!valuePart || h.includes(valuePart)) {
						results.push({ label: h, insert: `${prefix}tokens:${h}`, category: 'tokens' });
					}
				}
			} else if (key === 'cost') {
				const hints = ['>0.001', '>0.01', '>0.05', '>0.10', '>1.00'];
				for (const h of hints) {
					if (!valuePart || h.includes(valuePart)) {
						results.push({ label: h, insert: `${prefix}cost:${h}`, category: 'cost' });
					}
				}
			} else if (key === 'sort') {
				const fields = ['duration', 'tokens', 'cost', 'started_at', 'name'];
				for (const f of fields) {
					if (!valuePart || f.includes(valuePart)) {
						results.push({ label: f, insert: `${prefix}sort:${f}`, category: 'sort' });
					}
				}
			} else if (key === 'text' || key === 'input' || key === 'output') {
				// No value suggestions for free-text search — user types their own query
				if (!valuePart) {
					results.push({ label: `"search phrase"`, insert: `${prefix}${key}:`, category: 'search', description: 'Type a search phrase (use quotes for multi-word)' });
				}
			}
		} else {
			// Suggest key prefixes
			const q = currentToken.toLowerCase();
			const prefix = input.slice(0, lastSpace >= 0 ? lastSpace + 1 : 0);
			const keys: Suggestion[] = [
				{ label: 'kind:', insert: `${prefix}kind:`, category: 'filter', description: 'Filter by span kind' },
				{ label: 'model:', insert: `${prefix}model:`, category: 'filter', description: 'Filter by LLM model' },
				{ label: 'provider:', insert: `${prefix}provider:`, category: 'filter', description: 'Filter by provider' },
				{ label: 'status:', insert: `${prefix}status:`, category: 'filter', description: 'running, completed, failed' },
				{ label: 'since:', insert: `${prefix}since:`, category: 'filter', description: 'Time range (e.g. 1h, 24h, 7d)' },
				{ label: 'duration:', insert: `${prefix}duration:`, category: 'filter', description: 'Duration filter (e.g. >500ms)' },
				{ label: 'tokens:', insert: `${prefix}tokens:`, category: 'filter', description: 'Token count (e.g. >1000)' },
				{ label: 'cost:', insert: `${prefix}cost:`, category: 'filter', description: 'Cost filter (e.g. >0.01)' },
				{ label: 'text:', insert: `${prefix}text:`, category: 'filter', description: 'Search input & output content' },
				{ label: 'input:', insert: `${prefix}input:`, category: 'filter', description: 'Search input content only' },
				{ label: 'output:', insert: `${prefix}output:`, category: 'filter', description: 'Search output content only' },
				{ label: 'name:', insert: `${prefix}name:`, category: 'filter', description: 'Search span name' },
				{ label: 'trace:', insert: `${prefix}trace:`, category: 'filter', description: 'Filter by trace ID' },
				{ label: 'sort:', insert: `${prefix}sort:`, category: 'filter', description: 'Sort results' },
			];
			for (const k of keys) {
				if (k.label.startsWith(q) || k.description?.toLowerCase().includes(q)) {
					results.push(k);
				}
			}
		}

		return results.slice(0, 10);
	});

	function applySuggestion(s: Suggestion) {
		dslInput = s.insert;
		selectedSuggestionIdx = -1;
		// If the suggestion ends with ":", keep focus for value input
		if (s.insert.endsWith(':')) {
			searchInputEl?.focus();
		} else {
			// Add a space so they can keep typing
			dslInput = s.insert + ' ';
			searchInputEl?.focus();
		}
	}

	// ─── Active filter pills ───────────────────────────────────────────

	interface FilterPill {
		key: string;
		value: string;
		display: string;
		filterKey: keyof SpanFilter;
	}

	const activePills = $derived.by((): FilterPill[] => {
		const pills: FilterPill[] = [];
		const f = filter;

		if (f.kind) pills.push({ key: 'kind', value: f.kind, display: `kind: ${f.kind}`, filterKey: 'kind' });
		if (f.model) pills.push({ key: 'model', value: f.model, display: `model: ${f.model}`, filterKey: 'model' });
		if (f.provider) pills.push({ key: 'provider', value: f.provider, display: `provider: ${f.provider}`, filterKey: 'provider' });
		if (f.status) pills.push({ key: 'status', value: f.status, display: `status: ${f.status}`, filterKey: 'status' });
		if (f.since) pills.push({ key: 'since', value: f.since, display: `since: ${f.since}`, filterKey: 'since' });
		if (f.name_contains) pills.push({ key: 'name', value: f.name_contains, display: `name: ${f.name_contains}`, filterKey: 'name_contains' });
		if (f.trace_id) pills.push({ key: 'trace', value: f.trace_id, display: `trace: ${shortId(f.trace_id)}`, filterKey: 'trace_id' });
		if (f.duration_min) pills.push({ key: 'duration_min', value: f.duration_min, display: `duration > ${f.duration_min}ms`, filterKey: 'duration_min' });
		if (f.duration_max) pills.push({ key: 'duration_max', value: f.duration_max, display: `duration < ${f.duration_max}ms`, filterKey: 'duration_max' });
		if (f.tokens_min) pills.push({ key: 'tokens_min', value: f.tokens_min, display: `tokens > ${f.tokens_min}`, filterKey: 'tokens_min' });
		if (f.cost_min) pills.push({ key: 'cost_min', value: f.cost_min, display: `cost > $${f.cost_min}`, filterKey: 'cost_min' });

		return pills;
	});

	function removePill(pill: FilterPill) {
		delete filter[pill.filterKey];
		syncDslFromFilter();
	}

	function clearAllFilters() {
		filter = {};
		dslInput = '';
		syncDslFromFilter();
	}

	// ─── Quick filter presets ──────────────────────────────────────────

	interface Preset {
		label: string;
		dsl: string;
		icon: string;
	}

	const presets: Preset[] = [
		{ label: 'Slow LLM calls', dsl: 'kind:llm_call duration:>2s since:24h sort:duration', icon: 'slow' },
		{ label: 'Failed spans', dsl: 'status:failed since:24h', icon: 'error' },
		{ label: 'Expensive calls', dsl: 'kind:llm_call cost:>0.01 since:24h sort:cost', icon: 'cost' },
		{ label: 'High token usage', dsl: 'kind:llm_call tokens:>5000 since:24h sort:tokens', icon: 'tokens' },
		{ label: 'Recent LLM calls', dsl: 'kind:llm_call since:1h', icon: 'recent' },
		{ label: 'All spans (24h)', dsl: 'since:24h', icon: 'all' },
	];

	function applyPreset(p: Preset) {
		dslInput = p.dsl;
		applyDsl();
	}

	// ─── Grouped view ──────────────────────────────────────────────────

	interface TraceGroup {
		traceId: string;
		spans: Span[];
		totalDuration: number | null;
		totalTokens: number;
		totalCost: number;
		startedAt: string;
		status: string;
	}

	const groupedByTrace = $derived.by((): TraceGroup[] => {
		const map = new Map<string, Span[]>();
		for (const s of results) {
			let arr = map.get(s.trace_id);
			if (!arr) { arr = []; map.set(s.trace_id, arr); }
			arr.push(s);
		}
		const groups: TraceGroup[] = [];
		for (const [traceId, spans] of map) {
			const durations = spans.map(spanDurationMs).filter((d): d is number => d !== null);
			let totalTokens = 0;
			let totalCost = 0;
			for (const s of spans) {
				if (s.kind?.type === 'llm_call') {
					totalTokens += (s.kind.input_tokens ?? 0) + (s.kind.output_tokens ?? 0);
					totalCost += s.kind.cost ?? 0;
				}
			}
			const startedAt = spans.reduce((earliest, s) => {
				const t = spanStartedAt(s);
				return t < earliest ? t : earliest;
			}, spanStartedAt(spans[0]));
			const status = spans.some(s => spanStatus(s) === 'failed') ? 'failed'
				: spans.some(s => spanStatus(s) === 'running') ? 'running' : 'completed';
			groups.push({
				traceId,
				spans,
				totalDuration: durations.length ? Math.max(...durations) : null,
				totalTokens,
				totalCost,
				startedAt,
				status,
			});
		}
		// Sort by most recent
		groups.sort((a, b) => new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime());
		return groups;
	});

	// ─── Performance insights data ─────────────────────────────────────

	// Color palette for models/kinds
	const MODEL_COLORS = [
		'#6ee7b7', '#a78bfa', '#f9a8d4', '#fbbf24', '#67e8f9',
		'#fb923c', '#86efac', '#c4b5fd', '#fda4af', '#fde68a',
	];

	interface ScatterPoint {
		span: Span;
		x: number;       // timestamp
		y: number;       // duration ms
		logY: number;    // log-scaled Y position (0-1)
		tokens: number;
		status: string;
		colorLabel: string;  // model name or kind for coloring
		colorIdx: number;
	}

	const insightsData = $derived.by(() => {
		const empty = { points: [] as ScatterPoint[], xMin: 0, xMax: 1, yMin: 1, yMax: 1, logYMin: 0, logYMax: 1, p50: 0, p95: 0, p99: 0, colorLabels: [] as string[], totalTokens: 0, totalCost: 0, avgDuration: 0, failedCount: 0 };
		if (results.length === 0) return empty;

		// Build color label index (model for LLM calls, kind for others)
		const labelSet = new Set<string>();
		for (const s of results) {
			if (s.kind?.type === 'llm_call' && s.kind.model) labelSet.add(s.kind.model);
			else labelSet.add(spanKindLabel(s));
		}
		const colorLabels = [...labelSet];
		const labelToIdx = new Map<string, number>();
		colorLabels.forEach((l, i) => labelToIdx.set(l, i));

		// Aggregate stats
		let totalTokens = 0;
		let totalCost = 0;
		let failedCount = 0;
		const durations: number[] = [];

		const raw = results.map(s => {
			const dur = spanDurationMs(s) ?? 0;
			const tokens = s.kind?.type === 'llm_call' ? ((s.kind.input_tokens ?? 0) + (s.kind.output_tokens ?? 0)) : 0;
			const status = spanStatus(s);
			if (s.kind?.type === 'llm_call') {
				totalTokens += tokens;
				totalCost += s.kind.cost ?? 0;
			}
			if (status === 'failed') failedCount++;
			if (dur > 0) durations.push(dur);

			const label = (s.kind?.type === 'llm_call' && s.kind.model) ? s.kind.model : spanKindLabel(s);
			return {
				span: s,
				x: new Date(spanStartedAt(s)).getTime(),
				y: dur,
				logY: 0,
				tokens,
				status,
				colorLabel: label,
				colorIdx: labelToIdx.get(label) ?? 0,
			};
		}).filter(p => p.y > 0);

		if (raw.length === 0) return empty;

		// Percentiles from durations
		durations.sort((a, b) => a - b);
		const percentile = (arr: number[], p: number) => arr[Math.min(Math.floor(arr.length * p), arr.length - 1)];
		const p50 = percentile(durations, 0.5);
		const p95 = percentile(durations, 0.95);
		const p99 = percentile(durations, 0.99);
		const avgDuration = durations.reduce((a, b) => a + b, 0) / durations.length;

		// Log scale Y mapping
		const yMin = Math.max(1, Math.min(...raw.map(p => p.y)));
		const yMax = Math.max(...raw.map(p => p.y));
		const logMin = Math.log10(yMin);
		const logMax = Math.log10(yMax);
		const logRange = logMax - logMin || 1;

		for (const p of raw) {
			p.logY = (Math.log10(Math.max(1, p.y)) - logMin) / logRange;
		}

		const xMin = Math.min(...raw.map(p => p.x));
		const xMax = Math.max(...raw.map(p => p.x));

		return {
			points: raw,
			xMin,
			xMax: xMax === xMin ? xMin + 1 : xMax,
			yMin,
			yMax,
			logYMin: logMin,
			logYMax: logMax,
			p50, p95, p99,
			colorLabels,
			totalTokens,
			totalCost,
			avgDuration: Math.round(avgDuration),
			failedCount,
		};
	});

	function percentileLogY(ms: number): number {
		if (insightsData.logYMax === insightsData.logYMin) return 0.5;
		return (Math.log10(Math.max(1, ms)) - insightsData.logYMin) / (insightsData.logYMax - insightsData.logYMin);
	}

	let hoveredPoint: ScatterPoint | null = $state(null);

	// ─── URL state ──────────────────────────────────────────────────────

	function writeUrlState() {
		const url = new URL(window.location.href);
		const dsl = dslInput.trim();
		if (dsl) url.searchParams.set('q', dsl);
		else url.searchParams.delete('q');
		if (viewMode !== 'table') url.searchParams.set('view', viewMode);
		else url.searchParams.delete('view');
		window.history.replaceState({}, '', url.toString());
	}

	function readUrlState(): { q: string | null; view: string | null } {
		const params = new URLSearchParams(window.location.search);
		return { q: params.get('q'), view: params.get('view') };
	}

	// ─── Lifecycle ──────────────────────────────────────────────────────

	onMount(() => {
		// Load history
		try {
			const stored = localStorage.getItem(HISTORY_KEY);
			if (stored) history = JSON.parse(stored);
		} catch { /* ignore */ }

		// Load analytics summary for suggestions
		getAnalyticsSummary().then(s => { summary = s; }).catch(() => {});

		// Restore from URL
		const state = readUrlState();
		if (state.view === 'grouped' || state.view === 'scatter') viewMode = state.view;
		if (state.q) {
			dslInput = state.q;
			applyDsl();
		}

		// Keyboard shortcut
		document.addEventListener('keydown', globalKeydown);
		return () => document.removeEventListener('keydown', globalKeydown);
	});

	function globalKeydown(e: KeyboardEvent) {
		if (e.key === '/' && document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
			e.preventDefault();
			searchInputEl?.focus();
		}
	}

	// ─── Query execution ────────────────────────────────────────────────

	async function runQuery() {
		loading = true;
		hasQueried = true;
		writeUrlState();
		const start = performance.now();
		try {
			const resolved: SpanFilter = { ...filter };
			if (resolved.since) {
				const iso = parseRelativeTime(resolved.since);
				if (iso) resolved.since = iso;
			}
			if (resolved.until) {
				const iso = parseRelativeTime(resolved.until);
				if (iso) resolved.until = iso;
			}
			const res = await getSpans(resolved);
			results = res.items;
			resultCount = res.total ?? results.length;
			queryTimeMs = Math.round(performance.now() - start);
			pushHistory(dslInput, resultCount);
		} catch {
			results = [];
			resultCount = 0;
			queryTimeMs = Math.round(performance.now() - start);
		} finally {
			loading = false;
		}
	}

	function applyDsl() {
		filter = parseDsl(dslInput);
		if (filter.sort_by) {
			sortField = filter.sort_by as SortField;
			sortAsc = filter.sort_order === 'asc';
		}
		runQuery();
	}

	function syncDslFromFilter() {
		dslInput = filterToDsl(filter);
		runQuery();
	}

	// ─── Keyboard nav in search ────────────────────────────────────────

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			if (selectedSuggestionIdx >= 0 && selectedSuggestionIdx < suggestions.length) {
				e.preventDefault();
				applySuggestion(suggestions[selectedSuggestionIdx]);
			} else {
				applyDsl();
				searchFocused = false;
			}
		} else if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedSuggestionIdx = Math.min(selectedSuggestionIdx + 1, suggestions.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedSuggestionIdx = Math.max(selectedSuggestionIdx - 1, -1);
		} else if (e.key === 'Escape') {
			searchFocused = false;
			searchInputEl?.blur();
		} else {
			selectedSuggestionIdx = -1;
		}
	}

	// ─── History ────────────────────────────────────────────────────────

	function pushHistory(dsl: string, count: number) {
		if (!dsl.trim()) return;
		const existing = history.findIndex((h) => h.dsl === dsl);
		if (existing >= 0) {
			history[existing] = { dsl, timestamp: Date.now(), resultCount: count };
			const item = history.splice(existing, 1)[0];
			history.unshift(item);
		} else {
			history.unshift({ dsl, timestamp: Date.now(), resultCount: count });
		}
		if (history.length > MAX_HISTORY) history = history.slice(0, MAX_HISTORY);
		try { localStorage.setItem(HISTORY_KEY, JSON.stringify(history)); } catch { /* ignore */ }
	}

	function loadHistory(entry: HistoryEntry) {
		dslInput = entry.dsl;
		applyDsl();
	}

	function clearHistory() {
		history = [];
		try { localStorage.removeItem(HISTORY_KEY); } catch { /* ignore */ }
	}

	// ─── Formatting helpers ─────────────────────────────────────────────

	function formatDuration(ms: number | null): string {
		if (ms === null) return '-';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(1)}s`;
	}

	function formatTime(iso: string): string {
		const d = new Date(iso);
		return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });
	}

	function formatDateTime(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const isToday = d.toDateString() === now.toDateString();
		if (isToday) return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });
		return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) + ' ' +
			d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
	}

	function formatTokens(span: Span): string {
		const k = span.kind;
		if (!k || k.type !== 'llm_call') return '-';
		const total = (k.input_tokens ?? 0) + (k.output_tokens ?? 0);
		if (total === 0) return '-';
		return `${total.toLocaleString()}`;
	}

	function formatCost(span: Span): string {
		const k = span.kind;
		if (!k || k.type !== 'llm_call') return '-';
		const cost = k.cost;
		if (cost === undefined || cost === null || cost === 0) return '-';
		if (cost < 0.001) return `$${cost.toFixed(6)}`;
		if (cost < 0.01) return `$${cost.toFixed(4)}`;
		return `$${cost.toFixed(3)}`;
	}

	function formatCostNum(n: number): string {
		if (n === 0) return '-';
		if (n < 0.001) return `$${n.toFixed(6)}`;
		if (n < 0.01) return `$${n.toFixed(4)}`;
		return `$${n.toFixed(3)}`;
	}

	function timeAgo(ts: number): string {
		const diff = Date.now() - ts;
		if (diff < 60_000) return 'just now';
		if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
		if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
		return `${Math.floor(diff / 86_400_000)}d ago`;
	}

	function scatterPointLabel(point: ScatterPoint): string {
		return `${point.span.name}, ${formatDuration(point.y)}, ${point.status}. Open trace ${shortId(point.span.trace_id)}.`;
	}

	function exportResults() {
		if (results.length === 0) return;
		const blob = new Blob([JSON.stringify(results, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `query-results-${resultCount}.json`;
		a.click();
		URL.revokeObjectURL(url);
	}

	function copyShareUrl() {
		writeUrlState();
		navigator.clipboard.writeText(window.location.href);
	}
</script>

<svelte:head>
	<title>Query | Traceway</title>
</svelte:head>

<div class="h-[calc(100vh-8rem)] flex flex-col gap-4">
	<!-- Search bar area -->
	<div class="shrink-0 px-1 pt-1 pb-1 space-y-3 sticky top-[4.3rem] z-20">
		<!-- Search input -->
		<div class="relative query-float-strong rounded-2xl p-2" role="search" aria-label="Trace query search">
			<div class="flex items-center rounded-xl border border-border/40 bg-bg/20 transition-all duration-200 focus-within:border-accent/55 focus-within:bg-bg/35 focus-within:shadow-[0_0_0_1px_color-mix(in_oklab,var(--color-accent)_28%,transparent)]">
				<div class="pl-3 pr-2 text-text-muted/80">
					<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
					</svg>
				</div>
				<input
					bind:this={searchInputEl}
					bind:value={dslInput}
					onfocus={() => { searchFocused = true; selectedSuggestionIdx = -1; }}
					onblur={() => setTimeout(() => searchFocused = false, 150)}
					onkeydown={handleKeydown}
					type="text"
					placeholder="Search spans... kind:llm_call model:gpt-4 since:1h  (press / to focus)"
					aria-label="Query spans"
					class="flex-1 bg-transparent py-3 text-[13px] font-mono text-text placeholder:text-text-muted/45 focus:outline-none"
				/>
				{#if dslInput}
					<button
						class="px-2 text-text-muted hover:text-text transition-colors"
						onclick={() => { dslInput = ''; filter = {}; searchInputEl?.focus(); }}
						aria-label="Clear search"
					>
						<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
						</svg>
					</button>
				{/if}
				<button
					onclick={applyDsl}
					disabled={loading}
					class="px-4 py-2 m-1.5 bg-accent text-bg rounded-lg text-xs font-semibold tracking-wide hover:bg-accent/90 disabled:opacity-50 transition-colors"
				>
					{loading ? '...' : 'Run'}
				</button>
			</div>
			<div class="px-1 pt-2 text-[10px] text-text-muted/70 font-mono">Use key:value filters, press <kbd class="query-kbd">Enter</kbd> to run, <kbd class="query-kbd">/</kbd> to focus.</div>

			<!-- Autocomplete dropdown -->
			{#if searchFocused && suggestions.length > 0}
				<div class="absolute left-0 right-0 top-full mt-2 z-30 query-float-strong rounded-xl shadow-xl overflow-hidden">
					{#each suggestions as s, i}
						<button
							class="w-full flex items-center gap-3 px-3 py-2.5 text-left transition-colors
								{i === selectedSuggestionIdx ? 'bg-accent/14' : 'hover:bg-bg-tertiary/70'}"
							onmousedown={() => applySuggestion(s)}
							onmouseenter={() => selectedSuggestionIdx = i}
						>
							<span class="text-[10px] uppercase tracking-wider text-text-muted/65 w-14 shrink-0 text-right">{s.category}</span>
							<span class="text-sm font-mono text-text">{s.label}</span>
							{#if s.description}
								<span class="text-xs text-text-muted/90 ml-auto">{s.description}</span>
							{/if}
						</button>
					{/each}
				</div>
			{/if}

			<!-- History dropdown (shown when focused with empty input) -->
			{#if searchFocused && !dslInput.trim() && history.length > 0 && suggestions.length === 0}
				<div class="absolute left-0 right-0 top-full mt-2 z-30 query-float-strong rounded-xl shadow-xl overflow-hidden">
					<div class="px-3 py-1.5 border-b border-border/70 flex items-center justify-between">
						<span class="text-[10px] uppercase tracking-wider text-text-muted">Recent queries</span>
						<button class="text-[10px] text-text-muted hover:text-danger transition-colors" onmousedown={clearHistory}>clear</button>
					</div>
					{#each history.slice(0, 8) as entry}
						<button
							class="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-bg-tertiary/75 transition-colors"
							onmousedown={() => loadHistory(entry)}
						>
							<span class="text-xs font-mono text-text truncate">{entry.dsl}</span>
							<span class="text-[10px] text-text-muted shrink-0 ml-3">{entry.resultCount} results &middot; {timeAgo(entry.timestamp)}</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Active filter pills + controls row -->
		<div class="query-float rounded-xl px-3 py-2.5 flex items-center gap-2.5 flex-wrap min-h-[42px]">
			{#if activePills.length > 0}
				{#each activePills as pill}
					<button
						onclick={() => removePill(pill)}
						class="query-chip query-chip-active"
					>
						<span class="font-mono">{pill.display}</span>
						<svg class="w-3 h-3 text-accent/55 group-hover:text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
						</svg>
					</button>
				{/each}
				<button onclick={clearAllFilters} class="query-chip text-text-muted hover:text-text">Clear all</button>
			{:else if !hasQueried}
				<!-- Quick filter presets -->
				{#each presets as preset}
					<button
						onclick={() => applyPreset(preset)}
						class="query-chip text-text-secondary hover:text-text"
					>
						{#if preset.icon === 'slow'}
							<svg class="w-3 h-3 text-warning" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
						{:else if preset.icon === 'error'}
							<svg class="w-3 h-3 text-danger" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" /></svg>
						{:else if preset.icon === 'cost'}
							<svg class="w-3 h-3 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v12m-3-2.818.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
						{:else if preset.icon === 'tokens'}
							<svg class="w-3 h-3 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25H12" /></svg>
						{:else}
							<svg class="w-3 h-3 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" /></svg>
						{/if}
						{preset.label}
					</button>
				{/each}
			{/if}

			<div class="flex-1"></div>

			{#if hasQueried}
				<!-- Results meta + view mode + actions -->
				<div class="flex items-center gap-3 text-xs text-text-muted shrink-0">
					<span>{resultCount.toLocaleString()} span{resultCount !== 1 ? 's' : ''} &middot; {queryTimeMs}ms</span>

					<!-- View mode toggle -->
					<div class="flex items-center rounded-lg border border-border/65 bg-bg-secondary/70 p-0.5 text-[10px]">
						<button
							class="px-2.5 py-1 rounded-md transition-colors {viewMode === 'table' ? 'bg-accent/18 text-accent' : 'text-text-muted hover:text-text'}"
							onclick={() => { viewMode = 'table'; writeUrlState(); }}
							title="Table view"
							aria-label="Switch to table view"
						>
							<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3.375 19.5h17.25m-17.25 0a1.125 1.125 0 0 1-1.125-1.125M3.375 19.5h7.5c.621 0 1.125-.504 1.125-1.125m-9.75 0V5.625m0 12.75v-1.5c0-.621.504-1.125 1.125-1.125m18.375 2.625V5.625m0 12.75c0 .621-.504 1.125-1.125 1.125m1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125m0 3.75h-7.5A1.125 1.125 0 0 1 12 18.375m9.75-12.75c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125m19.5 0v1.5c0 .621-.504 1.125-1.125 1.125M2.25 5.625v1.5c0 .621.504 1.125 1.125 1.125m0 0h17.25m-17.25 0h7.5c.621 0 1.125.504 1.125 1.125M3.375 8.25c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125m17.25-3.75h-7.5c-.621 0-1.125.504-1.125 1.125m8.625-1.125c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125m-17.25 0h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125M12 10.875v-1.5m0 1.5c0 .621-.504 1.125-1.125 1.125M12 10.875c0 .621.504 1.125 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125M12 12h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125M20.625 12c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125m-17.25 0h7.5M12 14.625v-1.5m0 1.5c0 .621-.504 1.125-1.125 1.125M12 14.625c0 .621.504 1.125 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125m0 0v.75" /></svg>
						</button>
						<button
							class="px-2.5 py-1 rounded-md transition-colors {viewMode === 'grouped' ? 'bg-accent/18 text-accent' : 'text-text-muted hover:text-text'}"
							onclick={() => { viewMode = 'grouped'; writeUrlState(); }}
							title="Grouped by trace"
							aria-label="Switch to grouped trace view"
						>
							<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 7.125C2.25 6.504 2.754 6 3.375 6h6c.621 0 1.125.504 1.125 1.125v3.75c0 .621-.504 1.125-1.125 1.125h-6a1.125 1.125 0 0 1-1.125-1.125v-3.75ZM14.25 8.625c0-.621.504-1.125 1.125-1.125h5.25c.621 0 1.125.504 1.125 1.125v8.25c0 .621-.504 1.125-1.125 1.125h-5.25a1.125 1.125 0 0 1-1.125-1.125v-8.25ZM3.75 16.125c0-.621.504-1.125 1.125-1.125h5.25c.621 0 1.125.504 1.125 1.125v2.25c0 .621-.504 1.125-1.125 1.125h-5.25a1.125 1.125 0 0 1-1.125-1.125v-2.25Z" /></svg>
						</button>
						<button
							class="px-2.5 py-1 rounded-md transition-colors {viewMode === 'scatter' ? 'bg-accent/18 text-accent' : 'text-text-muted hover:text-text'}"
							onclick={() => { viewMode = 'scatter'; writeUrlState(); }}
							title="Performance insights"
							aria-label="Switch to performance insights view"
						>
							<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z" /></svg>
						</button>
					</div>

					<button
						onclick={copyShareUrl}
						class="query-icon-button"
						title="Copy shareable URL"
						aria-label="Copy shareable URL"
					>
						<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M13.19 8.688a4.5 4.5 0 0 1 1.242 7.244l-4.5 4.5a4.5 4.5 0 0 1-6.364-6.364l1.757-1.757m9.86-2.54a4.5 4.5 0 0 0-1.242-7.244l-4.5-4.5a4.5 4.5 0 0 0-6.364 6.364L4.343 8.28" />
						</svg>
					</button>

					{#if results.length > 0}
						<button
							onclick={exportResults}
							class="query-icon-button"
							title="Export results as JSON"
							aria-label="Export results as JSON"
						>
							<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
							</svg>
						</button>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<!-- Results area -->
	<div class="flex-1 min-h-0 overflow-y-auto px-1 pb-1">
		{#if hasQueried}
			<div class="query-result-shell p-3 sm:p-4">
			{#if results.length === 0 && !loading}
				<div class="flex flex-col items-center justify-center h-64 text-text-muted">
					<svg class="w-10 h-10 mb-3 text-text-muted/30" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
					</svg>
					<p class="text-sm">No spans match your query</p>
					<p class="text-xs text-text-muted/60 mt-1">Try broadening your filters or changing the time range</p>
				</div>

			{:else if viewMode === 'table'}
				<!-- Table view -->
				<div class="query-float rounded-xl overflow-hidden">
					<div class="overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="text-left text-[11px] text-text-muted border-b border-border/70 bg-bg-secondary/65 uppercase tracking-wider">
									<th class="px-3 py-2 font-medium cursor-pointer hover:text-text select-none" onclick={() => toggleSort('name')}>
										Name <span class="text-accent">{sortIcon('name')}</span>
									</th>
									<th class="px-3 py-2 font-medium cursor-pointer hover:text-text select-none" onclick={() => toggleSort('kind')}>
										Kind <span class="text-accent">{sortIcon('kind')}</span>
									</th>
									<th class="px-3 py-2 font-medium cursor-pointer hover:text-text select-none" onclick={() => toggleSort('model')}>
										Model <span class="text-accent">{sortIcon('model')}</span>
									</th>
									<th class="px-3 py-2 font-medium cursor-pointer hover:text-text select-none" onclick={() => toggleSort('status')}>
										Status <span class="text-accent">{sortIcon('status')}</span>
									</th>
									<th class="px-3 py-2 font-medium">Trace</th>
									<th class="px-3 py-2 font-medium text-right cursor-pointer hover:text-text select-none" onclick={() => toggleSort('tokens')}>
										<span class="text-accent">{sortIcon('tokens')}</span> Tokens
									</th>
									<th class="px-3 py-2 font-medium text-right cursor-pointer hover:text-text select-none" onclick={() => toggleSort('cost')}>
										<span class="text-accent">{sortIcon('cost')}</span> Cost
									</th>
									<th class="px-3 py-2 font-medium text-right cursor-pointer hover:text-text select-none" onclick={() => toggleSort('duration')}>
										<span class="text-accent">{sortIcon('duration')}</span> Duration
									</th>
									<th class="px-3 py-2 font-medium text-right cursor-pointer hover:text-text select-none" onclick={() => toggleSort('started_at')}>
										<span class="text-accent">{sortIcon('started_at')}</span> Time
									</th>
								</tr>
							</thead>
							<tbody>
								{#each results as span}
									<tr
										class="border-b border-border/30 hover:bg-bg-secondary/55 cursor-pointer transition-colors"
										onclick={() => goto(`/traces/${span.trace_id}`)}
									>
										<td class="px-3 py-1.5 font-mono text-xs text-text truncate max-w-[200px]">{span.name}</td>
										<td class="px-3 py-1.5">
											{#if span.kind}
												<span class="inline-flex items-center gap-1">
													<SpanKindIcon {span} />
													<span class="text-[11px] text-text-muted">{spanKindLabel(span)}</span>
												</span>
											{:else}
												<span class="text-text-muted">-</span>
											{/if}
										</td>
										<td class="px-3 py-1.5 text-text-secondary font-mono text-xs">{span.kind?.type === 'llm_call' ? span.kind.model : '-'}</td>
										<td class="px-3 py-1.5">
											<StatusBadge status={spanStatus(span)} />
										</td>
										<td class="px-3 py-1.5 font-mono text-xs text-accent">{shortId(span.trace_id)}</td>
										<td class="px-3 py-1.5 text-right text-text-secondary font-mono text-xs tabular-nums">{formatTokens(span)}</td>
										<td class="px-3 py-1.5 text-right text-text-secondary font-mono text-xs tabular-nums">{formatCost(span)}</td>
										<td class="px-3 py-1.5 text-right text-text-secondary font-mono text-xs tabular-nums">{formatDuration(spanDurationMs(span))}</td>
										<td class="px-3 py-1.5 text-right text-text-muted text-xs tabular-nums">{formatTime(spanStartedAt(span))}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>

			{:else if viewMode === 'grouped'}
				<!-- Grouped by trace view -->
				<div class="space-y-3">
					{#each groupedByTrace as group}
						<div class="query-float rounded-xl overflow-hidden">
							<!-- Trace header -->
							<button
								onclick={() => goto(`/traces/${group.traceId}`)}
								class="w-full flex items-center gap-3 px-4 py-2.5 bg-bg-secondary/70 hover:bg-bg-tertiary/70 transition-colors text-left"
							>
								<span class="w-2 h-2 rounded-full shrink-0
									{group.status === 'failed' ? 'bg-danger' : group.status === 'running' ? 'bg-warning animate-pulse' : 'bg-success'}"></span>
								<span class="font-mono text-xs text-accent">{shortId(group.traceId)}</span>
								<span class="text-xs text-text-muted">{group.spans.length} span{group.spans.length !== 1 ? 's' : ''}</span>

								<div class="flex-1"></div>

								<div class="flex items-center gap-3 text-xs font-mono text-text-secondary">
									{#if group.totalTokens > 0}
										<span>{group.totalTokens.toLocaleString()} tok</span>
									{/if}
									{#if group.totalCost > 0}
										<span class="text-success">{formatCostNum(group.totalCost)}</span>
									{/if}
									{#if group.totalDuration !== null}
										<span>{formatDuration(group.totalDuration)}</span>
									{/if}
									<span class="text-text-muted">{formatDateTime(group.startedAt)}</span>
								</div>
							</button>

							<!-- Span rows -->
							<div>
								{#each group.spans as span}
									<button
										onclick={() => goto(`/traces/${span.trace_id}`)}
										class="w-full flex items-center gap-2 px-4 py-1.5 border-t border-border/30 hover:bg-bg-secondary/45 transition-colors text-left"
									>
										<div class="w-4 shrink-0">
											<SpanKindIcon {span} />
										</div>
										<span class="text-xs text-text truncate flex-1 font-mono">{span.name}</span>
										{#if span.kind?.type === 'llm_call' && span.kind.model}
											<span class="text-[10px] text-purple-400 bg-purple-400/10 rounded px-1 py-px shrink-0">{span.kind.model}</span>
										{/if}
										<StatusBadge status={spanStatus(span)} />
										<span class="text-xs text-text-secondary font-mono tabular-nums w-16 text-right shrink-0">{formatDuration(spanDurationMs(span))}</span>
									</button>
								{/each}
							</div>
						</div>
					{/each}
				</div>

			{:else if viewMode === 'scatter'}
				<!-- Performance insights view -->
				<div class="space-y-3">
					<!-- Summary stat cards -->
					<div class="grid grid-cols-2 md:grid-cols-5 gap-2">
						<div class="query-float rounded-lg px-3 py-2.5">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-1">Avg latency</div>
							<div class="text-lg font-mono font-semibold text-text">{formatDuration(insightsData.avgDuration)}</div>
						</div>
						<div class="query-float rounded-lg px-3 py-2.5">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-1">P95 latency</div>
							<div class="text-lg font-mono font-semibold text-warning">{formatDuration(insightsData.p95)}</div>
						</div>
						<div class="query-float rounded-lg px-3 py-2.5">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-1">Total tokens</div>
							<div class="text-lg font-mono font-semibold text-text">{insightsData.totalTokens.toLocaleString()}</div>
						</div>
						<div class="query-float rounded-lg px-3 py-2.5">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-1">Total cost</div>
							<div class="text-lg font-mono font-semibold text-success">{formatCostNum(insightsData.totalCost)}</div>
						</div>
						<div class="query-float rounded-lg px-3 py-2.5">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-1">Errors</div>
							<div class="text-lg font-mono font-semibold {insightsData.failedCount > 0 ? 'text-danger' : 'text-text'}">{insightsData.failedCount}</div>
						</div>
					</div>

					<!-- Scatter chart -->
					<div class="query-float rounded-xl overflow-hidden bg-bg-secondary/45">
						<div class="px-4 py-2 border-b border-border/70 flex items-center justify-between">
							<span class="text-xs text-text-muted uppercase tracking-wider">Latency distribution</span>
							<div class="flex items-center gap-3">
								<!-- Legend -->
								{#each insightsData.colorLabels.slice(0, 6) as label, i}
									<span class="inline-flex items-center gap-1 text-[10px] text-text-muted">
										<span class="w-2 h-2 rounded-full shrink-0" style="background: {MODEL_COLORS[i % MODEL_COLORS.length]}"></span>
										{label}
									</span>
								{/each}
								{#if insightsData.colorLabels.length > 6}
									<span class="text-[10px] text-text-muted">+{insightsData.colorLabels.length - 6} more</span>
								{/if}
							</div>
						</div>

						{#if insightsData.points.length === 0}
							<div class="flex items-center justify-center h-64 text-text-muted text-sm">
								No spans with measurable duration
							</div>
						{:else}
							<div class="relative h-72 mx-4 mt-3 mb-6">
								<!-- Y axis labels (log scale) -->
								<div class="absolute left-0 top-0 bottom-6 w-14 flex flex-col justify-between text-[10px] text-text-muted font-mono text-right pr-2">
									<span>{formatDuration(insightsData.yMax)}</span>
									<span>{formatDuration(Math.pow(10, (insightsData.logYMin + insightsData.logYMax) / 2))}</span>
									<span>{formatDuration(insightsData.yMin)}</span>
								</div>

								<!-- Plot area -->
								<div class="absolute left-16 right-0 top-0 bottom-6 border-l border-b border-border/50 overflow-hidden">
									<!-- Percentile reference lines -->
									<div class="absolute left-0 right-0 border-t border-dashed border-text-muted/20" style="top: {(1 - percentileLogY(insightsData.p50)) * 100}%">
										<span class="absolute -top-2.5 right-1 text-[9px] text-text-muted/40 font-mono">p50 {formatDuration(insightsData.p50)}</span>
									</div>
									<div class="absolute left-0 right-0 border-t border-dashed border-warning/30" style="top: {(1 - percentileLogY(insightsData.p95)) * 100}%">
										<span class="absolute -top-2.5 right-1 text-[9px] text-warning/50 font-mono">p95 {formatDuration(insightsData.p95)}</span>
									</div>
									{#if insightsData.p99 !== insightsData.p95}
										<div class="absolute left-0 right-0 border-t border-dashed border-danger/30" style="top: {(1 - percentileLogY(insightsData.p99)) * 100}%">
											<span class="absolute -top-2.5 right-1 text-[9px] text-danger/50 font-mono">p99 {formatDuration(insightsData.p99)}</span>
										</div>
									{/if}

									<!-- Points -->
									{#each insightsData.points as point}
										{@const xPct = ((point.x - insightsData.xMin) / (insightsData.xMax - insightsData.xMin)) * 100}
										{@const yPct = (1 - point.logY) * 100}
										{@const size = Math.max(5, Math.min(20, Math.sqrt(point.tokens / 20) + 4))}
										{@const color = point.status === 'failed' ? '#ef4444' : MODEL_COLORS[point.colorIdx % MODEL_COLORS.length]}
										<button
											class="absolute rounded-full transition-all duration-100 hover:ring-2 hover:ring-white/30 hover:z-10"
											style="left: {xPct}%; top: {yPct}%; width: {size}px; height: {size}px; transform: translate(-50%, -50%); background: {color}; opacity: {hoveredPoint && hoveredPoint !== point ? 0.25 : 0.75}"
											onclick={() => goto(`/traces/${point.span.trace_id}`)}
											onmouseenter={() => hoveredPoint = point}
											onmouseleave={() => hoveredPoint = null}
											aria-label={scatterPointLabel(point)}
											title={scatterPointLabel(point)}
										></button>
									{/each}

									<!-- Hover tooltip -->
									{#if hoveredPoint}
										{@const xPct = ((hoveredPoint.x - insightsData.xMin) / (insightsData.xMax - insightsData.xMin)) * 100}
										{@const yPct = (1 - hoveredPoint.logY) * 100}
										{@const color = hoveredPoint.status === 'failed' ? '#ef4444' : MODEL_COLORS[hoveredPoint.colorIdx % MODEL_COLORS.length]}
										<div
											class="absolute z-20 bg-bg-tertiary border border-border rounded-lg px-3 py-2 shadow-xl pointer-events-none text-xs space-y-1 min-w-[200px]"
											style="left: {Math.min(Math.max(xPct, 15), 85)}%; top: {yPct < 30 ? yPct + 5 : yPct - 25}%; transform: translateX(-50%)"
										>
											<div class="flex items-center gap-2">
												<span class="w-2 h-2 rounded-full shrink-0" style="background: {color}"></span>
												<span class="font-mono text-text font-medium truncate">{hoveredPoint.span.name}</span>
											</div>
											<div class="grid grid-cols-2 gap-x-4 gap-y-0.5 text-text-muted">
												<span>Duration</span>
												<span class="text-text font-mono text-right">{formatDuration(hoveredPoint.y)}</span>
												{#if hoveredPoint.tokens > 0}
													<span>Tokens</span>
													<span class="text-text font-mono text-right">{hoveredPoint.tokens.toLocaleString()}</span>
												{/if}
												{#if hoveredPoint.span.kind?.type === 'llm_call' && hoveredPoint.span.kind.cost}
													<span>Cost</span>
													<span class="text-success font-mono text-right">{formatCost(hoveredPoint.span)}</span>
												{/if}
												{#if hoveredPoint.span.kind?.type === 'llm_call'}
													<span>Model</span>
													<span class="text-text font-mono text-right">{hoveredPoint.span.kind.model}</span>
												{/if}
											</div>
											<div class="text-text-muted/50 text-[10px]">{formatDateTime(spanStartedAt(hoveredPoint.span))}</div>
										</div>
									{/if}
								</div>

								<!-- X axis labels -->
								<div class="absolute left-16 right-0 bottom-0 h-5 flex justify-between text-[10px] text-text-muted font-mono">
									<span>{formatDateTime(new Date(insightsData.xMin).toISOString())}</span>
									{#if insightsData.points.length > 2}
										{@const midTime = insightsData.xMin + (insightsData.xMax - insightsData.xMin) / 2}
										<span>{formatDateTime(new Date(midTime).toISOString())}</span>
									{/if}
									<span>{formatDateTime(new Date(insightsData.xMax).toISOString())}</span>
								</div>

								<!-- Y axis title -->
								<div class="absolute -left-1 top-1/2 -translate-y-1/2 -rotate-90 text-[9px] text-text-muted/40 uppercase tracking-widest whitespace-nowrap">Duration (log)</div>
							</div>
						{/if}
					</div>

					<!-- Outliers table (only show top slowest spans) -->
					{#if insightsData.points.length > 0}
						{@const slowest = [...results].filter(s => spanDurationMs(s) !== null).sort((a, b) => (spanDurationMs(b) ?? 0) - (spanDurationMs(a) ?? 0)).slice(0, 20)}
						<div class="query-float rounded-xl overflow-hidden">
							<div class="px-3 py-2 bg-bg-secondary/60 border-b border-border/70 text-[11px] text-text-muted uppercase tracking-wider">
								Slowest spans
							</div>
							<table class="w-full text-sm">
								<thead>
									<tr class="text-left text-[11px] text-text-muted border-b border-border/70 bg-bg-secondary/50 uppercase tracking-wider">
										<th class="px-3 py-1.5 font-medium">Name</th>
										<th class="px-3 py-1.5 font-medium">Model</th>
										<th class="px-3 py-1.5 font-medium">Status</th>
										<th class="px-3 py-1.5 font-medium text-right">Tokens</th>
										<th class="px-3 py-1.5 font-medium text-right">Cost</th>
										<th class="px-3 py-1.5 font-medium text-right">Duration</th>
										<th class="px-3 py-1.5 font-medium text-right">Time</th>
									</tr>
								</thead>
								<tbody>
									{#each slowest as span}
										{@const dur = spanDurationMs(span) ?? 0}
										{@const isOutlier = dur >= insightsData.p95}
										<tr
											class="border-b border-border/30 cursor-pointer transition-colors
												{isOutlier ? 'hover:bg-warning/5 bg-warning/[0.02]' : 'hover:bg-bg-secondary/50'}"
											onclick={() => goto(`/traces/${span.trace_id}`)}
										>
											<td class="px-3 py-1.5 font-mono text-xs text-text truncate max-w-[200px]">{span.name}</td>
											<td class="px-3 py-1.5 text-text-secondary font-mono text-xs">{span.kind?.type === 'llm_call' ? span.kind.model : '-'}</td>
											<td class="px-3 py-1.5"><StatusBadge status={spanStatus(span)} /></td>
											<td class="px-3 py-1.5 text-right text-text-secondary font-mono text-xs tabular-nums">{formatTokens(span)}</td>
											<td class="px-3 py-1.5 text-right text-text-secondary font-mono text-xs tabular-nums">{formatCost(span)}</td>
											<td class="px-3 py-1.5 text-right font-mono text-xs tabular-nums {isOutlier ? 'text-warning font-semibold' : 'text-text-secondary'}">{formatDuration(dur)}</td>
											<td class="px-3 py-1.5 text-right text-text-muted text-xs tabular-nums">{formatTime(spanStartedAt(span))}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</div>
			{/if}
			</div>

		{:else}
			<!-- Empty state -->
			<div class="flex flex-col items-center justify-center h-full max-h-96 text-center">
				<div class="w-16 h-16 rounded-full bg-bg-secondary flex items-center justify-center mb-4">
					<svg class="w-7 h-7 text-text-muted/40" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
						<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
					</svg>
				</div>
				<h2 class="text-sm font-medium text-text mb-1">Query your spans</h2>
				<p class="text-xs text-text-muted mb-4 max-w-sm">
					Search across all traces to find specific LLM calls, debug errors, or analyze costs.
					Use the search bar above or click a preset to get started.
				</p>
				<div class="flex flex-col gap-1 text-xs text-text-muted/60 font-mono">
					<span>kind:llm_call model:gpt-4 since:1h</span>
					<span>status:failed duration:>500ms</span>
					<span>tokens:>1000 cost:>0.01 sort:cost</span>
				</div>
			</div>
		{/if}
	</div>
</div>
