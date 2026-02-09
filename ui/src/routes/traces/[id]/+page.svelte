<script lang="ts">
	import { page } from '$app/state';
	import { getTrace, subscribeEvents, type Span } from '$lib/api';
	import { shortId, spanStatus, spanDurationMs, spanKindLabel } from '$lib/api';
	import SpanTree from '$lib/components/SpanTree.svelte';
	import Waterfall from '$lib/components/Waterfall.svelte';
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

	// Extract files read/written from SpanKind
	const filesRead = $derived.by(() => {
		const files: { path: string; version?: string; spanName: string }[] = [];
		for (const s of spans) {
			if (s.kind && 'FsRead' in s.kind) {
				files.push({ path: s.kind.FsRead.path, version: s.kind.FsRead.file_version ?? undefined, spanName: s.name });
			}
		}
		return files;
	});

	const filesWritten = $derived.by(() => {
		const files: { path: string; version: string; spanName: string }[] = [];
		for (const s of spans) {
			if (s.kind && 'FsWrite' in s.kind) {
				files.push({ path: s.kind.FsWrite.path, version: s.kind.FsWrite.file_version, spanName: s.name });
			}
		}
		return files;
	});

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

<div class="max-w-7xl space-y-4">
	<!-- Header -->
	<div class="flex items-center gap-2">
		<a href="/traces" class="text-text-secondary hover:text-text text-sm">&larr; Traces</a>
		<span class="text-text-muted">/</span>
		<h1 class="text-lg font-bold font-mono">{shortId(traceId)}</h1>
		<span class="text-text-muted text-xs font-mono">{traceId}</span>
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading...</div>
	{:else if spans.length === 0}
		<div class="text-text-muted text-sm text-center py-8">Trace not found</div>
	{:else}
		<!-- Summary bar -->
		<div class="flex items-center gap-4 text-sm">
			<span class="px-2 py-0.5 rounded text-xs border
				{traceStatus === 'completed' ? 'bg-success/20 text-success border-success/30' :
				 traceStatus === 'running' ? 'bg-warning/20 text-warning border-warning/30' :
				 'bg-danger/20 text-danger border-danger/30'}">
				{traceStatus}
			</span>
			<span class="text-text-secondary">{spans.length} spans</span>
			{#if totalDuration !== null}
				<span class="text-text-secondary font-mono">{totalDuration}ms</span>
			{/if}
		</div>

		<!-- Context summary: files in → llm → files out -->
		{#if filesRead.length > 0 || filesWritten.length > 0}
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#if filesRead.length > 0}
					<div class="bg-bg-secondary border border-border rounded p-3">
						<div class="text-text-muted text-xs uppercase mb-2 flex items-center gap-1.5">
							<svg class="w-3.5 h-3.5 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m.75 12 3 3m0 0 3-3m-3 3v-6m-1.5-9H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
							</svg>
							Context In ({filesRead.length})
						</div>
						<div class="space-y-1">
							{#each filesRead as file}
								<a href="/files/{file.path}" class="flex items-center gap-2 text-xs hover:bg-bg-tertiary rounded px-1.5 py-1 transition-colors">
									<span class="text-accent font-mono">{file.path}</span>
									{#if file.version}
										<span class="text-text-muted">{file.version.slice(0, 8)}</span>
									{/if}
								</a>
							{/each}
						</div>
					</div>
				{/if}
				{#if filesWritten.length > 0}
					<div class="bg-bg-secondary border border-border rounded p-3">
						<div class="text-text-muted text-xs uppercase mb-2 flex items-center gap-1.5">
							<svg class="w-3.5 h-3.5 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m.75 12 3-3m0 0 3 3m-3-3v6m-1.5-15H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
							</svg>
							Context Out ({filesWritten.length})
						</div>
						<div class="space-y-1">
							{#each filesWritten as file}
								<a href="/files/{file.path}" class="flex items-center gap-2 text-xs hover:bg-bg-tertiary rounded px-1.5 py-1 transition-colors">
									<span class="text-success font-mono">{file.path}</span>
									<span class="text-text-muted">{file.version.slice(0, 8)}</span>
								</a>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Span tree + Waterfall -->
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase mb-3">Span Tree ({spans.length} spans)</div>
				<div class="space-y-0.5 max-h-[60vh] overflow-y-auto">
					<SpanTree
						{spans}
						selectedId={selectedSpan?.id ?? null}
						onSelect={selectSpan}
					/>
				</div>
			</div>

			<div class="bg-bg-secondary border border-border rounded p-4">
				<div class="text-text-muted text-xs uppercase mb-3">Timeline</div>
				<div class="max-h-[60vh] overflow-y-auto">
					<Waterfall
						{spans}
						selectedId={selectedSpan?.id ?? null}
						onSelect={selectSpan}
					/>
				</div>
			</div>
		</div>

		<!-- Span detail -->
		<SpanDetail span={selectedSpan} />
	{/if}
</div>
