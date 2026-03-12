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

	// Search & filters
	let searchQuery = $state('');
	let searchInputEl: HTMLInputElement | undefined = $state(undefined);
	let showFilters = $state(false);
	let showMetadata = $state(true);
	let statusFilter: 'all' | 'running' | 'completed' | 'failed' = $state('all');
	let kindFilter: 'all' | 'llm_call' | 'fs_read' | 'fs_write' | 'custom' = $state('all');

	function onKeydown(e: KeyboardEvent) {
		if (e.key === '/' && document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
			e.preventDefault();
			searchInputEl?.focus();
		}
		if (e.key === 'Escape' && document.activeElement === searchInputEl) {
			searchQuery = '';
			searchInputEl?.blur();
		}
	}

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
	let splitPercent = $state(56);
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
		splitPercent = Math.max(28, Math.min(74, pct));
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
		document.addEventListener('keydown', onKeydown);

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
			document.removeEventListener('keydown', onKeydown);
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

	const totalTokens = $derived.by(() => {
		let total = 0;
		for (const s of spans) {
			if (s.kind?.type === 'llm_call') {
				total += (s.kind.input_tokens ?? 0) + (s.kind.output_tokens ?? 0);
			}
		}
		return total || null;
	});

	const totalCost = $derived.by(() => {
		let total = 0;
		for (const s of spans) {
			if (s.kind?.type === 'llm_call' && s.kind.cost != null) {
				total += s.kind.cost;
			}
		}
		return total || null;
	});

	const parentOptions = $derived(spans.map((s) => ({ id: s.id, name: s.name })));

	const timelineSpans = $derived.by(() => {
		return spans.filter((s) => {
			const statusOk = statusFilter === 'all' || spanStatus(s) === statusFilter;
			const kind = s.kind?.type ?? 'custom';
			const kindOk = kindFilter === 'all' || kind === kindFilter;
			return statusOk && kindOk;
		});
	});

	const hasActiveFilters = $derived(statusFilter !== 'all' || kindFilter !== 'all');

	function clearFilters() {
		statusFilter = 'all';
		kindFilter = 'all';
	}
</script>

