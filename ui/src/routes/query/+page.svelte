<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		getSpans,
		spanStatus,
		spanStartedAt,
		spanDurationMs,
		spanKindLabel,
		shortId,
		type Span,
		type SpanFilter
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

	interface HistoryEntry {
		dsl: string;
		timestamp: number;
		resultCount: number;
	}

	let history: HistoryEntry[] = $state([]);
	let historyOpen = $state(false);

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
			sortAsc = false; // default desc for new column
		}
		// Update filter and re-run
		filter.sort_by = field;
		filter.sort_order = sortAsc ? 'asc' : 'desc';
		syncDslFromFilter();
	}

	function sortIcon(field: SortField): string {
		if (sortField !== field) return '';
		return sortAsc ? '\u2191' : '\u2193';
	}

	// ─── Relative time presets for the Since dropdown ────────────────────

	const sincePresets = [
		{ label: 'Any time', value: '' },
		{ label: 'Last 5m', value: '5m' },
		{ label: 'Last 15m', value: '15m' },
		{ label: 'Last 1h', value: '1h' },
		{ label: 'Last 6h', value: '6h' },
		{ label: 'Last 24h', value: '24h' },
		{ label: 'Last 7d', value: '7d' }
	];

	// ─── URL state ──────────────────────────────────────────────────────

	function writeUrlState() {
		const url = new URL(window.location.href);
		const dsl = dslInput.trim();
		if (dsl) {
			url.searchParams.set('q', dsl);
		} else {
			url.searchParams.delete('q');
		}
		// Use replaceState to avoid polluting history
		window.history.replaceState({}, '', url.toString());
	}

	function readUrlState(): string | null {
		const params = new URLSearchParams(window.location.search);
		return params.get('q');
	}

	// ─── Lifecycle ──────────────────────────────────────────────────────

	onMount(() => {
		try {
			const stored = localStorage.getItem(HISTORY_KEY);
			if (stored) history = JSON.parse(stored);
		} catch {
			// ignore
		}

		// Restore from URL
		const q = readUrlState();
		if (q) {
			dslInput = q;
			applyDsl();
		}
	});

	// ─── Query execution ────────────────────────────────────────────────

	async function runQuery() {
		loading = true;
		hasQueried = true;
		writeUrlState();
		const start = performance.now();
		try {
			// Resolve relative times before sending to API
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
			results = res.spans;
			resultCount = res.count;
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

	// ─── Sync: DSL → filter ─────────────────────────────────────────────

	function applyDsl() {
		filter = parseDsl(dslInput);
		// Restore sort state from filter
		if (filter.sort_by) {
			sortField = filter.sort_by as SortField;
			sortAsc = filter.sort_order === 'asc';
		}
		runQuery();
	}

	// ─── Sync: filter → DSL ─────────────────────────────────────────────

	function syncDslFromFilter() {
		dslInput = filterToDsl(filter);
		runQuery();
	}

	// ─── Structured filter change handlers ──────────────────────────────

	function onFilterChange(key: keyof SpanFilter, value: string) {
		if (value) {
			filter[key] = value;
		} else {
			delete filter[key];
		}
		syncDslFromFilter();
	}

	// ─── History ────────────────────────────────────────────────────────

	function pushHistory(dsl: string, count: number) {
		if (!dsl.trim()) return;
		const existing = history.findIndex((h) => h.dsl === dsl);
		if (existing >= 0) {
			history[existing] = { dsl, timestamp: Date.now(), resultCount: count };
			// move to front
			const item = history.splice(existing, 1)[0];
			history.unshift(item);
		} else {
			history.unshift({ dsl, timestamp: Date.now(), resultCount: count });
		}
		if (history.length > MAX_HISTORY) history = history.slice(0, MAX_HISTORY);
		try {
			localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
		} catch {
			// ignore
		}
	}

	function loadHistory(entry: HistoryEntry) {
		dslInput = entry.dsl;
		applyDsl();
	}

	function clearHistory() {
		history = [];
		try {
			localStorage.removeItem(HISTORY_KEY);
		} catch {
			// ignore
		}
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

	function formatTokens(span: Span): string {
		const k = span.kind;
		if (!k || k.type !== 'llm_call') return '-';
		const inp = k.input_tokens ?? 0;
		const out = k.output_tokens ?? 0;
		const total = inp + out;
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

	function timeAgo(ts: number): string {
		const diff = Date.now() - ts;
		if (diff < 60_000) return 'just now';
		if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
		if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
		return `${Math.floor(diff / 86_400_000)}d ago`;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') applyDsl();
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

<div class="space-y-4">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<h1 class="text-lg font-semibold text-text">Span Query</h1>
		{#if hasQueried && dslInput.trim()}
			<button
				onclick={copyShareUrl}
				class="text-xs text-text-muted hover:text-accent transition-colors flex items-center gap-1"
				title="Copy shareable URL"
			>
				<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d="M13.19 8.688a4.5 4.5 0 011.242 7.244l-4.5 4.5a4.5 4.5 0 01-6.364-6.364l1.757-1.757m9.86-2.54a4.5 4.5 0 00-1.242-7.244l-4.5-4.5a4.5 4.5 0 00-6.364 6.364L4.343 8.28" />
				</svg>
				Share
			</button>
		{/if}
	</div>

	<!-- DSL Search Bar -->
	<div class="flex gap-2">
		<input
			type="text"
			bind:value={dslInput}
			onkeydown={handleKeydown}
			placeholder="kind:llm_call model:gpt-4 since:1h duration:>500ms tokens:>100 cost:>0.01"
			class="flex-1 bg-bg-secondary border border-border rounded px-3 py-2 text-sm font-mono text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
		/>
		<button
			onclick={applyDsl}
			disabled={loading}
			class="px-4 py-2 bg-accent text-bg rounded text-sm font-medium hover:bg-accent/90 disabled:opacity-50 transition-colors"
		>
			{loading ? 'Running...' : 'Run'}
		</button>
	</div>

	<!-- Structured Filters -->
	<div class="flex flex-wrap gap-3 text-sm">
		<div class="flex items-center gap-1.5">
			<label for="f-kind" class="text-text-muted text-xs">Kind</label>
			<select
				id="f-kind"
				value={filter.kind ?? ''}
				onchange={(e) => onFilterChange('kind', e.currentTarget.value)}
				class="bg-bg-secondary border border-border rounded px-2 py-1 text-text text-xs"
			>
				<option value="">All</option>
				<option value="llm_call">LLM Call</option>
				<option value="fs_read">FS Read</option>
				<option value="fs_write">FS Write</option>
				<option value="custom">Custom</option>
			</select>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-model" class="text-text-muted text-xs">Model</label>
			<input
				id="f-model"
				type="text"
				value={filter.model ?? ''}
				onchange={(e) => onFilterChange('model', e.currentTarget.value)}
				placeholder="gpt-4"
				class="w-24 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-xs placeholder:text-text-muted"
			/>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-status" class="text-text-muted text-xs">Status</label>
			<select
				id="f-status"
				value={filter.status ?? ''}
				onchange={(e) => onFilterChange('status', e.currentTarget.value)}
				class="bg-bg-secondary border border-border rounded px-2 py-1 text-text text-xs"
			>
				<option value="">All</option>
				<option value="running">Running</option>
				<option value="completed">Completed</option>
				<option value="failed">Failed</option>
			</select>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-name" class="text-text-muted text-xs">Name</label>
			<input
				id="f-name"
				type="text"
				value={filter.name_contains ?? ''}
				onchange={(e) => onFilterChange('name_contains', e.currentTarget.value)}
				placeholder="span name"
				class="w-24 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-xs placeholder:text-text-muted"
			/>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-since" class="text-text-muted text-xs">Since</label>
			<select
				id="f-since"
				value={filter.since ?? ''}
				onchange={(e) => onFilterChange('since', e.currentTarget.value)}
				class="bg-bg-secondary border border-border rounded px-2 py-1 text-text text-xs"
			>
				{#each sincePresets as preset}
					<option value={preset.value}>{preset.label}</option>
				{/each}
			</select>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-provider" class="text-text-muted text-xs">Provider</label>
			<input
				id="f-provider"
				type="text"
				value={filter.provider ?? ''}
				onchange={(e) => onFilterChange('provider', e.currentTarget.value)}
				placeholder="openai"
				class="w-20 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-xs placeholder:text-text-muted"
			/>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-trace" class="text-text-muted text-xs">Trace</label>
			<input
				id="f-trace"
				type="text"
				value={filter.trace_id ?? ''}
				onchange={(e) => onFilterChange('trace_id', e.currentTarget.value)}
				placeholder="trace id"
				class="w-24 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-xs placeholder:text-text-muted font-mono"
			/>
		</div>
	</div>

	<!-- Results -->
	{#if hasQueried}
		<div class="border border-border rounded overflow-hidden">
			<!-- Results header -->
			<div class="px-3 py-2 bg-bg-secondary border-b border-border flex items-center gap-3 text-xs text-text-muted">
				<span class="font-medium">{resultCount.toLocaleString()} span{resultCount !== 1 ? 's' : ''}</span>
				<span class="text-border">|</span>
				<span>{queryTimeMs}ms</span>
				{#if results.length > 0}
					<div class="flex-1"></div>
					<button
						class="text-text-secondary hover:text-text transition-colors"
						onclick={exportResults}
					>Export JSON</button>
				{/if}
			</div>

			{#if results.length > 0}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="text-left text-[11px] text-text-muted border-b border-border bg-bg-secondary/50 uppercase tracking-wider">
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
									class="border-b border-border/30 hover:bg-bg-secondary/50 cursor-pointer transition-colors"
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
			{:else}
				<div class="px-3 py-8 text-center text-text-muted text-sm">
					No spans match the current query.
				</div>
			{/if}
		</div>
	{:else}
		<!-- Empty state with DSL help -->
		<div class="border border-border/50 rounded p-6 text-center space-y-3">
			<p class="text-text-muted text-sm">Search your spans using the query bar above.</p>
			<div class="text-xs text-text-muted space-y-1 font-mono">
				<p>kind:llm_call model:gpt-4 since:1h</p>
				<p>status:failed duration:>500ms</p>
				<p>tokens:>1000 cost:>0.01</p>
			</div>
		</div>
	{/if}

	<!-- Query History -->
	{#if history.length > 0}
		<details bind:open={historyOpen}>
			<summary class="cursor-pointer text-xs text-text-muted hover:text-text transition-colors select-none">
				History ({history.length})
			</summary>
			<div class="mt-2 border border-border rounded overflow-hidden">
				{#each history as entry}
					<button
						class="w-full flex items-center justify-between px-3 py-1.5 text-xs hover:bg-bg-secondary/50 border-b border-border/30 last:border-b-0 transition-colors text-left"
						onclick={() => loadHistory(entry)}
					>
						<span class="font-mono text-text truncate">{entry.dsl}</span>
						<span class="text-text-muted shrink-0 ml-3">
							{entry.resultCount} result{entry.resultCount !== 1 ? 's' : ''} &middot; {timeAgo(entry.timestamp)}
						</span>
					</button>
				{/each}
				<div class="px-3 py-1.5 bg-bg-secondary border-t border-border">
					<button
						class="text-xs text-danger hover:text-danger/80 transition-colors"
						onclick={clearHistory}
					>
						clear
					</button>
				</div>
			</div>
		</details>
	{/if}
</div>
