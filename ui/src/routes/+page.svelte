<script lang="ts">
	import { getAnalyticsSummary, getSpans, subscribeEvents, type Span, type AnalyticsSummary } from '$lib/api';
	import { spanStatus, spanStartedAt, spanModel, shortId } from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { onMount } from 'svelte';

	let summary: AnalyticsSummary | null = $state(null);
	let recentSpans: Span[] = $state([]);
	let activity: { time: string; text: string }[] = $state([]);
	let loaded = $state(false);

	const errorRate = $derived(
		summary && summary.total_spans > 0
			? ((summary.error_count / summary.total_spans) * 100).toFixed(1)
			: '0.0'
	);

	function addActivity(text: string) {
		const time = new Date().toLocaleTimeString();
		activity = [{ time, text }, ...activity.slice(0, 49)];
	}

	async function loadData() {
		try {
			const [s, spanRes] = await Promise.all([
				getAnalyticsSummary().catch(() => null),
				getSpans()
			]);
			summary = s;
			recentSpans = spanRes.spans
				.sort((a, b) => new Date(spanStartedAt(b)).getTime() - new Date(spanStartedAt(a)).getTime())
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
				recentSpans = recentSpans.map((s) => (s.id === event.span.id ? event.span : s));
			} else if (event.type === 'span_failed') {
				addActivity(`Span failed: ${event.span.name}`);
				recentSpans = recentSpans.map((s) => (s.id === event.span.id ? event.span : s));
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

		// Refresh summary periodically
		const interval = setInterval(() => {
			getAnalyticsSummary().then((s) => (summary = s)).catch(() => {});
		}, 10000);

		return () => {
			unsub();
			clearInterval(interval);
		};
	});
</script>

<div class="max-w-6xl mx-auto space-y-6">
	<h1 class="text-xl font-bold">Dashboard</h1>

	{#if !loaded}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if !summary || (summary.total_traces === 0 && summary.total_spans === 0)}
		<!-- Getting started -->
		<div class="bg-bg-secondary border border-border rounded-lg p-8 space-y-6">
			<div class="text-center space-y-2">
				<div class="flex items-center justify-center gap-2 text-text-secondary">
					<span class="w-2 h-2 rounded-full bg-success animate-pulse"></span>
					<span class="text-sm">Daemon connected</span>
				</div>
				<p class="text-text-muted text-xs">Instrument your LLM application to start collecting traces.</p>
			</div>

			<div class="space-y-4 max-w-2xl mx-auto">
				<details class="group" open>
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors font-semibold">
						Quick test with curl
					</summary>
					<pre class="mt-2 bg-bg-tertiary rounded p-3 text-xs text-text-secondary font-mono overflow-x-auto whitespace-pre"># Create a trace first
TRACE=$(curl -s localhost:3000/traces -X POST \
  -H 'Content-Type: application/json' \
  -d '{`{"name":"test-trace","tags":["manual"]}`}' | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")

# Create a span in that trace
curl -s localhost:3000/spans -X POST \
  -H 'Content-Type: application/json' \
  -d "{`{\"trace_id\":\"$TRACE\",\"name\":\"hello-world\",\"kind\":{\"type\":\"llm_call\",\"model\":\"gpt-4o\"}}`}"</pre>
				</details>

				<details class="group">
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors font-semibold">
						Dev ingest mode (synthetic data)
					</summary>
					<pre class="mt-2 bg-bg-tertiary rounded p-3 text-xs text-text-secondary font-mono overflow-x-auto whitespace-pre">cargo run -p daemon -- --foreground --dev-ingest --dev-ingest-interval 3</pre>
				</details>

				<details class="group">
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors font-semibold">
						Python SDK
					</summary>
					<pre class="mt-2 bg-bg-tertiary rounded p-3 text-xs text-text-secondary font-mono overflow-x-auto whitespace-pre">pip install llmtrace

from llmtrace import TraceContext

ctx = TraceContext()
with ctx.span("my-task") as s:
    # ... your code ...
    s.complete({`{"result": "done"}`})</pre>
				</details>
			</div>
		</div>
	{:else}
		<!-- Stats cards -->
		<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">Traces</div>
				<div class="text-2xl font-bold text-text mt-1">{summary.total_traces}</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">Spans</div>
				<div class="text-2xl font-bold text-text mt-1">{summary.total_spans}</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">Avg Latency</div>
				<div class="text-2xl font-bold text-text mt-1">{Math.round(summary.avg_latency_ms)}ms</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase">Error Rate</div>
				<div class="text-2xl font-bold {parseFloat(errorRate) > 0 ? 'text-danger' : 'text-success'} mt-1">
					{errorRate}%
				</div>
			</div>
		</div>

		<!-- LLM metrics row -->
		{#if summary.total_llm_calls > 0}
			<div class="grid grid-cols-3 gap-4">
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase">LLM Calls</div>
					<div class="text-xl font-bold text-text mt-1">{summary.total_llm_calls}</div>
				</div>
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase">Total Tokens</div>
					<div class="text-xl font-bold text-text mt-1">{summary.total_tokens.toLocaleString()}</div>
				</div>
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase">Total Cost</div>
					<div class="text-xl font-bold text-text mt-1">${summary.total_cost.toFixed(4)}</div>
				</div>
			</div>
		{/if}

		<!-- Models & Providers -->
		{#if summary.models_used.length > 0}
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase mb-2">Models</div>
				<div class="flex flex-wrap gap-2">
					{#each summary.cost_by_model as item}
						<span class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs">
							<span class="text-accent">{item.model}</span>
							<span class="text-text-muted ml-1">{item.span_count} calls</span>
							{#if item.cost > 0}
								<span class="text-text-muted ml-1">${item.cost.toFixed(4)}</span>
							{/if}
						</span>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Token breakdown by model -->
		{#if summary.tokens_by_model.length > 0}
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase mb-3">Tokens by Model</div>
				<div class="space-y-2">
					{#each summary.tokens_by_model as item}
						{@const maxTokens = Math.max(...summary.tokens_by_model.map(t => t.total_tokens))}
						<div class="flex items-center gap-3 text-sm">
							<span class="text-text w-40 truncate font-mono text-xs">{item.model}</span>
							<div class="flex-1 bg-bg-tertiary rounded-full h-2 overflow-hidden">
								<div class="h-full bg-purple-400/60 rounded-full" style="width: {maxTokens > 0 ? (item.total_tokens / maxTokens * 100) : 0}%"></div>
							</div>
							<span class="text-text-muted text-xs w-24 text-right font-mono">{item.total_tokens.toLocaleString()}</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
			<!-- Recent spans -->
			<div class="bg-bg-secondary border border-border rounded p-4">
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
						<div class="text-text-muted text-sm">No spans yet</div>
					{/each}
				</div>
			</div>

			<!-- Activity feed -->
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase mb-3">Activity Feed</div>
				<div class="space-y-1 max-h-80 overflow-y-auto">
					{#each activity as item}
						<div class="text-xs py-0.5">
							<span class="text-text-muted font-mono">{item.time}</span>
							<span class="text-text ml-2">{item.text}</span>
						</div>
					{:else}
						<div class="text-text-muted text-sm">Waiting for events...</div>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>
