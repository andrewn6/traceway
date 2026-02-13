<script lang="ts">
	import { getStats, getSpans, subscribeEvents, type Span, type SpanEvent } from '$lib/api';
	import { spanStatus, spanStartedAt, spanDurationMs, shortId } from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { onMount } from 'svelte';

	let traceCount = $state(0);
	let spanCount = $state(0);
	let recentSpans: Span[] = $state([]);
	let activity: { time: string; text: string }[] = $state([]);
	let models: Map<string, number> = $state(new Map());
	let errorCount = $state(0);
	let totalDuration = $state(0);
	let completedCount = $state(0);

	function addActivity(text: string) {
		const time = new Date().toLocaleTimeString();
		activity = [{ time, text }, ...activity.slice(0, 49)];
	}

	async function loadData() {
		try {
			const stats = await getStats();
			traceCount = stats.trace_count;
			spanCount = stats.span_count;

			const { spans } = await getSpans();
			recentSpans = spans
				.sort((a, b) => new Date(spanStartedAt(b)).getTime() - new Date(spanStartedAt(a)).getTime())
				.slice(0, 20);

			const m = new Map<string, number>();
			let errors = 0;
			let dur = 0;
			let completed = 0;
			for (const s of spans) {
				const modelName = s.kind?.type === 'llm_call' ? s.kind.model : s.metadata.model;
				if (modelName) {
					m.set(modelName, (m.get(modelName) ?? 0) + 1);
				}
				if (spanStatus(s) === 'failed') errors++;
				const d = spanDurationMs(s);
				if (d !== null) {
					dur += d;
					completed++;
				}
			}
			models = m;
			errorCount = errors;
			totalDuration = dur;
			completedCount = completed;
		} catch {
			// API not available
		}
	}

	onMount(() => {
		loadData();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'span_created') {
				spanCount++;
				addActivity(`Span created: ${event.span.name}`);
				recentSpans = [event.span, ...recentSpans.slice(0, 19)];
				const evtModel = event.span.kind?.type === 'llm_call' ? event.span.kind.model : event.span.metadata.model;
				if (evtModel) {
					models.set(evtModel, (models.get(evtModel) ?? 0) + 1);
					models = new Map(models);
				}
			} else if (event.type === 'span_updated') {
				addActivity(`Span ${spanStatus(event.span)}: ${event.span.name}`);
				recentSpans = recentSpans.map((s) => (s.id === event.span.id ? event.span : s));
			} else if (event.type === 'span_deleted') {
				addActivity(`Span deleted: ${shortId(event.span_id)}`);
				spanCount = Math.max(0, spanCount - 1);
			} else if (event.type === 'trace_deleted') {
				addActivity(`Trace deleted: ${shortId(event.trace_id)}`);
				loadData();
			} else if (event.type === 'cleared') {
				addActivity('All traces cleared');
				traceCount = 0;
				spanCount = 0;
				recentSpans = [];
				models = new Map();
			}
		});

		return unsub;
	});

	const avgLatency = $derived(completedCount > 0 ? Math.round(totalDuration / completedCount) : 0);
	const errorRate = $derived(spanCount > 0 ? ((errorCount / spanCount) * 100).toFixed(1) : '0.0');
</script>

<div class="max-w-6xl mx-auto space-y-6">
	<h1 class="text-xl font-bold">Dashboard</h1>

	<!-- Stats cards -->
	<div class="grid grid-cols-2 md:grid-cols-4 gap-4">
		<div class="bg-bg-secondary border border-border rounded p-4">
			<div class="text-text-muted text-xs uppercase">Traces</div>
			<div class="text-2xl font-bold text-text mt-1">{traceCount}</div>
		</div>
		<div class="bg-bg-secondary border border-border rounded p-4">
			<div class="text-text-muted text-xs uppercase">Spans</div>
			<div class="text-2xl font-bold text-text mt-1">{spanCount}</div>
		</div>
		<div class="bg-bg-secondary border border-border rounded p-4">
			<div class="text-text-muted text-xs uppercase">Avg Latency</div>
			<div class="text-2xl font-bold text-text mt-1">{avgLatency}ms</div>
		</div>
		<div class="bg-bg-secondary border border-border rounded p-4">
			<div class="text-text-muted text-xs uppercase">Error Rate</div>
			<div class="text-2xl font-bold {parseFloat(errorRate) > 0 ? 'text-danger' : 'text-success'} mt-1">
				{errorRate}%
			</div>
		</div>
	</div>

	<!-- Models -->
	{#if models.size > 0}
		<div class="bg-bg-secondary border border-border rounded p-4">
			<div class="text-text-muted text-xs uppercase mb-2">Models</div>
			<div class="flex flex-wrap gap-2">
				{#each [...models.entries()] as [model, count]}
					<span class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs">
						<span class="text-accent">{model}</span>
						<span class="text-text-muted ml-1">{count}</span>
					</span>
				{/each}
			</div>
		</div>
	{/if}

	{#if traceCount === 0 && spanCount === 0}
		<!-- Getting started -->
		<div class="bg-bg-secondary border border-border rounded-lg p-8 space-y-6">
			<div class="text-center space-y-2">
				<div class="flex items-center justify-center gap-2 text-text-secondary">
					<span class="w-2 h-2 rounded-full bg-success animate-pulse"></span>
					<span class="text-sm">Listening on localhost:3000</span>
				</div>
				<p class="text-text-muted text-xs">Instrument your LLM application to start collecting traces.</p>
			</div>

			<div class="space-y-4 max-w-2xl mx-auto">
				<details class="group" open>
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors font-semibold">
						Quick test with curl
					</summary>
					<pre class="mt-2 bg-bg-tertiary rounded p-3 text-xs text-text-secondary font-mono overflow-x-auto whitespace-pre"># Create a span (starts a trace automatically)
curl -s http://localhost:3000/spans -X POST \
  -H 'Content-Type: application/json' \
  -d '{`{"trace_id":"00000000-0000-0000-0000-000000000001","name":"hello-world"}`}'</pre>
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

				<details class="group">
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors font-semibold">
						TypeScript SDK
					</summary>
					<pre class="mt-2 bg-bg-tertiary rounded p-3 text-xs text-text-secondary font-mono overflow-x-auto whitespace-pre">npm install llmtrace

import {`{ TraceContext }`} from "llmtrace";

const ctx = new TraceContext();
const span = ctx.span("my-task");
// ... your code ...
await span.complete({`{ result: "done" }`});</pre>
				</details>
			</div>
		</div>
	{:else}
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
							{#if span.kind?.type === 'llm_call'}
								<span class="text-text-muted">{span.kind.model}</span>
							{:else if span.metadata.model}
								<span class="text-text-muted">{span.metadata.model}</span>
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
