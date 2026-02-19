<script lang="ts">
	import { getAnalyticsSummary, type AnalyticsSummary } from '$lib/api';
	import { onMount } from 'svelte';

	let summary: AnalyticsSummary | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	async function loadData() {
		try {
			summary = await getAnalyticsSummary();
			error = '';
		} catch (e) {
			error = 'Could not load analytics. Is the daemon running?';
		}
		loading = false;
	}

	onMount(() => {
		loadData();
		const interval = setInterval(loadData, 10000);
		return () => clearInterval(interval);
	});
</script>

<div class="max-w-5xl space-y-6">
	<h1 class="text-xl font-bold">Analytics</h1>

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading analytics...</div>
	{:else if error}
		<div class="bg-bg-secondary border border-border rounded p-6 text-center">
			<p class="text-text-muted text-sm">{error}</p>
		</div>
	{:else if !summary || summary.total_spans === 0}
		<div class="bg-bg-secondary border border-border rounded p-6 text-center">
			<p class="text-text-muted text-sm">No data yet. Create some traces to see analytics.</p>
		</div>
	{:else}
		<!-- Overview cards -->
		<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">Total Traces</div>
				<div class="text-2xl font-bold text-text mt-1">{summary.total_traces}</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">Total Spans</div>
				<div class="text-2xl font-bold text-text mt-1">{summary.total_spans}</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">LLM Calls</div>
				<div class="text-2xl font-bold text-text mt-1">{summary.total_llm_calls}</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">Errors</div>
				<div class="text-2xl font-bold {summary.error_count > 0 ? 'text-danger' : 'text-success'} mt-1">
					{summary.error_count}
				</div>
			</div>
		</div>

		<!-- Cost & Tokens -->
		{#if summary.total_llm_calls > 0}
			<div class="grid grid-cols-3 gap-4">
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase">Total Cost</div>
					<div class="text-xl font-bold text-text mt-1">${summary.total_cost.toFixed(4)}</div>
				</div>
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase">Total Tokens</div>
					<div class="text-xl font-bold text-text mt-1">{summary.total_tokens.toLocaleString()}</div>
				</div>
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase">Avg Latency</div>
					<div class="text-xl font-bold text-text mt-1">{Math.round(summary.avg_latency_ms)}ms</div>
				</div>
			</div>
		{/if}

		<!-- Cost by Model -->
		{#if summary.cost_by_model.length > 0}
			<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
				<h2 class="text-text-muted text-xs uppercase">Cost by Model</h2>
				<div class="space-y-2">
					{#each summary.cost_by_model as item}
						{@const maxCost = Math.max(...summary.cost_by_model.map(c => c.cost))}
						<div class="flex items-center gap-3 text-sm">
							<span class="text-text w-48 truncate font-mono text-xs">{item.model}</span>
							<div class="flex-1 bg-bg-tertiary rounded-full h-3 overflow-hidden">
								{#if maxCost > 0}
									<div class="h-full bg-accent/60 rounded-full" style="width: {(item.cost / maxCost * 100)}%"></div>
								{/if}
							</div>
							<span class="text-text text-xs w-20 text-right font-mono">${item.cost.toFixed(4)}</span>
							<span class="text-text-muted text-xs w-16 text-right">{item.span_count} calls</span>
						</div>
					{/each}
				</div>
			</section>
		{/if}

		<!-- Tokens by Model -->
		{#if summary.tokens_by_model.length > 0}
			<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
				<h2 class="text-text-muted text-xs uppercase">Tokens by Model</h2>
				<div class="space-y-3">
					{#each summary.tokens_by_model as item}
						{@const maxTokens = Math.max(...summary.tokens_by_model.map(t => t.total_tokens))}
						<div class="space-y-1">
							<div class="flex items-center justify-between text-xs">
								<span class="text-text font-mono">{item.model}</span>
								<span class="text-text-muted">{item.total_tokens.toLocaleString()} total</span>
							</div>
							<div class="flex h-3 rounded-full overflow-hidden bg-bg-tertiary">
								{#if maxTokens > 0}
									<div
										class="bg-blue-400/60 h-full"
										style="width: {(item.input_tokens / maxTokens * 100)}%"
										title="Input: {item.input_tokens.toLocaleString()}"
									></div>
									<div
										class="bg-purple-400/60 h-full"
										style="width: {(item.output_tokens / maxTokens * 100)}%"
										title="Output: {item.output_tokens.toLocaleString()}"
									></div>
								{/if}
							</div>
							<div class="flex items-center gap-4 text-xs text-text-muted">
								<span class="flex items-center gap-1">
									<span class="w-2 h-2 rounded-sm bg-blue-400/60"></span>
									Input: {item.input_tokens.toLocaleString()}
								</span>
								<span class="flex items-center gap-1">
									<span class="w-2 h-2 rounded-sm bg-purple-400/60"></span>
									Output: {item.output_tokens.toLocaleString()}
								</span>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/if}

		<!-- Providers -->
		{#if summary.providers_used.length > 0}
			<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
				<h2 class="text-text-muted text-xs uppercase">Providers</h2>
				<div class="flex flex-wrap gap-2">
					{#each summary.providers_used as provider}
						<span class="bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text">
							{provider}
						</span>
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</div>
