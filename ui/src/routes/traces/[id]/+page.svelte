<script lang="ts">
	import { page } from '$app/state';
	import { getTrace, subscribeEvents, type Span } from '$lib/api';
	import { shortId, spanStatus, spanDurationMs } from '$lib/api';
	import TraceTimeline from '$lib/components/TraceTimeline.svelte';
	import SpanDetail from '$lib/components/SpanDetail.svelte';
	import { onMount } from 'svelte';

	const traceId = $derived(page.params.id ?? '');
	let spans: Span[] = $state([]);
	let selectedSpan: Span | null = $state(null);
	let loading = $state(true);

	async function loadTrace(id: string) {
		try {
			const result = await getTrace(id);
			spans = result.spans;
			if (selectedSpan) {
				const updated = spans.find((s) => s.id === selectedSpan!.id);
				if (updated) selectedSpan = updated;
			}
		} catch {
			// not found
		}
		loading = false;
	}

	onMount(() => {
		loadTrace(traceId);

		const unsub = subscribeEvents((event) => {
			if (event.type === 'span_created' && event.span.trace_id === traceId) {
				spans = [...spans, event.span];
			} else if (event.type === 'span_updated' && event.span.trace_id === traceId) {
				spans = spans.map((s) => (s.id === event.span.id ? event.span : s));
				if (selectedSpan?.id === event.span.id) {
					selectedSpan = event.span;
				}
			} else if (event.type === 'span_deleted') {
				spans = spans.filter((s) => s.id !== event.span_id);
				if (selectedSpan?.id === event.span_id) selectedSpan = null;
			} else if (event.type === 'trace_deleted' && event.trace_id === traceId) {
				spans = [];
				selectedSpan = null;
			}
		});

		return unsub;
	});

	function selectSpan(span: Span) {
		selectedSpan = span;
	}

	const filesReadCount = $derived(spans.filter((s) => s.kind && 'FsRead' in s.kind).length);
	const filesWrittenCount = $derived(spans.filter((s) => s.kind && 'FsWrite' in s.kind).length);

	const traceStatus = $derived.by(() => {
		if (spans.some((s) => spanStatus(s) === 'failed')) return 'failed';
		if (spans.some((s) => spanStatus(s) === 'running')) return 'running';
		return 'completed';
	});

	const totalDuration = $derived.by(() => {
		const durations = spans.map(spanDurationMs).filter((d): d is number => d !== null);
		if (durations.length === 0) return null;
		return Math.max(...durations);
	});
</script>

<div class="h-[calc(100vh-5rem)] flex flex-col">
	<!-- Header -->
	<div class="flex items-center gap-3 px-4 py-2 shrink-0">
		<a href="/traces" class="text-text-secondary hover:text-text text-sm">&larr; Traces</a>
		<span class="text-text-muted">/</span>
		<h1 class="text-lg font-bold font-mono">{shortId(traceId)}</h1>

		{#if !loading && spans.length > 0}
			<span class="px-2 py-0.5 rounded text-xs border
				{traceStatus === 'completed' ? 'bg-success/20 text-success border-success/30' :
				 traceStatus === 'running' ? 'bg-warning/20 text-warning border-warning/30' :
				 'bg-danger/20 text-danger border-danger/30'}">
				{traceStatus}
			</span>
			<span class="text-text-secondary text-xs">{spans.length} spans</span>
			{#if totalDuration !== null}
				<span class="text-text-secondary text-xs font-mono">{totalDuration}ms</span>
			{/if}
			{#if filesReadCount > 0 || filesWrittenCount > 0}
				<span class="text-text-muted text-xs">
					{#if filesReadCount > 0}{filesReadCount} read{/if}{#if filesReadCount > 0 && filesWrittenCount > 0} &middot; {/if}{#if filesWrittenCount > 0}{filesWrittenCount} written{/if}
				</span>
			{/if}
		{/if}
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8 flex-1">Loading...</div>
	{:else if spans.length === 0}
		<div class="text-text-muted text-sm text-center py-8 flex-1">Trace not found</div>
	{:else}
		<!-- Timeline -->
		<div class="flex-1 min-h-0 border border-border rounded mx-4 bg-bg-secondary overflow-hidden">
			<TraceTimeline
				{spans}
				selectedId={selectedSpan?.id ?? null}
				onSelect={selectSpan}
			/>
		</div>

		<!-- Span detail panel -->
		{#if selectedSpan}
			<div class="shrink-0 max-h-64 overflow-y-auto mx-4 mt-2 mb-2">
				<SpanDetail span={selectedSpan} />
			</div>
		{/if}
	{/if}
</div>
