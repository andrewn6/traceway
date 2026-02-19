<script lang="ts">
	import { page } from '$app/state';
	import { getTrace, subscribeEvents, createSpan, type Span, type SpanKind } from '$lib/api';
	import { shortId, spanStatus, spanDurationMs } from '$lib/api';
	import TraceTimeline from '$lib/components/TraceTimeline.svelte';
	import SpanDetail from '$lib/components/SpanDetail.svelte';
	import { onMount } from 'svelte';

	const traceId = $derived(page.params.id ?? '');
	let spans: Span[] = $state([]);
	let selectedSpan: Span | null = $state(null);
	let loading = $state(true);

	// Add span form
	let showAddSpan = $state(false);
	let newSpanName = $state('');
	let newSpanKindType: 'custom' | 'llm_call' | 'fs_read' | 'fs_write' = $state('custom');
	let newSpanParent: string = $state('');
	// LlmCall fields
	let llmModel = $state('');
	let llmProvider = $state('');
	// Fs fields
	let fsPath = $state('');
	// Custom fields
	let customKind = $state('task');
	// Shared
	let newSpanInput = $state('');
	let addingSpan = $state(false);

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
			} else if ((event.type === 'span_completed' || event.type === 'span_failed') && event.span.trace_id === traceId) {
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

	function buildKind(): SpanKind {
		switch (newSpanKindType) {
			case 'llm_call':
				return { type: 'llm_call', model: llmModel || 'unknown', provider: llmProvider || undefined };
			case 'fs_read':
				return { type: 'fs_read', path: fsPath || '/unknown', bytes_read: 0 };
			case 'fs_write':
				return { type: 'fs_write', path: fsPath || '/unknown', file_version: crypto.randomUUID().slice(0, 8), bytes_written: 0 };
			default:
				return { type: 'custom', kind: customKind || 'task', attributes: {} };
		}
	}

	async function handleAddSpan() {
		if (!newSpanName.trim()) return;
		addingSpan = true;
		try {
			const input = newSpanInput.trim() ? JSON.parse(newSpanInput) : undefined;
			await createSpan({
				trace_id: traceId,
				parent_id: newSpanParent || undefined,
				name: newSpanName.trim(),
				kind: buildKind(),
				input
			});
			newSpanName = '';
			newSpanInput = '';
			showAddSpan = false;
		} catch {
			// error
		}
		addingSpan = false;
	}

	function addChildSpan() {
		if (selectedSpan) {
			newSpanParent = selectedSpan.id;
		}
		showAddSpan = true;
	}

	function onSpanAction() {
		// Reload to pick up completed/failed state
		loadTrace(traceId);
	}

	const filesReadCount = $derived(spans.filter((s) => s.kind?.type === 'fs_read').length);
	const filesWrittenCount = $derived(spans.filter((s) => s.kind?.type === 'fs_write').length);

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

	// Parent options for dropdown
	const parentOptions = $derived(spans.map((s) => ({ id: s.id, name: s.name })));
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

			<div class="flex-1"></div>
			<button
				class="px-3 py-1 text-xs bg-accent/10 text-accent border border-accent/20 rounded hover:bg-accent/20 transition-colors"
				onclick={() => { showAddSpan = !showAddSpan; if (!showAddSpan) newSpanParent = ''; }}
			>
				{showAddSpan ? 'Cancel' : '+ Add Span'}
			</button>
		{/if}
	</div>

	<!-- Add span form -->
	{#if showAddSpan}
		<form
			class="mx-4 mb-2 bg-bg-secondary border border-border rounded p-4 space-y-3 shrink-0"
			onsubmit={(e) => { e.preventDefault(); handleAddSpan(); }}
		>
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				<div>
					<label for="span-name" class="block text-xs text-text-muted uppercase mb-1">Name</label>
					<input
						id="span-name"
						type="text"
						bind:value={newSpanName}
						placeholder="e.g. llm-call, read-file"
						class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted"
					/>
				</div>
				<div>
					<label for="span-kind" class="block text-xs text-text-muted uppercase mb-1">Kind</label>
					<select
						id="span-kind"
						bind:value={newSpanKindType}
						class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text"
					>
						<option value="custom">Custom</option>
						<option value="llm_call">LLM Call</option>
						<option value="fs_read">File Read</option>
						<option value="fs_write">File Write</option>
					</select>
				</div>
				<div>
					<label for="span-parent" class="block text-xs text-text-muted uppercase mb-1">Parent (optional)</label>
					<select
						id="span-parent"
						bind:value={newSpanParent}
						class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text"
					>
						<option value="">None (root)</option>
						{#each parentOptions as opt}
							<option value={opt.id}>{opt.name} ({shortId(opt.id)})</option>
						{/each}
					</select>
				</div>

				<!-- Kind-specific fields -->
				{#if newSpanKindType === 'llm_call'}
					<div>
						<label for="llm-model" class="block text-xs text-text-muted uppercase mb-1">Model</label>
						<input
							id="llm-model"
							type="text"
							bind:value={llmModel}
							placeholder="gpt-4, claude-3, etc."
							class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted"
						/>
					</div>
				{:else if newSpanKindType === 'fs_read' || newSpanKindType === 'fs_write'}
					<div>
						<label for="fs-path" class="block text-xs text-text-muted uppercase mb-1">Path</label>
						<input
							id="fs-path"
							type="text"
							bind:value={fsPath}
							placeholder="/path/to/file"
							class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted"
						/>
					</div>
				{:else}
					<div>
						<label for="custom-kind" class="block text-xs text-text-muted uppercase mb-1">Custom Kind</label>
						<input
							id="custom-kind"
							type="text"
							bind:value={customKind}
							placeholder="task, step, etc."
							class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted"
						/>
					</div>
				{/if}
			</div>

			<details class="text-xs">
				<summary class="text-text-muted cursor-pointer hover:text-text transition-colors">Input payload (optional JSON)</summary>
				<textarea
					bind:value={newSpanInput}
					rows={3}
					placeholder={'{"prompt": "..."}'}
					class="w-full mt-2 bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text font-mono placeholder:text-text-muted"
				></textarea>
			</details>

			<button
				type="submit"
				disabled={addingSpan || !newSpanName.trim()}
				class="px-4 py-1.5 text-xs bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50"
			>
				{addingSpan ? 'Creating...' : 'Create Span'}
			</button>
		</form>
	{/if}

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
		<div class="shrink-0 max-h-72 overflow-y-auto mx-4 mt-2 mb-2">
			{#if selectedSpan}
				<div class="space-y-2">
					{#if spanStatus(selectedSpan) === 'running'}
						<div class="flex items-center gap-2">
							<button
								class="px-3 py-1 text-xs bg-accent/10 text-accent border border-accent/20 rounded hover:bg-accent/20 transition-colors"
								onclick={addChildSpan}
							>+ Add Child Span</button>
						</div>
					{/if}
					<SpanDetail span={selectedSpan} {onSpanAction} />
				</div>
			{:else}
				<SpanDetail span={null} />
			{/if}
		</div>
	{/if}
</div>
