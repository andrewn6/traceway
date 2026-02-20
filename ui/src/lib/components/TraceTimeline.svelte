<script lang="ts">
	import type { Span } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs } from '$lib/api';
	import SpanKindIcon from './SpanKindIcon.svelte';
	import { onMount, onDestroy } from 'svelte';

	let {
		spans,
		selectedId = null,
		onSelect
	}: {
		spans: Span[];
		selectedId?: string | null;
		onSelect?: (span: Span) => void;
	} = $props();

	// ── Constants ──────────────────────────────────────────────────────
	const ROW_HEIGHT = 32;
	const OVERSCAN = 10;
	const INDENT_PX = 18;

	// ── Search / filter ───────────────────────────────────────────────
	let searchQuery = $state('');

	// ── View mode ─────────────────────────────────────────────────────
	let viewMode: 'tree' | 'flat' = $state('tree');

	// ── Collapse state ─────────────────────────────────────────────────
	let collapsed: Set<string> = $state(new Set());

	function toggleCollapse(id: string) {
		const next = new Set(collapsed);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		collapsed = next;
	}

	function expandAll() {
		collapsed = new Set();
	}

	function collapseAll() {
		const withChildren = new Set<string>();
		for (const s of spans) {
			if (s.parent_id) withChildren.add(s.parent_id);
		}
		collapsed = withChildren;
	}

	// ── Build child index ──────────────────────────────────────────────
	interface TreeNode {
		span: Span;
		depth: number;
		hasChildren: boolean;
		descendantCount: number;
	}

	const childIndex = $derived.by(() => {
		const idx = new Map<string | null, Span[]>();
		for (const s of spans) {
			const key = s.parent_id ?? null;
			let arr = idx.get(key);
			if (!arr) {
				arr = [];
				idx.set(key, arr);
			}
			arr.push(s);
		}
		return idx;
	});

	function countDescendants(id: string): number {
		const children = childIndex.get(id);
		if (!children) return 0;
		let count = children.length;
		for (const c of children) count += countDescendants(c.id);
		return count;
	}

	// ── Time range for relative offset display ────────────────────────
	const timeRange = $derived.by(() => {
		if (spans.length === 0) return { min: 0, max: 1 };
		const starts = spans.map((s) => new Date(spanStartedAt(s)).getTime());
		const ends = spans.map((s) => {
			const end = spanEndedAt(s);
			return end ? new Date(end).getTime() : Date.now();
		});
		const min = Math.min(...starts);
		const max = Math.max(...ends);
		return { min, max: max === min ? min + 1 : max };
	});

	// ── DFS flat tree (respects collapse + search) ─────────────────────
	const flatTree = $derived.by((): TreeNode[] => {
		const query = searchQuery.toLowerCase().trim();

		if (viewMode === 'flat') {
			// Flat: sorted by start time, all depth 0
			const sorted = [...spans].sort(
				(a, b) => new Date(spanStartedAt(a)).getTime() - new Date(spanStartedAt(b)).getTime()
			);
			const filtered = query
				? sorted.filter((s) => s.name.toLowerCase().includes(query) || kindLabel(s).toLowerCase().includes(query))
				: sorted;
			return filtered.map((span) => ({
				span,
				depth: 0,
				hasChildren: childIndex.has(span.id),
				descendantCount: 0
			}));
		}

		// Tree mode
		const result: TreeNode[] = [];
		function walk(parentId: string | null, depth: number) {
			const children = childIndex.get(parentId);
			if (!children) return;
			const sorted = [...children].sort(
				(a, b) => new Date(spanStartedAt(a)).getTime() - new Date(spanStartedAt(b)).getTime()
			);
			for (const span of sorted) {
				if (query && !span.name.toLowerCase().includes(query) && !kindLabel(span).toLowerCase().includes(query)) {
					// Still walk children in case they match
					walk(span.id, depth + 1);
					continue;
				}
				const hasChildren = childIndex.has(span.id);
				const descendantCount = hasChildren ? countDescendants(span.id) : 0;
				result.push({ span, depth, hasChildren, descendantCount });
				if (hasChildren && !collapsed.has(span.id)) {
					walk(span.id, depth + 1);
				}
			}
		}
		walk(null, 0);
		return result;
	});

	// ── Virtual scroll ─────────────────────────────────────────────────
	let scrollTop = $state(0);
	let containerHeight = $state(400);
	let scrollContainer: HTMLDivElement | undefined = $state(undefined);

	const totalHeight = $derived(flatTree.length * ROW_HEIGHT);

	const visibleRange = $derived.by(() => {
		const startIdx = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
		const endIdx = Math.min(
			flatTree.length,
			Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + OVERSCAN
		);
		return { startIdx, endIdx };
	});

	const visibleNodes = $derived(flatTree.slice(visibleRange.startIdx, visibleRange.endIdx));

	function onScroll(e: Event) {
		const el = e.target as HTMLDivElement;
		scrollTop = el.scrollTop;
	}

	// ── Span helpers ───────────────────────────────────────────────────
	function kindLabel(s: Span): string {
		if (!s.kind) return 'unknown';
		if (s.kind.type === 'custom') return s.kind.kind;
		return s.kind.type;
	}

	function kindType(s: Span): string {
		if (!s.kind) return 'custom';
		return s.kind.type;
	}

	function modelBadge(s: Span): string | null {
		if (s.kind?.type === 'llm_call') return s.kind.model;
		return null;
	}

	function tokenCount(s: Span): string | null {
		if (s.kind?.type !== 'llm_call') return null;
		const inp = s.kind.input_tokens ?? null;
		const out = s.kind.output_tokens ?? null;
		if (inp == null && out == null) return null;
		const total = (inp ?? 0) + (out ?? 0);
		return total.toLocaleString();
	}

	function costBadge(s: Span): string | null {
		if (s.kind?.type === 'llm_call' && s.kind.cost != null) {
			return `$${s.kind.cost.toFixed(4)}`;
		}
		return null;
	}

	function bytesBadge(s: Span): string | null {
		if (!s.kind) return null;
		if (s.kind.type === 'fs_read') return formatBytes(s.kind.bytes_read);
		if (s.kind.type === 'fs_write') return formatBytes(s.kind.bytes_written);
		return null;
	}

	function formatBytes(n: number): string {
		if (n < 1024) return `${n}B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)}KB`;
		return `${(n / (1024 * 1024)).toFixed(1)}MB`;
	}

	function formatDuration(ms: number | null): string {
		if (ms === null) return '...';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function relativeOffset(s: Span): string {
		const start = new Date(spanStartedAt(s)).getTime();
		const offset = start - timeRange.min;
		if (offset < 1000) return `+${offset}ms`;
		return `+${(offset / 1000).toFixed(2)}s`;
	}

	function statusDotClass(s: Span): string {
		const st = spanStatus(s);
		if (st === 'running') return 'bg-warning animate-pulse';
		if (st === 'failed') return 'bg-danger';
		return 'bg-success';
	}

	// Scroll selected span into view
	$effect(() => {
		if (selectedId && scrollContainer) {
			const idx = flatTree.findIndex((n) => n.span.id === selectedId);
			if (idx >= 0) {
				const top = idx * ROW_HEIGHT;
				const bottom = top + ROW_HEIGHT;
				const viewTop = scrollContainer.scrollTop;
				const viewBottom = viewTop + containerHeight;
				if (top < viewTop || bottom > viewBottom) {
					scrollContainer.scrollTop = top - containerHeight / 2 + ROW_HEIGHT / 2;
				}
			}
		}
	});
</script>

<div class="flex flex-col h-full min-h-0 bg-bg-secondary border-r border-border">
	<!-- Toolbar -->
	<div class="flex items-center gap-1.5 px-2 py-1.5 border-b border-border shrink-0">
		<!-- View mode toggle -->
		<div class="flex items-center bg-bg-tertiary rounded text-[10px]">
			<button
				class="px-2 py-1 rounded transition-colors {viewMode === 'tree' ? 'bg-accent/20 text-accent' : 'text-text-muted hover:text-text'}"
				onclick={() => viewMode = 'tree'}
			>Tree</button>
			<button
				class="px-2 py-1 rounded transition-colors {viewMode === 'flat' ? 'bg-accent/20 text-accent' : 'text-text-muted hover:text-text'}"
				onclick={() => viewMode = 'flat'}
			>Flat</button>
		</div>

		{#if viewMode === 'tree'}
			<div class="flex items-center gap-0.5 text-[10px] text-text-muted">
				<button class="hover:text-text transition-colors px-1" onclick={expandAll}>expand</button>
				<span>/</span>
				<button class="hover:text-text transition-colors px-1" onclick={collapseAll}>collapse</button>
			</div>
		{/if}

		<div class="flex-1"></div>

		<!-- Search -->
		<div class="relative">
			<input
				type="text"
				bind:value={searchQuery}
				placeholder="Filter..."
				class="w-28 bg-bg-tertiary border border-border rounded px-2 py-0.5 text-[11px] text-text placeholder:text-text-muted focus:w-40 focus:border-accent/50 transition-all outline-none"
			/>
			{#if searchQuery}
				<button
					class="absolute right-1 top-1/2 -translate-y-1/2 text-text-muted hover:text-text text-xs"
					onclick={() => searchQuery = ''}
				>x</button>
			{/if}
		</div>
	</div>

	<!-- Span count -->
	<div class="flex items-center justify-between px-3 py-1 border-b border-border text-[10px] text-text-muted shrink-0 uppercase tracking-wider">
		<span>{flatTree.length} span{flatTree.length !== 1 ? 's' : ''}</span>
		<span>duration</span>
	</div>

	<!-- Virtual scroll area -->
	<div
		bind:this={scrollContainer}
		bind:clientHeight={containerHeight}
		class="flex-1 min-h-0 overflow-y-auto"
		onscroll={onScroll}
	>
		<div class="relative" style="height: {totalHeight}px">
			{#each visibleNodes as node, i (node.span.id)}
				{@const idx = visibleRange.startIdx + i}
				{@const s = node.span}
				{@const status = spanStatus(s)}
				{@const duration = spanDurationMs(s)}
				{@const model = modelBadge(s)}
				{@const tokens = tokenCount(s)}
				{@const cost = costBadge(s)}
				{@const bytes = bytesBadge(s)}
				<button
					class="absolute left-0 right-0 flex items-center text-xs transition-colors group
						{selectedId === s.id ? 'bg-accent/10 border-l-2 border-l-accent' : 'hover:bg-bg-tertiary border-l-2 border-l-transparent'}"
					style="top: {idx * ROW_HEIGHT}px; height: {ROW_HEIGHT}px"
					onclick={() => onSelect?.(s)}
				>
					<!-- Span info -->
					<div class="flex items-center gap-1 flex-1 px-2 overflow-hidden min-w-0">
						<!-- Indent + collapse -->
						{#if viewMode === 'tree'}
							<div class="flex items-center shrink-0" style="width: {node.depth * INDENT_PX + 18}px">
								<div style="width: {node.depth * INDENT_PX}px"></div>
								{#if node.hasChildren}
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<span
										role="switch"
										aria-checked={!collapsed.has(s.id)}
										tabindex={-1}
										class="w-[18px] h-[18px] flex items-center justify-center text-text-muted hover:text-text transition-colors cursor-pointer"
										onclick={(e: MouseEvent) => { e.stopPropagation(); toggleCollapse(s.id); }}
									>
										{#if collapsed.has(s.id)}
											<svg class="w-3 h-3" viewBox="0 0 12 12" fill="currentColor"><path d="M4 2l6 4-6 4V2z"/></svg>
										{:else}
											<svg class="w-3 h-3" viewBox="0 0 12 12" fill="currentColor"><path d="M2 4l4 6 4-6H2z"/></svg>
										{/if}
									</span>
								{:else}
									<div class="w-[18px]"></div>
								{/if}
							</div>
						{/if}

						<!-- Status dot -->
						<span class="w-2 h-2 rounded-full shrink-0 {statusDotClass(s)}"></span>

						<!-- Icon -->
						<div class="shrink-0">
							<SpanKindIcon span={s} />
						</div>

						<!-- Name -->
						<span class="text-text truncate text-[11px] font-medium">{s.name}</span>

						<!-- Inline badges -->
						{#if model}
							<span class="shrink-0 text-purple-400 text-[9px] opacity-70">{model}</span>
						{/if}
						{#if tokens}
							<span class="shrink-0 text-text-muted text-[9px]">{tokens}tok</span>
						{/if}
						{#if cost}
							<span class="shrink-0 text-text-muted text-[9px]">{cost}</span>
						{/if}
						{#if bytes}
							<span class="shrink-0 text-text-muted text-[9px]">{bytes}</span>
						{/if}

						<!-- Collapsed count -->
						{#if node.hasChildren && collapsed.has(s.id)}
							<span class="shrink-0 text-text-muted text-[9px] bg-bg-tertiary rounded px-1">+{node.descendantCount}</span>
						{/if}
					</div>

					<!-- Duration / offset (right side) -->
					<div class="shrink-0 flex flex-col items-end pr-3 text-right min-w-14">
						<span class="text-[11px] text-text-secondary font-mono">{formatDuration(duration)}</span>
						<span class="text-[9px] text-text-muted font-mono">{relativeOffset(s)}</span>
					</div>
				</button>
			{/each}
		</div>
	</div>
</div>
