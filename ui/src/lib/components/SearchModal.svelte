<script lang="ts">
	import { getSpans, getTraces, getDatasets, spanStatus, spanStartedAt, spanDurationMs, shortId, type Span, type SpanFilter, type Trace, type DatasetWithCount } from '$lib/api';
	import SpanKindIcon from './SpanKindIcon.svelte';
	import StatusBadge from './StatusBadge.svelte';
	import { goto } from '$app/navigation';

	let { open = $bindable(false) }: { open: boolean } = $props();

	let query = $state('');
	let loading = $state(false);
	let searched = $state(false);
	let selectedIdx = $state(0);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let inputEl: HTMLInputElement | undefined = $state(undefined);

	// Result types
	interface PageResult { type: 'page'; label: string; description: string; href: string; icon: string; }
	interface TraceResult { type: 'trace'; trace: Trace; }
	interface SpanResult { type: 'span'; span: Span; }
	interface DatasetResult { type: 'dataset'; dataset: DatasetWithCount; }
	type Result = PageResult | TraceResult | SpanResult | DatasetResult;

	let results: Result[] = $state([]);

	// ── Static pages ──────────────────────────────────────────────────
	const pages: PageResult[] = [
		{ type: 'page', label: 'Dashboard', description: 'Overview and stats', href: '/', icon: 'dashboard' },
		{ type: 'page', label: 'Traces', description: 'View all traces', href: '/traces', icon: 'trace' },
		{ type: 'page', label: 'Query', description: 'Search and filter spans', href: '/query', icon: 'query' },
		{ type: 'page', label: 'Analytics', description: 'Charts and metrics', href: '/analytics', icon: 'analytics' },
		{ type: 'page', label: 'Datasets', description: 'Manage datasets and datapoints', href: '/datasets', icon: 'dataset' },
		{ type: 'page', label: 'Review', description: 'Human annotation queue', href: '/review', icon: 'review' },
		{ type: 'page', label: 'Providers', description: 'LLM provider connections', href: '/settings/providers', icon: 'provider' },
		{ type: 'page', label: 'Settings', description: 'Project settings', href: '/settings', icon: 'settings' },
		{ type: 'page', label: 'Team', description: 'Team members and invites', href: '/settings/team', icon: 'team' },
		{ type: 'page', label: 'API Keys', description: 'Manage API keys', href: '/settings/api-keys', icon: 'key' },
		{ type: 'page', label: 'Billing', description: 'Plan and usage', href: '/settings/billing', icon: 'billing' },
	];

	$effect(() => {
		if (open && inputEl) {
			setTimeout(() => inputEl?.focus(), 50);
		}
		if (!open) {
			query = '';
			results = [];
			searched = false;
			selectedIdx = 0;
		}
	});

	function close() {
		open = false;
	}

	$effect(() => {
		doSearch(query);
	});

	function doSearch(text: string) {
		if (debounceTimer) clearTimeout(debounceTimer);
		const q = text.trim().toLowerCase();

		if (!q) {
			// Show page navigation when empty
			results = pages.slice(0, 8);
			searched = false;
			selectedIdx = 0;
			return;
		}

		// Immediately filter pages
		const matchedPages = pages.filter(
			(p) => p.label.toLowerCase().includes(q) || p.description.toLowerCase().includes(q)
		);

		// Show pages immediately, then search data
		results = matchedPages;
		selectedIdx = 0;

		debounceTimer = setTimeout(async () => {
			loading = true;
			try {
				const dataResults: Result[] = [...matchedPages];

				// Search traces by name
				const [tracesRes, datasetsRes, spansRes] = await Promise.all([
					getTraces().catch(() => ({ items: [] as Trace[] })),
					getDatasets().catch(() => ({ datasets: [] as DatasetWithCount[] })),
					getSpans({ text_contains: text.trim() } as SpanFilter).catch(() => ({ items: [] as Span[] })),
				]);

				// Filter traces client-side by name
				const matchedTraces = tracesRes.items
					.filter((t: Trace) => t.name?.toLowerCase().includes(q) || t.id.toLowerCase().includes(q))
					.slice(0, 5)
					.map((t: Trace): TraceResult => ({ type: 'trace', trace: t }));

				// Filter datasets client-side by name
				const matchedDatasets = datasetsRes.datasets
					.filter((d: DatasetWithCount) => d.name.toLowerCase().includes(q) || d.id.toLowerCase().includes(q))
					.slice(0, 3)
					.map((d: DatasetWithCount): DatasetResult => ({ type: 'dataset', dataset: d }));

				// Span results from text search
				const matchedSpans = spansRes.items
					.slice(0, 8)
					.map((s: Span): SpanResult => ({ type: 'span', span: s }));

				dataResults.push(...matchedTraces, ...matchedDatasets, ...matchedSpans);
				results = dataResults;
				searched = true;
				selectedIdx = Math.min(selectedIdx, dataResults.length - 1);
			} catch {
				// keep page results
			}
			loading = false;
		}, 300);
	}

	function navigate(result: Result) {
		close();
		switch (result.type) {
			case 'page':
				goto(result.href);
				break;
			case 'trace':
				goto(`/traces/${result.trace.id}`);
				break;
			case 'span':
				goto(`/traces/${result.span.trace_id}`);
				break;
			case 'dataset':
				goto(`/datasets/${result.dataset.id}`);
				break;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			close();
			e.preventDefault();
		} else if (e.key === 'ArrowDown') {
			selectedIdx = Math.min(selectedIdx + 1, results.length - 1);
			e.preventDefault();
		} else if (e.key === 'ArrowUp') {
			selectedIdx = Math.max(selectedIdx - 1, 0);
			e.preventDefault();
		} else if (e.key === 'Enter' && results.length > 0) {
			navigate(results[selectedIdx]);
			e.preventDefault();
		}
	}

	// ── Helpers ────────────────────────────────────────────────────────
	function formatDuration(ms: number | null): string {
		if (ms === null) return '...';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function extractSnippet(span: Span, needle: string): string | null {
		const lowerNeedle = needle.toLowerCase();
		for (const field of [span.input, span.output]) {
			if (!field) continue;
			const text = typeof field === 'string' ? field : JSON.stringify(field);
			const lowerText = text.toLowerCase();
			const idx = lowerText.indexOf(lowerNeedle);
			if (idx >= 0) {
				const start = Math.max(0, idx - 40);
				const end = Math.min(text.length, idx + needle.length + 60);
				const prefix = start > 0 ? '...' : '';
				const suffix = end < text.length ? '...' : '';
				return prefix + text.slice(start, end) + suffix;
			}
		}
		return null;
	}

	function sectionLabel(result: Result, idx: number): string | null {
		if (idx === 0 && result.type === 'page') return 'Pages';
		if (result.type === 'trace' && (idx === 0 || results[idx - 1].type !== 'trace')) return 'Traces';
		if (result.type === 'dataset' && (idx === 0 || results[idx - 1].type !== 'dataset')) return 'Datasets';
		if (result.type === 'span' && (idx === 0 || results[idx - 1].type !== 'span')) return 'Spans';
		return null;
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[200] flex items-start justify-center pt-[15vh] bg-black/60"
		onclick={(e) => { if (e.target === e.currentTarget) close(); }}
		onkeydown={handleKeydown}
	>
		<div class="w-full max-w-xl bg-bg-secondary border border-border rounded-xl shadow-2xl overflow-hidden">
			<!-- Search input -->
			<div class="flex items-center gap-3 px-4 py-3 border-b border-border">
				<svg class="w-5 h-5 text-text-muted shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
					<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
				</svg>
				<input
					bind:this={inputEl}
					bind:value={query}
					type="text"
					placeholder="Search or jump to..."
					class="flex-1 bg-transparent text-sm text-text placeholder:text-text-muted/50 outline-none"
				/>
				{#if loading}
					<div class="w-4 h-4 border-2 border-accent/30 border-t-accent rounded-full animate-spin"></div>
				{/if}
				<kbd class="text-[10px] text-text-muted bg-bg-tertiary rounded px-1.5 py-0.5 border border-border">Esc</kbd>
			</div>

			<!-- Results -->
			<div class="max-h-80 overflow-y-auto">
				{#if results.length === 0 && searched}
					<div class="px-4 py-6 text-center text-xs text-text-muted">
						No results for "{query}"
					</div>
				{:else}
					{#each results as result, idx (result.type === 'page' ? result.href : result.type === 'trace' ? result.trace.id : result.type === 'dataset' ? result.dataset.id : result.span.id)}
						{@const section = sectionLabel(result, idx)}

						{#if section}
							<div class="px-4 pt-2 pb-1 text-[10px] font-semibold text-text-muted/50 uppercase tracking-widest">{section}</div>
						{/if}

						<button
							class="w-full flex items-center gap-3 px-4 py-2 text-left transition-colors
								{idx === selectedIdx ? 'bg-accent/10' : 'hover:bg-bg-tertiary/50'}"
							onclick={() => navigate(result)}
							onmouseenter={() => selectedIdx = idx}
						>
							{#if result.type === 'page'}
								<!-- Page icon -->
								<div class="w-5 h-5 flex items-center justify-center text-text-muted shrink-0">
									<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
										<path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" />
									</svg>
								</div>
								<div class="flex-1 min-w-0">
									<div class="text-xs font-medium text-text">{result.label}</div>
									<div class="text-[10px] text-text-muted truncate">{result.description}</div>
								</div>

							{:else if result.type === 'trace'}
								<div class="w-5 h-5 flex items-center justify-center text-accent shrink-0">
									<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
										<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 0 1 0 3.75H5.625a1.875 1.875 0 0 1 0-3.75Z" />
									</svg>
								</div>
								<div class="flex-1 min-w-0">
									<div class="text-xs font-medium text-text truncate">{result.trace.name || shortId(result.trace.id)}</div>
									<div class="text-[10px] text-text-muted">
										{#if result.trace.tags?.length}
											{result.trace.tags.join(', ')}
										{:else}
											Trace
										{/if}
									</div>
								</div>
								<span class="text-[10px] text-text-muted font-mono shrink-0">{shortId(result.trace.id)}</span>

							{:else if result.type === 'dataset'}
								<div class="w-5 h-5 flex items-center justify-center text-amber-400 shrink-0">
									<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
										<path stroke-linecap="round" stroke-linejoin="round" d="M3.375 19.5h17.25m-17.25 0a1.125 1.125 0 0 1-1.125-1.125M3.375 19.5h7.5c.621 0 1.125-.504 1.125-1.125m-9.75 0V5.625m0 12.75v-1.5c0-.621.504-1.125 1.125-1.125m18.375 2.625V5.625m0 12.75c0 .621-.504 1.125-1.125 1.125m1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125m0 3.75h-7.5A1.125 1.125 0 0 1 12 18.375m9.75-12.75c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125m19.5 0v1.5c0 .621-.504 1.125-1.125 1.125M2.25 5.625v1.5c0 .621.504 1.125 1.125 1.125m0 0h17.25m-17.25 0h7.5c.621 0 1.125.504 1.125 1.125M3.375 8.25c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125m17.25-3.75h-7.5c-.621 0-1.125.504-1.125 1.125m8.625-1.125c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125m-17.25 0h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125M12 10.875v-1.5m0 1.5c0 .621-.504 1.125-1.125 1.125M12 10.875c0 .621.504 1.125 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125M10.875 12c-.621 0-1.125.504-1.125 1.125M12 12c.621 0 1.125.504 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125m0 0v1.5c0 .621-.504 1.125-1.125 1.125M12 15.375c0-.621-.504-1.125-1.125-1.125" />
									</svg>
								</div>
								<div class="flex-1 min-w-0">
									<div class="text-xs font-medium text-text truncate">{result.dataset.name}</div>
									<div class="text-[10px] text-text-muted">{result.dataset.datapoint_count} datapoints</div>
								</div>

							{:else if result.type === 'span'}
								<div class="w-5 h-5 flex items-center justify-center shrink-0">
									<SpanKindIcon span={result.span} />
								</div>
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-1.5">
										<span class="text-xs font-medium text-text truncate">{result.span.name}</span>
										{#if result.span.kind?.type === 'llm_call'}
											<span class="text-[10px] text-purple-400 bg-purple-400/10 rounded px-1 py-px shrink-0">{result.span.kind.model}</span>
										{/if}
									</div>
									{#if extractSnippet(result.span, query)}
										<div class="text-[10px] text-text-muted/70 font-mono truncate">{extractSnippet(result.span, query)}</div>
									{:else}
										<div class="text-[10px] text-text-muted">Trace {shortId(result.span.trace_id)}</div>
									{/if}
								</div>
								<StatusBadge status={spanStatus(result.span)} />
								<span class="text-[10px] text-text-muted font-mono shrink-0">{formatDuration(spanDurationMs(result.span))}</span>
							{/if}
						</button>
					{/each}
				{/if}
			</div>

			<!-- Footer -->
			<div class="flex items-center gap-4 px-4 py-2 border-t border-border text-[10px] text-text-muted">
				<span><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border">↑</kbd><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border ml-0.5">↓</kbd> navigate</span>
				<span><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border">Enter</kbd> open</span>
				<span><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border">Esc</kbd> close</span>
			</div>
		</div>
	</div>
{/if}
