<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { getTrace, subscribeEvents, createSpan, deleteTrace, exportJson, type Span, type SpanKind } from '$lib/api';
	import { shortId, spanStatus, spanDurationMs } from '$lib/api';
	import TraceTimeline from '$lib/components/TraceTimeline.svelte';
	import SpanDetail from '$lib/components/SpanDetail.svelte';
	import { onMount } from 'svelte';

	let traceId = $state('');
	let spans: Span[] = $state([]);
	let selectedSpan: Span | null = $state(null);
	let loading = $state(true);

	// Add span form
	let showAddSpan = $state(false);
	let newSpanName = $state('');
	let newSpanKindType: 'custom' | 'llm_call' | 'fs_read' | 'fs_write' = $state('custom');
	let newSpanParent: string = $state('');
	let llmModel = $state('');
	let llmProvider = $state('');
	let fsPath = $state('');
	let customKind = $state('task');
	let newSpanInput = $state('');
	let addingSpan = $state(false);

	// Resizable split panel
	let splitPercent = $state(40);
	let isDragging = $state(false);
	let containerEl: HTMLDivElement | undefined = $state(undefined);

	function onDividerPointerDown(e: PointerEvent) {
		isDragging = true;
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
		e.preventDefault();
	}

	function onDividerPointerMove(e: PointerEvent) {
		if (!isDragging || !containerEl) return;
		const rect = containerEl.getBoundingClientRect();
		const x = e.clientX - rect.left;
		const pct = (x / rect.width) * 100;
		splitPercent = Math.max(20, Math.min(70, pct));
	}

	function onDividerPointerUp(e: PointerEvent) {
		isDragging = false;
		(e.target as HTMLElement).releasePointerCapture(e.pointerId);
	}

	async function loadTrace(id: string) {
		try {
			const result = await getTrace(id);
			spans = result.spans;
			if (selectedSpan) {
				const updated = spans.find((s) => s.id === selectedSpan!.id);
				if (updated) selectedSpan = updated;
			}
			// Auto-select root span if nothing selected
			if (!selectedSpan && spans.length > 0) {
				const root = spans.find((s) => !s.parent_id);
				if (root) selectedSpan = root;
				else selectedSpan = spans[0];
			}
		} catch (e) {
			console.error('[TraceDetail] getTrace error:', e);
		}
		loading = false;
	}

	onMount(() => {
		const unsubPage = page.subscribe(p => {
			const newId = p.params.id ?? '';
			if (newId && newId !== traceId) {
				traceId = newId;
				loading = true;
				loadTrace(newId);
			}
		});

		const unsubEvents = subscribeEvents((event) => {
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

		return () => {
			unsubPage();
			unsubEvents();
		};
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
		loadTrace(traceId);
	}

	let confirmDeleteTrace = $state(false);

	async function handleDeleteTrace() {
		if (!confirmDeleteTrace) {
			confirmDeleteTrace = true;
			setTimeout(() => (confirmDeleteTrace = false), 3000);
			return;
		}
		try {
			await deleteTrace(traceId);
			goto('/traces');
		} catch {
			// error
		}
		confirmDeleteTrace = false;
	}

	async function handleExportTrace() {
		try {
			const data = await exportJson(traceId);
			const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `trace-${shortId(traceId)}.json`;
			a.click();
			URL.revokeObjectURL(url);
		} catch {
			// error
		}
	}

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

	const parentOptions = $derived(spans.map((s) => ({ id: s.id, name: s.name })));
</script>

<div class="h-[calc(100vh-5rem)] flex flex-col">
	<!-- Header -->
	<div class="flex items-center gap-3 px-4 py-2 shrink-0 border-b border-border">
		<a href="/traces" class="text-text-secondary hover:text-text text-sm">&larr; Traces</a>
		<span class="text-text-muted">/</span>
		<h1 class="text-sm font-bold font-mono">{shortId(traceId)}</h1>

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

			<div class="flex-1"></div>
			<button
				class="px-3 py-1 text-xs bg-accent/10 text-accent border border-accent/20 rounded hover:bg-accent/20 transition-colors"
				onclick={() => { showAddSpan = !showAddSpan; if (!showAddSpan) newSpanParent = ''; }}
			>
				{showAddSpan ? 'Cancel' : '+ Span'}
			</button>
			<button
				class="px-3 py-1 text-xs bg-bg-tertiary text-text-secondary border border-border rounded hover:text-text transition-colors"
				onclick={handleExportTrace}
			>Export</button>
			<button
				class="px-3 py-1 text-xs transition-colors border rounded {confirmDeleteTrace ? 'bg-danger/10 text-danger border-danger/30 font-semibold' : 'bg-bg-tertiary text-text-muted border-border hover:text-danger hover:border-danger/30'}"
				onclick={handleDeleteTrace}
			>{confirmDeleteTrace ? 'Confirm?' : 'Delete'}</button>
		{/if}
	</div>

	<!-- Add span form (slides down) -->
	{#if showAddSpan}
		<form
			class="px-4 py-3 bg-bg-secondary border-b border-border space-y-3 shrink-0"
			onsubmit={(e) => { e.preventDefault(); handleAddSpan(); }}
		>
			<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
				<div>
					<label for="span-name" class="block text-xs text-text-muted uppercase mb-1">Name</label>
					<input id="span-name" type="text" bind:value={newSpanName} placeholder="e.g. llm-call"
						class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted" />
				</div>
				<div>
					<label for="span-kind" class="block text-xs text-text-muted uppercase mb-1">Kind</label>
					<select id="span-kind" bind:value={newSpanKindType}
						class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text">
						<option value="custom">Custom</option>
						<option value="llm_call">LLM Call</option>
						<option value="fs_read">File Read</option>
						<option value="fs_write">File Write</option>
					</select>
				</div>
				<div>
					<label for="span-parent" class="block text-xs text-text-muted uppercase mb-1">Parent</label>
					<select id="span-parent" bind:value={newSpanParent}
						class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text">
						<option value="">None (root)</option>
						{#each parentOptions as opt}
							<option value={opt.id}>{opt.name} ({shortId(opt.id)})</option>
						{/each}
					</select>
				</div>
				{#if newSpanKindType === 'llm_call'}
					<div>
						<label for="llm-model" class="block text-xs text-text-muted uppercase mb-1">Model</label>
						<input id="llm-model" type="text" bind:value={llmModel} placeholder="gpt-4"
							class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted" />
					</div>
				{:else if newSpanKindType === 'fs_read' || newSpanKindType === 'fs_write'}
					<div>
						<label for="fs-path" class="block text-xs text-text-muted uppercase mb-1">Path</label>
						<input id="fs-path" type="text" bind:value={fsPath} placeholder="/path/to/file"
							class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted" />
					</div>
				{:else}
					<div>
						<label for="custom-kind" class="block text-xs text-text-muted uppercase mb-1">Kind</label>
						<input id="custom-kind" type="text" bind:value={customKind} placeholder="task"
							class="w-full bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted" />
					</div>
				{/if}
			</div>
			<div class="flex items-center gap-3">
				<button type="submit" disabled={addingSpan || !newSpanName.trim()}
					class="px-4 py-1.5 text-xs bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50">
					{addingSpan ? 'Creating...' : 'Create Span'}
				</button>
				<details class="text-xs">
					<summary class="text-text-muted cursor-pointer hover:text-text transition-colors">Input JSON</summary>
					<textarea bind:value={newSpanInput} rows={2} placeholder={'{"prompt": "..."}'}
						class="w-full mt-1 bg-bg-tertiary border border-border rounded px-2 py-1.5 text-xs text-text font-mono placeholder:text-text-muted"></textarea>
				</details>
			</div>
		</form>
	{/if}

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8 flex-1">Loading...</div>
	{:else if spans.length === 0}
		<div class="text-text-muted text-sm text-center py-8 flex-1">Trace not found</div>
	{:else}
		<!-- Split panel: tree left, detail right -->
		<div
			bind:this={containerEl}
			class="flex-1 min-h-0 flex"
			class:select-none={isDragging}
		>
			<!-- Left panel: span tree -->
			<div class="min-h-0 overflow-hidden flex flex-col" style="width: {splitPercent}%">
				<TraceTimeline
					{spans}
					selectedId={selectedSpan?.id ?? null}
					onSelect={selectSpan}
				/>
			</div>

			<!-- Resizable divider -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="w-1 shrink-0 cursor-col-resize relative group"
				onpointerdown={onDividerPointerDown}
				onpointermove={onDividerPointerMove}
				onpointerup={onDividerPointerUp}
			>
				<div class="absolute inset-y-0 -left-0.5 -right-0.5 bg-border group-hover:bg-accent/50 transition-colors"
					class:bg-accent={isDragging}></div>
			</div>

			<!-- Right panel: span detail -->
			<div class="flex-1 min-h-0 overflow-y-auto bg-bg">
				{#if selectedSpan}
					<SpanDetail span={selectedSpan} {onSpanAction} allSpans={spans} />
				{:else}
					<div class="flex items-center justify-center h-full text-text-muted text-sm">
						Select a span to view details
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
