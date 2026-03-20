<script lang="ts">
	import { getAnalyticsSummary, getSpans, subscribeEvents, type Span, type AnalyticsSummary } from '$lib/api';
	import { spanStatus, spanStartedAt, spanModel, shortId } from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { onMount } from 'svelte';

	let summary: AnalyticsSummary | null = $state(null);
	let recentSpans: Span[] = $state([]);
	let activity: { time: string; text: string }[] = $state([]);
	let loaded = $state(false);

	const s = $derived(summary ?? {
		total_traces: 0,
		total_spans: 0,
		avg_latency_ms: 0,
		total_llm_calls: 0,
		total_tokens: 0,
		total_cost: 0,
		error_count: 0,
		models_used: [],
		providers_used: [],
		cost_by_model: [],
		tokens_by_model: [],
	} as AnalyticsSummary);

	const errorRate = $derived.by(() => {
		if (s.total_spans > 0) {
			return ((s.error_count / s.total_spans) * 100).toFixed(1);
		}
		return '0.0';
	});

	function addActivity(text: string) {
		const time = new Date().toLocaleTimeString();
		activity = [{ time, text }, ...activity.slice(0, 49)];
	}

	async function loadData() {
		try {
			const [summaryRes, spanRes] = await Promise.all([
				getAnalyticsSummary().catch(() => null),
				getSpans()
			]);
			summary = summaryRes;
			recentSpans = spanRes.items
				.sort((a: Span, b: Span) => new Date(spanStartedAt(b)).getTime() - new Date(spanStartedAt(a)).getTime())
				.slice(0, 20);
		} catch {
			// API not available
		}
		loaded = true;
	}

	onMount(() => {
		loadData();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'span_created') {
				addActivity(`Span created: ${event.span.name}`);
				recentSpans = [event.span, ...recentSpans.slice(0, 19)];
			} else if (event.type === 'span_completed') {
				addActivity(`Span completed: ${event.span.name}`);
				recentSpans = recentSpans.map((sp) => (sp.id === event.span.id ? event.span : sp));
			} else if (event.type === 'span_failed') {
				addActivity(`Span failed: ${event.span.name}`);
				recentSpans = recentSpans.map((sp) => (sp.id === event.span.id ? event.span : sp));
			} else if (event.type === 'span_deleted') {
				addActivity(`Span deleted: ${shortId(event.span_id)}`);
			} else if (event.type === 'trace_deleted') {
				addActivity(`Trace deleted: ${shortId(event.trace_id)}`);
				loadData();
			} else if (event.type === 'cleared') {
				addActivity('All traces cleared');
				summary = null;
				recentSpans = [];
			}
		});

		const interval = setInterval(() => {
			getAnalyticsSummary().then((res) => (summary = res)).catch(() => {});
		}, 10000);

		return () => {
			unsub();
			clearInterval(interval);
		};
	});
</script>

<div class="space-y-5">
	<h1 class="text-xl font-semibold tracking-tight">Dashboard</h1>

	{#if !loaded}
		<div class="text-text-muted text-sm py-10 text-center">Loading...</div>
	{:else}
		<!-- Stats cards — always visible -->
		<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase">Traces</div>
				<div class="text-2xl font-bold text-text mt-1">{s.total_traces}</div>
			</div>
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase">Spans</div>
				<div class="text-2xl font-bold text-text mt-1">{s.total_spans}</div>
			</div>
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase">Avg Latency</div>
				<div class="text-2xl font-bold text-text mt-1">{Math.round(s.avg_latency_ms)}ms</div>
			</div>
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase">Error Rate</div>
				<div class="text-2xl font-bold {parseFloat(errorRate) > 0 ? 'text-danger' : 'text-success'} mt-1">
					{errorRate}%
				</div>
			</div>
		</div>

		<!-- LLM metrics row — always visible -->
		<div class="grid grid-cols-3 gap-3">
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase">LLM Calls</div>
				<div class="text-xl font-bold text-text mt-1">{s.total_llm_calls}</div>
			</div>
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase">Total Tokens</div>
				<div class="text-xl font-bold text-text mt-1">{s.total_tokens.toLocaleString()}</div>
			</div>
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase">Total Cost</div>
				<div class="text-xl font-bold text-text mt-1">${s.total_cost.toFixed(4)}</div>
			</div>
		</div>

		<!-- Models -->
		<div class="table-float p-4">
			<div class="text-text-muted text-xs uppercase mb-2">Models</div>
			{#if s.cost_by_model.length > 0}
				<div class="flex flex-wrap gap-2">
					{#each s.cost_by_model as item}
						<span class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs">
							<span class="text-accent">{item.model}</span>
							<span class="text-text-muted ml-1">{item.span_count} calls</span>
							{#if item.cost > 0}
								<span class="text-text-muted ml-1">${item.cost.toFixed(4)}</span>
							{/if}
						</span>
					{/each}
				</div>
			{:else}
				<div class="text-text-muted text-xs">No models used yet</div>
			{/if}
		</div>

		<!-- Tokens by model -->
		<div class="table-float p-4">
			<div class="text-text-muted text-xs uppercase mb-3">Tokens by Model</div>
			{#if s.tokens_by_model.length > 0}
				<div class="space-y-2">
					{#each s.tokens_by_model as item}
						{@const maxTokens = Math.max(...s.tokens_by_model.map((t: any) => t.total_tokens))}
						<div class="flex items-center gap-3 text-sm">
							<span class="text-text w-40 truncate font-mono text-xs">{item.model}</span>
							<div class="flex-1 bg-bg-tertiary rounded-full h-2 overflow-hidden">
								<div class="h-full bg-purple-400/60 rounded-full" style="width: {maxTokens > 0 ? (item.total_tokens / maxTokens * 100) : 0}%"></div>
							</div>
							<span class="text-text-muted text-xs w-24 text-right font-mono">{item.total_tokens.toLocaleString()}</span>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-text-muted text-xs">No token data yet</div>
			{/if}
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
			<!-- Recent spans -->
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase mb-3">Recent Spans</div>
				<div class="space-y-1 max-h-80 overflow-y-auto">
					{#each recentSpans as span (span.id)}
						<div class="flex items-center gap-2 text-xs py-1">
							<StatusBadge status={spanStatus(span)} />
							<a href="/traces/{span.trace_id}" class="text-accent hover:underline font-mono">
								{shortId(span.trace_id)}
							</a>
							<span class="text-text truncate flex-1">{span.name}</span>
							{#if spanModel(span)}
								<span class="text-text-muted">{spanModel(span)}</span>
							{/if}
						</div>
					{:else}
						<div class="text-text-muted text-xs">No spans yet</div>
					{/each}
				</div>
			</div>

			<!-- Activity feed -->
			<div class="table-float p-4">
				<div class="text-text-muted text-xs uppercase mb-3">Activity Feed</div>
				<div class="space-y-1 max-h-80 overflow-y-auto">
					{#each activity as item}
						<div class="text-xs py-0.5">
							<span class="text-text-muted font-mono">{item.time}</span>
							<span class="text-text ml-2">{item.text}</span>
						</div>
					{:else}
						<div class="text-text-muted text-xs">Waiting for events...</div>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>
