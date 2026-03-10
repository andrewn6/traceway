<script lang="ts">
	import type { Span } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs } from '$lib/api';
	import SpanKindIcon from './SpanKindIcon.svelte';
	import TimelineView from './TimelineView.svelte';
	import ReaderView from './ReaderView.svelte';
	import { onMount, onDestroy } from 'svelte';

	let {
		spans,
		selectedId = null,
		onSelect,
		searchQuery = '',
		showMetadata = true,
	}: {
		spans: Span[];
		selectedId?: string | null;
		onSelect?: (span: Span) => void;
		searchQuery?: string;
		showMetadata?: boolean;
	} = $props();

	// ── Constants ──────────────────────────────────────────────────────
	const SPAN_ROW_HEIGHT = 40;
	const PREVIEW_LINE_HEIGHT = 17;
	const PREVIEW_PADDING = 6;
	const OVERSCAN = 10;
	const INDENT_PX = 20;

	/** Estimate preview height based on text length (rough char-per-line estimate) */
	function previewHeight(text: string): number {
		const charsPerLine = 72;
		const lines = Math.min(3, Math.max(1, Math.ceil(text.length / charsPerLine)));
		return lines * PREVIEW_LINE_HEIGHT + PREVIEW_PADDING;
	}

	// ── View mode (persisted to localStorage) ─────────────────────────
	type ViewMode = 'tree' | 'flat' | 'timeline' | 'reader';
	const VIEW_MODE_KEY = 'traceway:trace-view-mode';
	let viewMode: ViewMode = $state('tree');

	onMount(() => {
		const stored = localStorage.getItem(VIEW_MODE_KEY);
		if (stored === 'tree' || stored === 'flat' || stored === 'timeline' || stored === 'reader') {
			viewMode = stored;
		}
	});

	$effect(() => {
		localStorage.setItem(VIEW_MODE_KEY, viewMode);
	});

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
	type TreeNode = {
		type: 'span';
		span: Span;
		depth: number;
		hasChildren: boolean;
		descendantCount: number;
		height: number;
	} | {
		type: 'preview';
		span: Span;
		depth: number;
		text: string;
		height: number;
	};

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

	function pushSpanNode(result: TreeNode[], span: Span, depth: number) {
		const hasChildren = childIndex.has(span.id);
		const descendantCount = hasChildren ? countDescendants(span.id) : 0;
		result.push({ type: 'span', span, depth, hasChildren, descendantCount, height: SPAN_ROW_HEIGHT });
	}

	const flatTree = $derived.by((): TreeNode[] => {
		const query = searchQuery.toLowerCase().trim();

		if (viewMode === 'flat') {
			const sorted = [...spans].sort(
				(a, b) => new Date(spanStartedAt(a)).getTime() - new Date(spanStartedAt(b)).getTime()
			);
			const filtered = query
				? sorted.filter((s) => s.name.toLowerCase().includes(query) || kindLabel(s).toLowerCase().includes(query))
				: sorted;
			const result: TreeNode[] = [];
			for (const span of filtered) {
				pushSpanNode(result, span, 0);
			}
			return result;
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
					walk(span.id, depth + 1);
					continue;
				}
				pushSpanNode(result, span, depth);
				const hasChildren = childIndex.has(span.id);
				if (hasChildren && !collapsed.has(span.id)) {
					walk(span.id, depth + 1);
				}
			}
		}
		walk(null, 0);
		return result;
	});

	// ── Virtual scroll (variable height) ──────────────────────────────
	let scrollTop = $state(0);
	let containerHeight = $state(400);
	let scrollContainer: HTMLDivElement | undefined = $state(undefined);

	// Precompute cumulative offsets for variable-height rows
	const rowOffsets = $derived.by(() => {
		const offsets: number[] = new Array(flatTree.length);
		let y = 0;
		for (let i = 0; i < flatTree.length; i++) {
			offsets[i] = y;
			y += flatTree[i].height;
		}
		return offsets;
	});

	const totalHeight = $derived(
		flatTree.length === 0 ? 0 : rowOffsets[flatTree.length - 1] + flatTree[flatTree.length - 1].height
	);

	const visibleRange = $derived.by(() => {
		// Binary search for first visible row
		let lo = 0, hi = flatTree.length;
		while (lo < hi) {
			const mid = (lo + hi) >> 1;
			if (rowOffsets[mid] + flatTree[mid].height <= scrollTop - OVERSCAN * SPAN_ROW_HEIGHT) lo = mid + 1;
			else hi = mid;
		}
		const startIdx = Math.max(0, lo);

		lo = startIdx; hi = flatTree.length;
		while (lo < hi) {
			const mid = (lo + hi) >> 1;
			if (rowOffsets[mid] < scrollTop + containerHeight + OVERSCAN * SPAN_ROW_HEIGHT) lo = mid + 1;
			else hi = mid;
		}
		const endIdx = Math.min(flatTree.length, lo);

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

	function providerBadge(s: Span): string | null {
		if (s.kind?.type === 'llm_call') return s.kind.provider ?? null;
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

	function extractPreviewText(s: Span): string | null {
		// LLM call: try output preview, then input preview
		if (s.kind?.type === 'llm_call') {
			if (s.kind.output_preview) return s.kind.output_preview;
			if (s.kind.input_preview) return s.kind.input_preview;
		}
		// String output
		if (typeof s.output === 'string' && s.output.length > 0) {
			return s.output;
		}
		// Object output: try to find content
		if (s.output && typeof s.output === 'object') {
			const obj = s.output as Record<string, unknown>;
			if (typeof obj.content === 'string') return obj.content;
			if (Array.isArray(obj.choices)) {
				const first = obj.choices[0] as Record<string, unknown> | undefined;
				if (first?.message && typeof (first.message as Record<string, unknown>).content === 'string') {
					return (first.message as Record<string, unknown>).content as string;
				}
			}
			// Try messages array (chat format output)
			if (Array.isArray(obj.messages)) {
				const last = obj.messages[obj.messages.length - 1] as Record<string, unknown> | undefined;
				if (last && typeof last.content === 'string') return last.content;
			}
		}
		// Input: try to extract last user message
		if (s.input && typeof s.input === 'object' && !Array.isArray(s.input)) {
			const obj = s.input as Record<string, unknown>;
			if (Array.isArray(obj.messages) && obj.messages.length > 0) {
				const last = obj.messages[obj.messages.length - 1] as Record<string, unknown> | undefined;
				if (last && typeof last.content === 'string') return last.content;
			}
		}
		if (Array.isArray(s.input) && s.input.length > 0) {
			const last = s.input[s.input.length - 1] as Record<string, unknown> | undefined;
			if (last && typeof last.content === 'string') return last.content;
		}
		return null;
	}

	/** Whether a span should get an inline content preview block */
	function shouldShowPreview(s: Span): boolean {
		return s.kind?.type === 'llm_call' && extractPreviewText(s) !== null;
	}

	// Scroll selected span into view
	$effect(() => {
		if (selectedId && scrollContainer) {
			const idx = flatTree.findIndex((n) => n.type === 'span' && n.span.id === selectedId);
			if (idx >= 0) {
				const top = rowOffsets[idx];
				const bottom = top + flatTree[idx].height;
				const viewTop = scrollContainer.scrollTop;
				const viewBottom = viewTop + containerHeight;
				if (top < viewTop || bottom > viewBottom) {
					scrollContainer.scrollTop = top - containerHeight / 2 + flatTree[idx].height / 2;
				}
			}
		}
	});
</script>

	<div class="flex flex-col h-full min-h-0 bg-transparent">
	<!-- Toolbar -->
	<div class="flex items-center gap-2 px-3 py-2.5 border-b border-border/55 shrink-0 bg-bg-secondary/30 backdrop-blur-sm">
		{#if viewMode === 'tree' || viewMode === 'flat'}
			<span class="text-[11px] uppercase tracking-[0.1em] text-text-muted">{flatTree.filter(n => n.type === 'span').length} spans</span>
		{/if}

		<div class="flex-1"></div>

		{#if viewMode === 'tree'}
			<div class="flex items-center gap-0.5 text-[11px] text-text-muted bg-bg-tertiary/35 border border-border/50 rounded-md px-1.5 py-0.5">
				<button class="hover:text-text transition-colors duration-150 px-1" onclick={expandAll}>expand</button>
				<span class="text-border">/</span>
				<button class="hover:text-text transition-colors duration-150 px-1" onclick={collapseAll}>collapse</button>
			</div>
		{/if}

		<!-- View mode toggle -->
		<div class="flex items-center bg-bg-tertiary/45 border border-border/55 rounded-md text-[11px] p-0.5 backdrop-blur-sm">
			{#each (['tree', 'flat', 'timeline', 'reader'] as const) as mode}
				<button
					class="px-2 py-0.5 rounded transition-colors duration-150 capitalize {viewMode === mode ? 'bg-accent/20 text-accent border border-accent/35' : 'text-text-muted hover:text-text'}"
					onclick={() => viewMode = mode}
				>{mode}</button>
			{/each}
		</div>
	</div>

	{#if viewMode === 'timeline'}
		<TimelineView {spans} {selectedId} {onSelect} {searchQuery} />
	{:else if viewMode === 'reader'}
		<ReaderView {spans} {selectedId} {onSelect} {searchQuery} />
	{:else}
		<!-- Virtual scroll area (tree/flat modes) -->
		<div
			bind:this={scrollContainer}
			bind:clientHeight={containerHeight}
			class="flex-1 min-h-0 overflow-y-auto bg-[radial-gradient(120%_140%_at_50%_0%,rgba(255,255,255,0.02),transparent_65%)]"
			onscroll={onScroll}
		>
			<div class="relative" style="height: {totalHeight}px">
				{#each visibleNodes as node, i (`${node.type}-${node.span.id}-${i}`)}
					{@const idx = visibleRange.startIdx + i}
					{@const topPx = rowOffsets[idx]}
					{@const s = node.span}

					{#if node.type === 'preview'}
						<!-- Content preview block -->
						<div
							class="absolute left-0 right-0 cursor-pointer transition-colors duration-150
								{selectedId === s.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'border-l-2 border-l-transparent hover:bg-bg-tertiary/50'}"
							style="top: {topPx}px; height: {node.height}px"
							role="button"
							tabindex={0}
							aria-label={`Select span ${s.name}`}
							onclick={() => onSelect?.(s)}
							onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') onSelect?.(s); }}
						>
							<div class="overflow-hidden pr-3" style="padding-left: {(node.depth + (viewMode === 'tree' ? 1 : 0)) * INDENT_PX + 46}px; padding-top: 3px">
								<p class="text-[12px] text-text-muted/82 leading-[19px] line-clamp-3">{node.text}</p>
							</div>
						</div>
					{:else}
						<!-- Span row -->
						{@const status = spanStatus(s)}
						{@const duration = spanDurationMs(s)}
						{@const model = modelBadge(s)}
						{@const provider = providerBadge(s)}
						{@const tokens = tokenCount(s)}
						{@const cost = costBadge(s)}
						{@const bytes = bytesBadge(s)}
						<div
							class="absolute left-0 right-0 flex items-center text-xs transition-all duration-150 group
								{selectedId === s.id ? 'bg-warning/14 border-l-2 border-l-warning shadow-[inset_0_0_0_1px_rgba(245,158,11,0.22)]' : 'hover:bg-bg-tertiary/70 border-l-2 border-l-transparent'}"
							style="top: {topPx}px; height: {node.height}px"
							role="button"
							tabindex={0}
							aria-label={`Select span ${s.name}`}
							onclick={() => onSelect?.(s)}
							onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect?.(s); } }}
						>
							<!-- Span info -->
							<div class="flex items-center gap-1.5 flex-1 px-2 overflow-hidden min-w-0">
								<!-- Indent + collapse -->
								{#if viewMode === 'tree'}
									<div class="relative flex items-center shrink-0" style="width: {node.depth * INDENT_PX + 20}px">
										{#if node.depth > 0}
											<div class="absolute top-0 bottom-0 border-l border-border/35" style="left: {node.depth * INDENT_PX - 10}px"></div>
											<div class="absolute top-1/2 border-t border-border/35 w-2.5" style="left: {node.depth * INDENT_PX - 10}px"></div>
										{/if}
										<div style="width: {node.depth * INDENT_PX}px"></div>
										{#if node.hasChildren}
										<button
											type="button"
											aria-label={collapsed.has(s.id) ? `Expand ${s.name}` : `Collapse ${s.name}`}
											class="w-5 h-5 flex items-center justify-center text-text-muted hover:text-text transition-colors duration-150 rounded-sm hover:bg-bg-tertiary/80"
											onclick={(e: MouseEvent) => { e.stopPropagation(); toggleCollapse(s.id); }}
										>
												{#if collapsed.has(s.id)}
													<svg class="w-3 h-3" viewBox="0 0 12 12" fill="currentColor"><path d="M4 2l6 4-6 4V2z"/></svg>
												{:else}
													<svg class="w-3 h-3" viewBox="0 0 12 12" fill="currentColor"><path d="M2 4l4 6 4-6H2z"/></svg>
												{/if}
										</button>
									{:else}
										<div class="w-5"></div>
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
							<span class="text-text truncate text-[12px] font-medium max-w-[42%] tracking-[0.01em]">{s.name}</span>

							<!-- Primary metadata -->
							<span class="shrink-0 text-[10px] text-text-secondary bg-bg-tertiary/55 border border-border/55 rounded px-1.5 py-px font-mono">{formatDuration(duration)}</span>

							<!-- Inline badges -->
							{#if showMetadata && model}
								<span class="shrink-0 text-[10px] text-accent bg-accent/10 border border-accent/30 rounded px-1.5 py-px max-w-[30%] truncate">{model}</span>
							{/if}
							{#if showMetadata && provider}
								<span class="shrink-0 text-[10px] text-text-muted bg-bg-tertiary/50 border border-border/50 rounded px-1.5 py-px">{provider}</span>
							{/if}
							{#if showMetadata && tokens}
								<span class="shrink-0 text-text-muted text-[10px] bg-bg-tertiary/45 border border-border/45 rounded px-1.5 py-px">{tokens}</span>
							{/if}
							{#if showMetadata && cost}
								<span class="shrink-0 text-success text-[10px] bg-success/10 border border-success/25 rounded px-1.5 py-px">{cost}</span>
							{/if}
							{#if showMetadata && bytes}
								<span class="shrink-0 text-text-muted text-[10px] bg-bg-tertiary/45 border border-border/45 rounded px-1.5 py-px">{bytes}</span>
							{/if}

								<!-- Collapsed count -->
								{#if node.hasChildren && collapsed.has(s.id)}
									<span class="shrink-0 text-text-muted text-[10px] bg-bg-tertiary rounded px-1.5 py-px">+{node.descendantCount}</span>
								{/if}
							</div>

							<!-- Duration / offset (right side) -->
							<div class="shrink-0 flex flex-col items-end pr-2.5 text-right min-w-14">
								<span class="text-[10px] text-text-muted font-mono">{relativeOffset(s)}</span>
							</div>
						</div>
					{/if}
				{/each}
			</div>
		</div>
	{/if}
</div>
