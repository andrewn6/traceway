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

	// Search
	let searchQuery = $state('');
	let searchFocused = $state(false);
	let searchInputEl: HTMLInputElement | undefined = $state(undefined);

	// Build suggestion list from current spans
	const searchSuggestions = $derived.by((): Array<{ label: string; category: string }> => {
		if (!searchQuery.trim()) return [];
		const q = searchQuery.toLowerCase().trim();
		const seen = new Set<string>();
		const results: Array<{ label: string; category: string }> = [];

		// Span names
		for (const s of spans) {
			const key = `name:${s.name}`;
			if (!seen.has(key) && s.name.toLowerCase().includes(q)) {
				seen.add(key);
				results.push({ label: s.name, category: 'span' });
			}
		}
		// Models
		for (const s of spans) {
			if (s.kind?.type === 'llm_call' && s.kind.model) {
				const key = `model:${s.kind.model}`;
				if (!seen.has(key) && s.kind.model.toLowerCase().includes(q)) {
					seen.add(key);
					results.push({ label: s.kind.model, category: 'model' });
				}
			}
		}
		// Providers
		for (const s of spans) {
			if (s.kind?.type === 'llm_call' && s.kind.provider) {
				const key = `provider:${s.kind.provider}`;
				if (!seen.has(key) && s.kind.provider.toLowerCase().includes(q)) {
					seen.add(key);
					results.push({ label: s.kind.provider, category: 'provider' });
				}
			}
		}
		// Kind types
		const kindLabels: Record<string, string> = { llm_call: 'LLM Call', fs_read: 'File Read', fs_write: 'File Write', custom: 'Custom' };
		const seenKinds = new Set<string>();
		for (const s of spans) {
			const kt = s.kind?.type ?? 'custom';
			if (!seenKinds.has(kt)) {
				seenKinds.add(kt);
				const label = kindLabels[kt] ?? kt;
				if (label.toLowerCase().includes(q) || kt.toLowerCase().includes(q)) {
					const key = `kind:${kt}`;
					if (!seen.has(key)) {
						seen.add(key);
						results.push({ label, category: 'kind' });
					}
				}
			}
		}
		// Custom kind values
		for (const s of spans) {
			if (s.kind?.type === 'custom' && s.kind.kind) {
				const key = `customkind:${s.kind.kind}`;
				if (!seen.has(key) && s.kind.kind.toLowerCase().includes(q)) {
					seen.add(key);
					results.push({ label: s.kind.kind, category: 'kind' });
				}
			}
		}

		return results.slice(0, 8);
	});

	function applySuggestion(label: string) {
		searchQuery = label;
		searchFocused = false;
		searchInputEl?.blur();
	}

	// Keyboard shortcut: / to focus search
	function onKeydown(e: KeyboardEvent) {
		if (e.key === '/' && document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
			e.preventDefault();
			searchInputEl?.focus();
		}
		if (e.key === 'Escape' && searchFocused) {
			searchQuery = '';
			searchFocused = false;
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
</script>

<div class="h-[calc(100vh-8rem)] flex flex-col gap-3">
	<!-- Header -->
	<div class="flex items-center gap-3 px-4 py-2.5 shrink-0 glass-surface rounded-2xl border border-border/70 shadow-[0_24px_44px_-34px_rgba(5,14,32,0.9)]">
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

			<!-- Summary stats -->
			<div class="flex items-center gap-1.5 text-xs text-text-secondary font-mono bg-bg-tertiary/35 rounded-lg px-2.5 py-1 border border-border/45">
				{#if totalDuration !== null}
					<span class="inline-flex items-center gap-1">
						<svg class="w-3.5 h-3.5 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
						{totalDuration < 1000 ? `${totalDuration}ms` : `${(totalDuration / 1000).toFixed(2)}s`}
					</span>
				{/if}
				{#if totalTokens}
					<span class="text-border mx-0.5">|</span>
					<span class="inline-flex items-center gap-1">
						<svg class="w-3.5 h-3.5 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25H12" /></svg>
						{totalTokens.toLocaleString()}
					</span>
				{/if}
				{#if totalCost}
					<span class="text-border mx-0.5">|</span>
					<span class="inline-flex items-center gap-1 text-success">
						<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v12m-3-2.818.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
						${totalCost.toFixed(4)}
					</span>
				{/if}
				<span class="text-border mx-0.5">|</span>
				<span class="text-text-muted">{spans.length} spans</span>
			</div>

			<div class="flex-1"></div>
			<button
				class="px-3 py-1 text-xs bg-accent/10 text-accent border border-accent/20 rounded-lg hover:bg-accent/20 transition-colors duration-150"
				onclick={() => { showAddSpan = !showAddSpan; if (!showAddSpan) newSpanParent = ''; }}
			>
				{showAddSpan ? 'Cancel' : '+ Span'}
			</button>
			<button
				class="px-3 py-1 text-xs bg-bg-tertiary/70 text-text-secondary border border-border rounded-lg hover:text-text hover:bg-bg-tertiary transition-colors duration-150"
				onclick={handleExportTrace}
			>Export</button>
			<button
				class="px-3 py-1 text-xs transition-colors duration-150 border rounded-lg {confirmDeleteTrace ? 'bg-danger/10 text-danger border-danger/30 font-semibold' : 'bg-bg-tertiary/70 text-text-muted border-border hover:text-danger hover:border-danger/30'}"
				onclick={handleDeleteTrace}
			>{confirmDeleteTrace ? 'Confirm?' : 'Delete'}</button>
		{/if}
	</div>

	<!-- Add span form (slides down) -->
	{#if showAddSpan}
		<form
			class="px-4 py-3 glass-soft rounded-2xl space-y-3 shrink-0"
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
			class="flex-1 min-h-0 flex gap-3"
			class:select-none={isDragging}
		>
			<!-- Left panel: search + span tree -->
			<div class="min-h-0 overflow-hidden flex flex-col glass-surface rounded-2xl border border-border/60 shadow-[0_28px_56px_-36px_rgba(5,14,32,0.9)]" style="width: {splitPercent}%">
				<div class="px-3 py-2 border-b border-border/55 bg-gradient-to-r from-bg-secondary/70 via-bg-secondary/45 to-transparent">
					<p class="text-[10px] uppercase tracking-[0.16em] text-text-muted">Trace timeline</p>
				</div>
				<!-- Search bar -->
				<div class="relative shrink-0 border-b border-border/55 bg-bg-secondary/25">
					<div class="flex items-center px-3 py-1.5 gap-2">
						<svg class="w-3.5 h-3.5 text-text-muted shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
						</svg>
						<input
							bind:this={searchInputEl}
							bind:value={searchQuery}
							onfocus={() => searchFocused = true}
							onblur={() => setTimeout(() => searchFocused = false, 150)}
							type="text"
							placeholder="Search spans...  /"
							class="flex-1 bg-transparent text-xs text-text placeholder:text-text-muted/50 focus:outline-none"
						/>
						{#if searchQuery}
							<button
							class="text-text-muted hover:text-text text-xs shrink-0 transition-colors duration-150"
							onclick={() => { searchQuery = ''; searchInputEl?.focus(); }}
							aria-label="Clear search"
							>
								<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
									<path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
								</svg>
							</button>
						{/if}
					</div>

					<!-- Autocomplete suggestions -->
					{#if searchFocused && searchQuery.trim() && searchSuggestions.length > 0}
						<div class="absolute left-0 right-0 top-full z-20 bg-bg-secondary/95 border border-border border-t-0 rounded-b-lg shadow-[0_20px_36px_-30px_rgba(0,0,0,0.9)] overflow-hidden backdrop-blur-sm">
							{#each searchSuggestions as suggestion}
								<button
									class="w-full flex items-center gap-2 px-3 py-1.5 text-left hover:bg-bg-tertiary/85 transition-colors duration-150"
									onmousedown={() => applySuggestion(suggestion.label)}
								>
									<span class="text-[10px] uppercase tracking-wider text-text-muted/60 w-12 shrink-0">{suggestion.category}</span>
									<span class="text-xs text-text truncate">{suggestion.label}</span>
								</button>
							{/each}
						</div>
					{/if}
				</div>

				<TraceTimeline
					{spans}
					{searchQuery}
					selectedId={selectedSpan?.id ?? null}
					onSelect={selectSpan}
				/>
			</div>

			<!-- Resizable divider -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="w-1.5 shrink-0 cursor-col-resize relative group"
				onpointerdown={onDividerPointerDown}
				onpointermove={onDividerPointerMove}
				onpointerup={onDividerPointerUp}
			>
				<div class="absolute inset-y-1 -left-0.5 -right-0.5 bg-border/45 rounded-full group-hover:bg-accent/55 transition-colors duration-150"
					class:bg-accent={isDragging}></div>
			</div>

			<!-- Right panel: span detail -->
			<div class="flex-1 min-h-0 overflow-y-auto glass-surface rounded-2xl border border-border/60 shadow-[0_28px_56px_-36px_rgba(5,14,32,0.9)]">
				<div class="px-3 py-2 border-b border-border/55 bg-gradient-to-r from-bg-secondary/70 via-bg-secondary/45 to-transparent sticky top-0 z-10 backdrop-blur-md">
					<p class="text-[10px] uppercase tracking-[0.16em] text-text-muted">Span inspection</p>
				</div>
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
