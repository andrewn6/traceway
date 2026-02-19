<script lang="ts">
	import { goto } from '$app/navigation';
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

	const HISTORY_KEY = 'llmtrace:query-history';
	const MAX_HISTORY = 50;

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

	// ─── Lifecycle ──────────────────────────────────────────────────────

	onMount(() => {
		try {
			const stored = localStorage.getItem(HISTORY_KEY);
			if (stored) history = JSON.parse(stored);
		} catch {
			// ignore
		}
	});

	// ─── Query execution ────────────────────────────────────────────────

	async function runQuery() {
		loading = true;
		hasQueried = true;
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
</script>

<svelte:head>
	<title>Query | llmtrace</title>
</svelte:head>

<div class="space-y-4">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<h1 class="text-lg font-semibold text-text">Span Query Explorer</h1>
	</div>

	<!-- DSL Search Bar -->
	<div class="flex gap-2">
		<input
			type="text"
			bind:value={dslInput}
			onkeydown={handleKeydown}
			placeholder="kind:llm_call model:gpt-4 since:1h — or just type a span name"
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
			<label for="f-kind" class="text-text-muted">Kind</label>
			<select
				id="f-kind"
				value={filter.kind ?? ''}
				onchange={(e) => onFilterChange('kind', e.currentTarget.value)}
				class="bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm"
			>
				<option value="">All</option>
				<option value="llm_call">LLM Call</option>
				<option value="fs_read">FS Read</option>
				<option value="fs_write">FS Write</option>
				<option value="custom">Custom</option>
			</select>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-model" class="text-text-muted">Model</label>
			<input
				id="f-model"
				type="text"
				value={filter.model ?? ''}
				onchange={(e) => onFilterChange('model', e.currentTarget.value)}
				placeholder="gpt-4"
				class="w-28 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm placeholder:text-text-muted"
			/>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-provider" class="text-text-muted">Provider</label>
			<input
				id="f-provider"
				type="text"
				value={filter.provider ?? ''}
				onchange={(e) => onFilterChange('provider', e.currentTarget.value)}
				placeholder="openai"
				class="w-24 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm placeholder:text-text-muted"
			/>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-status" class="text-text-muted">Status</label>
			<select
				id="f-status"
				value={filter.status ?? ''}
				onchange={(e) => onFilterChange('status', e.currentTarget.value)}
				class="bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm"
			>
				<option value="">All</option>
				<option value="running">Running</option>
				<option value="completed">Completed</option>
				<option value="failed">Failed</option>
			</select>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-name" class="text-text-muted">Name</label>
			<input
				id="f-name"
				type="text"
				value={filter.name_contains ?? ''}
				onchange={(e) => onFilterChange('name_contains', e.currentTarget.value)}
				placeholder="span name"
				class="w-28 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm placeholder:text-text-muted"
			/>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-path" class="text-text-muted">Path</label>
			<input
				id="f-path"
				type="text"
				value={filter.path ?? ''}
				onchange={(e) => onFilterChange('path', e.currentTarget.value)}
				placeholder="/src/..."
				class="w-28 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm placeholder:text-text-muted"
			/>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-since" class="text-text-muted">Since</label>
			<select
				id="f-since"
				value={filter.since ?? ''}
				onchange={(e) => onFilterChange('since', e.currentTarget.value)}
				class="bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm"
			>
				{#each sincePresets as preset}
					<option value={preset.value}>{preset.label}</option>
				{/each}
			</select>
		</div>

		<div class="flex items-center gap-1.5">
			<label for="f-trace" class="text-text-muted">Trace</label>
			<input
				id="f-trace"
				type="text"
				value={filter.trace_id ?? ''}
				onchange={(e) => onFilterChange('trace_id', e.currentTarget.value)}
				placeholder="trace id"
				class="w-28 bg-bg-secondary border border-border rounded px-2 py-1 text-text text-sm placeholder:text-text-muted font-mono"
			/>
		</div>
	</div>

	<!-- Results -->
	{#if hasQueried}
		<div class="border border-border rounded overflow-hidden">
			<!-- Results header -->
			<div class="px-3 py-2 bg-bg-secondary border-b border-border flex items-center gap-2 text-xs text-text-muted">
				<span>{resultCount} span{resultCount !== 1 ? 's' : ''} found</span>
				<span>&middot;</span>
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
							<tr class="text-left text-xs text-text-muted border-b border-border">
								<th class="px-3 py-2 font-medium">Name</th>
								<th class="px-3 py-2 font-medium">Kind</th>
								<th class="px-3 py-2 font-medium">Model</th>
								<th class="px-3 py-2 font-medium">Status</th>
								<th class="px-3 py-2 font-medium">Trace</th>
								<th class="px-3 py-2 font-medium text-right">Duration</th>
								<th class="px-3 py-2 font-medium text-right">Time</th>
							</tr>
						</thead>
						<tbody>
							{#each results as span}
								<tr
									class="border-b border-border/50 hover:bg-bg-secondary/50 cursor-pointer transition-colors"
									onclick={() => goto(`/traces/${span.trace_id}`)}
								>
									<td class="px-3 py-2 font-mono text-text truncate max-w-xs">{span.name}</td>
									<td class="px-3 py-2">
										{#if span.kind}
											<span class="inline-flex items-center gap-1">
												<SpanKindIcon {span} />
												<span class="text-xs text-text-muted">{spanKindLabel(span)}</span>
											</span>
										{:else}
											<span class="text-text-muted">-</span>
										{/if}
									</td>
									<td class="px-3 py-2 text-text-secondary font-mono text-xs">{span.kind?.type === 'llm_call' ? span.kind.model : '-'}</td>
									<td class="px-3 py-2">
										<StatusBadge status={spanStatus(span)} />
									</td>
									<td class="px-3 py-2 font-mono text-xs text-accent">{shortId(span.trace_id)}</td>
									<td class="px-3 py-2 text-right text-text-secondary font-mono text-xs">{formatDuration(spanDurationMs(span))}</td>
									<td class="px-3 py-2 text-right text-text-muted text-xs">{formatTime(spanStartedAt(span))}</td>
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
	{/if}

	<!-- Query History -->
	{#if history.length > 0}
		<details bind:open={historyOpen}>
			<summary class="cursor-pointer text-sm text-text-muted hover:text-text transition-colors select-none">
				Query History ({history.length})
			</summary>
			<div class="mt-2 border border-border rounded overflow-hidden">
				{#each history as entry}
					<button
						class="w-full flex items-center justify-between px-3 py-2 text-sm hover:bg-bg-secondary/50 border-b border-border/50 last:border-b-0 transition-colors text-left"
						onclick={() => loadHistory(entry)}
					>
						<span class="font-mono text-text truncate">{entry.dsl}</span>
						<span class="text-xs text-text-muted shrink-0 ml-3">
							{entry.resultCount} result{entry.resultCount !== 1 ? 's' : ''} &middot; {timeAgo(entry.timestamp)}
						</span>
					</button>
				{/each}
				<div class="px-3 py-2 bg-bg-secondary border-t border-border">
					<button
						class="text-xs text-danger hover:text-danger/80 transition-colors"
						onclick={clearHistory}
					>
						clear history
					</button>
				</div>
			</div>
		</details>
	{/if}
</div>
