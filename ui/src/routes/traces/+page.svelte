<script lang="ts">
	import { getTraces, getTrace, subscribeEvents, type Span, type SpanEvent } from '$lib/api';
	import { spanStatus, spanStartedAt } from '$lib/api';
	import TraceRow from '$lib/components/TraceRow.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { onMount } from 'svelte';

	let traceSpans: Map<string, Span[]> = $state(new Map());
	let traceIds: string[] = $state([]);
	let filterModel = $state('');
	let filterStatus = $state('');
	let loading = $state(true);

	async function loadTraces() {
		try {
			const { traces } = await getTraces();
			const spanMap = new Map<string, Span[]>();
			for (const id of traces) {
				const { spans } = await getTrace(id);
				spanMap.set(id, spans);
			}
			traceSpans = spanMap;
			traceIds = traces;
		} catch {
			// API not available
		}
		loading = false;
	}

	onMount(() => {
		loadTraces();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'span_created') {
				const tid = event.span.trace_id;
				const existing = traceSpans.get(tid) ?? [];
				traceSpans.set(tid, [...existing, event.span]);
				traceSpans = new Map(traceSpans);
				if (!traceIds.includes(tid)) {
					traceIds = [tid, ...traceIds];
				}
			} else if (event.type === 'span_updated') {
				const tid = event.span.trace_id;
				const existing = traceSpans.get(tid);
				if (existing) {
					traceSpans.set(
						tid,
						existing.map((s) => (s.id === event.span.id ? event.span : s))
					);
					traceSpans = new Map(traceSpans);
				}
			} else if (event.type === 'trace_deleted') {
				traceSpans.delete(event.trace_id);
				traceSpans = new Map(traceSpans);
				traceIds = traceIds.filter((id) => id !== event.trace_id);
			} else if (event.type === 'cleared') {
				traceSpans = new Map();
				traceIds = [];
			}
		});

		return unsub;
	});

	const filtered = $derived.by(() => {
		return traceIds.filter((id) => {
			const spans = traceSpans.get(id) ?? [];
			if (filterModel) {
				if (!spans.some((s) => s.metadata.model?.includes(filterModel))) return false;
			}
			if (filterStatus) {
				const traceStatus = spans.some((s) => spanStatus(s) === 'failed')
					? 'failed'
					: spans.some((s) => spanStatus(s) === 'running')
						? 'running'
						: 'completed';
				if (traceStatus !== filterStatus) return false;
			}
			return true;
		});
	});
</script>

<div class="max-w-6xl mx-auto space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold">Traces</h1>
		<div class="flex items-center gap-2 text-sm">
			<input
				type="text"
				placeholder="Filter model..."
				bind:value={filterModel}
				class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs text-text placeholder:text-text-muted w-36"
			/>
			<select
				bind:value={filterStatus}
				class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs text-text"
			>
				<option value="">All statuses</option>
				<option value="running">Running</option>
				<option value="completed">Completed</option>
				<option value="failed">Failed</option>
			</select>
		</div>
	</div>

	<!-- Table header -->
	<div class="grid grid-cols-[1fr_80px_100px_140px_100px_100px] gap-4 px-3 text-xs text-text-muted uppercase">
		<span>Trace ID</span>
		<span class="text-center">Spans</span>
		<span class="text-center">Status</span>
		<span>Started</span>
		<span class="text-right">Duration</span>
		<span>Model</span>
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading...</div>
	{:else if traceIds.length === 0}
		<!-- Empty state: waiting for traces -->
		<div class="bg-bg-secondary border border-border rounded-lg p-8 space-y-6">
			<div class="text-center space-y-2">
				<div class="flex items-center justify-center gap-2 text-text-secondary">
					<span class="w-2 h-2 rounded-full bg-success animate-pulse"></span>
					<span class="text-sm">Listening on localhost:3000</span>
				</div>
				<p class="text-text-muted text-xs">Send traces from your code to see them here in real time.</p>
			</div>

			<div class="space-y-4 max-w-2xl mx-auto">
				<!-- curl -->
				<details class="group">
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors">
						Quick test with curl
					</summary>
					<pre class="mt-2 bg-bg-tertiary rounded p-3 text-xs text-text-secondary font-mono overflow-x-auto whitespace-pre"># 1. Create a span (starts a trace automatically)
curl -s http://localhost:3000/spans -X POST \
  -H 'Content-Type: application/json' \
  -d '{`{"trace_id":"00000000-0000-0000-0000-000000000001","name":"my-first-span"}`}'</pre>
				</details>

				<!-- Python -->
				<details class="group">
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors">
						Python SDK
					</summary>
					<pre class="mt-2 bg-bg-tertiary rounded p-3 text-xs text-text-secondary font-mono overflow-x-auto whitespace-pre">pip install llmtrace

from llmtrace import TraceContext

ctx = TraceContext()
with ctx.span("my-task") as s:
    # ... your code ...
    s.complete({`{"result": "done"}`})</pre>
				</details>

				<!-- TypeScript -->
				<details class="group">
					<summary class="text-xs text-text-secondary cursor-pointer hover:text-text transition-colors">
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
	{:else if filtered.length === 0}
		<div class="text-text-muted text-sm text-center py-8">No traces match filters</div>
	{:else}
		<div class="space-y-0">
			{#each filtered as traceId (traceId)}
				<TraceRow {traceId} spans={traceSpans.get(traceId) ?? []} />
			{/each}
		</div>
	{/if}
</div>