{#if loading}
	<div class="text-text-muted text-sm text-center py-16">Loading...</div>
{:else if spans.length === 0}
	<div class="text-text-muted text-sm text-center py-16">Trace not found</div>
{:else}
	<div class="flex flex-col h-[calc(100vh-8rem)] -m-4 lg:-m-5 rounded-xl overflow-hidden border border-border/40 bg-bg-secondary/20">
		<!-- Header -->
		<div class="flex items-center gap-2.5 px-4 py-2 border-b border-border/55 bg-bg-secondary/30 shrink-0">
			<a href="/traces" class="text-text-secondary hover:text-text text-[13px] shrink-0">&larr; Traces</a>
			<span class="text-border/50">/</span>
			<h1 class="text-[13px] font-semibold font-mono truncate max-w-[10rem]">{shortId(traceId)}</h1>

			<span class="px-2 py-0.5 rounded text-[11px] border shrink-0
				{traceStatus === 'completed' ? 'bg-success/15 text-success border-success/25' :
				 traceStatus === 'running' ? 'bg-warning/15 text-warning border-warning/25' :
				 'bg-danger/15 text-danger border-danger/25'}">
				{traceStatus}
			</span>

			<div class="hidden md:flex items-center gap-1.5 text-[11px] text-text-secondary font-mono shrink-0">
				{#if totalDuration !== null}
					<span>{totalDuration < 1000 ? `${totalDuration}ms` : `${(totalDuration / 1000).toFixed(2)}s`}</span>
				{/if}
				{#if totalTokens}
					<span class="text-border/60">&middot;</span>
					<span>{totalTokens.toLocaleString()} tok</span>
				{/if}
				{#if totalCost}
					<span class="text-border/60">&middot;</span>
					<span class="text-success">${totalCost.toFixed(4)}</span>
				{/if}
				<span class="text-border/60">&middot;</span>
				<span class="text-text-muted">{spans.length} spans</span>
			</div>

			<div class="ml-auto flex items-center gap-1 shrink-0">
				<button
					class="btn-ghost h-7 text-[11px] px-2.5"
					onclick={() => { showAddSpan = !showAddSpan; }}
				>{showAddSpan ? 'Cancel' : '+ Span'}</button>
				<button class="btn-ghost h-7 text-[11px] px-2.5" onclick={handleExportTrace}>Export</button>
				<button
					class="btn-ghost h-7 text-[11px] px-2.5 {confirmDeleteTrace ? '!text-danger !border-danger/30' : ''}"
					onclick={handleDeleteTrace}
				>{confirmDeleteTrace ? 'Confirm delete?' : 'Delete'}</button>
			</div>
		</div>

		<!-- Add span form (collapsible) -->
		{#if showAddSpan}
			<form
				class="px-4 py-3 border-b border-border/55 bg-bg-tertiary/25 space-y-3 shrink-0 motion-rise-in"
				onsubmit={(e) => { e.preventDefault(); handleAddSpan(); }}
			>
				<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
					<div>
						<label for="span-name" class="block text-[10px] text-text-muted uppercase tracking-wider mb-1">Name</label>
						<input id="span-name" type="text" bind:value={newSpanName} placeholder="e.g. llm-call"
							class="control-input h-8 text-xs" />
					</div>
					<div>
						<label for="span-kind" class="block text-[10px] text-text-muted uppercase tracking-wider mb-1">Kind</label>
						<select id="span-kind" bind:value={newSpanKindType} class="control-select h-8 text-xs">
							<option value="custom">Custom</option>
							<option value="llm_call">LLM Call</option>
							<option value="fs_read">File Read</option>
							<option value="fs_write">File Write</option>
						</select>
					</div>
					<div>
						<label for="span-parent" class="block text-[10px] text-text-muted uppercase tracking-wider mb-1">Parent</label>
						<select id="span-parent" bind:value={newSpanParent} class="control-select h-8 text-xs">
							<option value="">None (root)</option>
							{#each parentOptions as opt}
								<option value={opt.id}>{opt.name} ({shortId(opt.id)})</option>
							{/each}
						</select>
					</div>
					{#if newSpanKindType === 'llm_call'}
						<div>
							<label for="llm-model" class="block text-[10px] text-text-muted uppercase tracking-wider mb-1">Model</label>
							<input id="llm-model" type="text" bind:value={llmModel} placeholder="gpt-4"
								class="control-input h-8 text-xs" />
						</div>
					{:else if newSpanKindType === 'fs_read' || newSpanKindType === 'fs_write'}
						<div>
							<label for="fs-path" class="block text-[10px] text-text-muted uppercase tracking-wider mb-1">Path</label>
							<input id="fs-path" type="text" bind:value={fsPath} placeholder="/path/to/file"
								class="control-input h-8 text-xs" />
						</div>
					{:else}
						<div>
							<label for="custom-kind" class="block text-[10px] text-text-muted uppercase tracking-wider mb-1">Kind</label>
							<input id="custom-kind" type="text" bind:value={customKind} placeholder="task"
								class="control-input h-8 text-xs" />
						</div>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button type="submit" disabled={addingSpan || !newSpanName.trim()}
						class="btn-primary h-7 text-xs">
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

		<!-- Split: tree left, detail right -->
		<div
			bind:this={containerEl}
			class="flex-1 min-h-0 flex"
			class:select-none={isDragging}
		>
			<!-- Left panel: search + tree -->
			<div class="min-h-0 flex flex-col" style="width: {splitPercent}%">
				<!-- Search toolbar -->
				<div class="flex items-center gap-2 px-3 py-1.5 border-b border-border/55 bg-bg-secondary/20 shrink-0">
					<svg class="w-3.5 h-3.5 text-text-muted shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
					</svg>
					<input
						bind:this={searchInputEl}
						bind:value={searchQuery}
						type="text"
						placeholder="Search spans..."
						class="flex-1 bg-transparent text-[12px] text-text placeholder:text-text-muted/50 focus:outline-none min-w-0"
					/>
					<button
						class="query-chip h-6 text-[11px] {showFilters ? 'query-chip-active' : ''}"
						onclick={() => (showFilters = !showFilters)}
					>
						Filters
						{#if hasActiveFilters}
							<span class="w-1.5 h-1.5 rounded-full bg-warning shrink-0"></span>
						{/if}
					</button>
					<button
						class="query-chip h-6 text-[11px] {showMetadata ? 'query-chip-active' : ''}"
						onclick={() => (showMetadata = !showMetadata)}
					>Meta</button>
				</div>

				<!-- Filter row (collapsible) -->
				{#if showFilters}
					<div class="flex items-center gap-1.5 px-3 py-1.5 border-b border-border/55 bg-bg-tertiary/20 shrink-0 motion-rise-in">
						<select bind:value={statusFilter} class="control-select h-7 text-[11px] w-28">
							<option value="all">All status</option>
							<option value="running">Running</option>
							<option value="completed">Completed</option>
							<option value="failed">Failed</option>
						</select>
						<select bind:value={kindFilter} class="control-select h-7 text-[11px] w-28">
							<option value="all">All kinds</option>
							<option value="llm_call">LLM call</option>
							<option value="fs_read">File read</option>
							<option value="fs_write">File write</option>
							<option value="custom">Custom</option>
						</select>
						<button class="btn-ghost h-7 text-[11px]" onclick={clearFilters}>Reset</button>
						<span class="text-[10px] text-text-muted ml-auto">{timelineSpans.length}/{spans.length}</span>
					</div>
				{/if}

				<!-- Tree -->
				<TraceTimeline
					spans={timelineSpans}
					{searchQuery}
					{showMetadata}
					selectedId={selectedSpan?.id ?? null}
					onSelect={selectSpan}
				/>
			</div>

			<!-- Resizable divider -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="w-px shrink-0 cursor-col-resize relative group"
				onpointerdown={onDividerPointerDown}
				onpointermove={onDividerPointerMove}
				onpointerup={onDividerPointerUp}
			>
				<div class="absolute inset-y-0 -left-[3px] -right-[3px] z-10"></div>
				<div class="absolute inset-y-0 left-0 right-0 bg-border/55 group-hover:bg-accent/60 transition-colors"
					class:!bg-accent={isDragging}></div>
			</div>

			<!-- Right panel: span detail -->
			<div class="flex-1 min-h-0 overflow-hidden">
				{#if selectedSpan}
					<SpanDetail span={selectedSpan} {onSpanAction} allSpans={spans} />
				{:else}
					<div class="flex items-center justify-center h-full text-text-muted text-sm">
						Select a span to view details
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
