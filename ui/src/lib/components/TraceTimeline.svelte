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
	const SPAN_ROW_HEIGHT = 30;
	const PREVIEW_LINE_HEIGHT = 16;
	const PREVIEW_PADDING = 4;
	const OVERSCAN = 10;
	const INDENT_PX = 16;

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

	// ── Time range for gantt bar positioning ──────────────────────────
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

	// Per-model color palette for LLM calls (vibrant, distinct)
	const MODEL_COLORS: [string, string][] = [
		['claude',    '#c084fc'], // purple
		['gpt-4o',    '#fb923c'], // orange
		['gpt-4',     '#f97316'], // deep orange
		['gpt-3',     '#fbbf24'], // amber
		['o1',        '#a78bfa'], // violet
		['o3',        '#818cf8'], // indigo
		['gemini',    '#34d399'], // emerald
		['llama',     '#f472b6'], // pink
		['mistral',   '#38bdf8'], // sky
		['deepseek',  '#2dd4bf'], // teal
		['command',   '#fb7185'], // rose
		['qwen',      '#a3e635'], // lime
	];
	const LLM_FALLBACK_COLORS = ['#ec4899', '#8b5cf6', '#f59e0b', '#06b6d4', '#10b981', '#ef4444', '#6366f1'];
	let modelColorCache = new Map<string, string>();

	function colorForModel(model: string): string {
		const cached = modelColorCache.get(model);
		if (cached) return cached;
		const lower = model.toLowerCase();
		for (const [prefix, color] of MODEL_COLORS) {
			if (lower.includes(prefix)) {
				modelColorCache.set(model, color);
				return color;
			}
		}
		// Hash-based fallback for unknown models
		let hash = 0;
		for (let i = 0; i < model.length; i++) hash = (hash * 31 + model.charCodeAt(i)) | 0;
		const color = LLM_FALLBACK_COLORS[Math.abs(hash) % LLM_FALLBACK_COLORS.length];
		modelColorCache.set(model, color);
		return color;
	}

	function spanColor(s: Span): string {
		const kind = kindType(s);
		if (kind === 'llm_call') {
			const model = s.kind?.type === 'llm_call' ? (s.kind.model ?? '') : '';
			return model ? colorForModel(model) : '#ec4899';
		}
		if (kind === 'tool_call') return '#f59e0b';   // amber
		if (kind === 'fs_read') return '#38bdf8';      // sky
		if (kind === 'fs_write') return '#34d399';     // emerald
		return '#6366f1';                               // indigo for custom
	}

	function barProps(s: Span): { left: string; width: string; color: string } {
		const start = new Date(spanStartedAt(s)).getTime();
		const endStr = spanEndedAt(s);
		const end = endStr ? new Date(endStr).getTime() : Date.now();
		const range = timeRange.max - timeRange.min;
		const leftPct = range > 0 ? ((start - timeRange.min) / range) * 100 : 0;
		const widthPct = range > 0 ? Math.max(1.5, ((end - start) / range) * 100) : 100;

		return { left: `${leftPct}%`, width: `${widthPct}%`, color: spanColor(s) };
	}

	function statusDotClass(s: Span): string {
		const st = spanStatus(s);
		if (st === 'running') return 'bg-warning animate-pulse';
		if (st === 'failed') return 'bg-danger';
		return ''; // use inline color instead
	}

	function statusDotStyle(s: Span): string {
		const st = spanStatus(s);
		if (st === 'running' || st === 'failed') return '';
		return `background-color: ${spanColor(s)}`;
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

	// ── Aggregate stats ───────────────────────────────────────────────
	const totalStats = $derived.by(() => {
		let tokens = 0;
		let cost = 0;
		let depth = 0;
		for (const s of spans) {
			if (s.kind?.type === 'llm_call') {
				tokens += (s.kind.input_tokens ?? 0) + (s.kind.output_tokens ?? 0);
				cost += s.kind.cost ?? 0;
			}
			// rough depth calc
			const d = flatTree.find(n => n.type === 'span' && n.span.id === s.id);
			if (d && d.depth > depth) depth = d.depth;
		}
		const dur = timeRange.max - timeRange.min;
		return { tokens, cost, durationMs: dur, maxDepth: depth };
	});

	// ── Mini waterfall bars ───────────────────────────────────────────
	const waterfallBars = $derived.by(() => {
		const range = timeRange.max - timeRange.min;
		if (range <= 0) return [];
		return spans.map(s => {
			const start = new Date(spanStartedAt(s)).getTime();
			const endStr = spanEndedAt(s);
			const end = endStr ? new Date(endStr).getTime() : Date.now();
			const leftPct = ((start - timeRange.min) / range) * 100;
			const widthPct = Math.max(0.5, ((end - start) / range) * 100);
			return { span: s, left: leftPct, width: widthPct, color: spanColor(s) };
		});
	});

	// ── Waterfall time ticks ──────────────────────────────────────────
	const waterfallTicks = $derived.by(() => {
		const dur = timeRange.max - timeRange.min;
		if (dur <= 0) return [];
		const count = 5;
		const step = dur / count;
		const ticks: { pct: number; label: string }[] = [];
		for (let i = 0; i <= count; i++) {
			const ms = step * i;
			const pct = (ms / dur) * 100;
			let label: string;
			if (ms === 0) label = '0';
			else if (ms < 1000) label = `${Math.round(ms)}ms`;
			else if (ms < 60000) label = `${(ms / 1000).toFixed(1)}s`;
			else label = `${(ms / 60000).toFixed(1)}m`;
			ticks.push({ pct, label });
		}
		return ticks;
	});

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
	<div class="flex items-center gap-1.5 px-2 py-1 border-b border-border/40 shrink-0">
		<span class="text-[10px] uppercase tracking-[0.08em] text-text-muted/70 tabular-nums">{flatTree.filter(n => n.type === 'span').length} spans</span>

		<div class="flex-1"></div>

		{#if viewMode === 'tree'}
			<div class="flex items-center gap-0 text-[10px] text-text-muted">
				<button class="hover:text-text transition-colors px-1" onclick={expandAll}>expand</button>
				<span class="text-border/40">/</span>
				<button class="hover:text-text transition-colors px-1" onclick={collapseAll}>collapse</button>
			</div>
		{/if}

		<!-- View mode toggle -->
		<div class="flex items-center bg-bg-tertiary/30 border border-border/40 rounded-md text-[10px] p-px">
			{#each (['tree', 'flat', 'timeline', 'reader'] as const) as mode}
				<button
					class="px-1.5 py-px rounded transition-colors capitalize {viewMode === mode ? 'bg-accent/15 text-accent' : 'text-text-muted hover:text-text'}"
					onclick={() => viewMode = mode}
				>{mode === 'timeline' ? 'Time' : mode}</button>
			{/each}
		</div>
	</div>

	{#if viewMode === 'timeline'}
		<TimelineView {spans} {selectedId} {onSelect} {searchQuery} />
	{:else if viewMode === 'reader'}
		<ReaderView {spans} {selectedId} {onSelect} {searchQuery} />
	{:else}
		<!-- Mini waterfall overview -->
		{#if waterfallBars.length > 0}
			<div class="shrink-0 border-b border-border/30 bg-bg-secondary/20">
				<!-- Time axis labels -->
				<div class="relative h-4 px-2">
					{#each waterfallTicks as tick}
						<span class="absolute text-[8px] text-text-muted/50 font-mono" style="left: {tick.pct}%; transform: translateX(-50%)">{tick.label}</span>
					{/each}
				</div>
				<!-- Bars -->
				<div class="relative h-10 px-2 mb-1">
					<!-- Grid lines -->
					{#each waterfallTicks as tick}
						<div class="absolute top-0 bottom-0 border-l border-border/15" style="left: calc({tick.pct}% + 8px)"></div>
					{/each}
					<!-- Span bars stacked vertically by index -->
					{#each waterfallBars as bar, i}
						{@const barH = Math.min(6, Math.max(2, 36 / Math.max(waterfallBars.length, 1)))}
						<div
							class="absolute rounded-[1px] cursor-pointer hover:brightness-125 transition-all"
							style="left: {bar.left}%; width: max(2px, {bar.width}%); height: {barH}px; top: {(i * (barH + 1))}px; background-color: {bar.color}; opacity: 0.85"
							role="button"
							tabindex={-1}
							onclick={() => onSelect?.(bar.span)}
						></div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Summary stats -->
		<div class="flex items-center gap-3 px-2 py-1 border-b border-border/30 text-[10px] text-text-muted shrink-0">
			<span class="font-mono tabular-nums">{spans.length} spans</span>
			<span class="text-border/30">·</span>
			<span class="font-mono tabular-nums">{totalStats.durationMs < 1000 ? `${Math.round(totalStats.durationMs)}ms` : `${(totalStats.durationMs / 1000).toFixed(2)}s`}</span>
			{#if totalStats.tokens > 0}
				<span class="text-border/30">·</span>
				<span class="font-mono tabular-nums">{totalStats.tokens > 999 ? `${(totalStats.tokens / 1000).toFixed(1)}K` : totalStats.tokens} tok</span>
			{/if}
			{#if totalStats.cost > 0}
				<span class="text-border/30">·</span>
				<span class="font-mono tabular-nums text-success/70">${totalStats.cost.toFixed(4)}</span>
			{/if}
			{#if totalStats.maxDepth > 0}
				<span class="text-border/30">·</span>
				<span>depth {totalStats.maxDepth}</span>
			{/if}
		</div>

		<!-- Virtual scroll area (tree/flat modes) -->
		<div
			bind:this={scrollContainer}
			bind:clientHeight={containerHeight}
			class="flex-1 min-h-0 overflow-y-auto"
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
							class="absolute left-0 right-0 cursor-pointer motion-row
								{selectedId === s.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'border-l-2 border-l-transparent hover:bg-bg-tertiary/30'}"
							style="top: {topPx}px; height: {node.height}px"
							role="button"
							tabindex={0}
							aria-label={`Select span ${s.name}`}
							onclick={() => onSelect?.(s)}
							onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') onSelect?.(s); }}
						>
							<div class="overflow-hidden pr-2" style="padding-left: {(node.depth + (viewMode === 'tree' ? 1 : 0)) * INDENT_PX + 36}px; padding-top: 2px">
								<p class="text-[11px] text-text-muted/70 leading-[16px] line-clamp-3">{node.text}</p>
							</div>
						</div>
					{:else}
						<!-- Span row -->
						{@const status = spanStatus(s)}
						{@const duration = spanDurationMs(s)}
						{@const model = modelBadge(s)}
						{@const cost = costBadge(s)}
						{@const tokens = tokenCount(s)}
						{@const bytes = bytesBadge(s)}
						{@const bar = barProps(s)}
						<div
							class="absolute left-0 right-0 flex items-center text-xs motion-row group border-b border-border/10
								{selectedId === s.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'hover:bg-bg-tertiary/30 border-l-2 border-l-transparent'}"
							style="top: {topPx}px; height: {node.height}px"
							role="button"
							tabindex={0}
							aria-label={`Select span ${s.name}`}
							onclick={() => onSelect?.(s)}
							onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect?.(s); } }}
						>
							<!-- Tree indent + status + name -->
							<div class="flex items-center gap-1 pl-1.5 overflow-hidden min-w-0 flex-1">
								{#if viewMode === 'tree'}
									<div class="relative flex items-center shrink-0" style="width: {node.depth * INDENT_PX + 16}px">
										{#if node.depth > 0}
											<div class="absolute top-0 bottom-0 border-l border-border/15" style="left: {node.depth * INDENT_PX - 8}px"></div>
											<div class="absolute top-1/2 border-t border-border/15 w-1.5" style="left: {node.depth * INDENT_PX - 8}px"></div>
										{/if}
										<div style="width: {node.depth * INDENT_PX}px"></div>
										{#if node.hasChildren}
											<button
												type="button"
												aria-label={collapsed.has(s.id) ? `Expand ${s.name}` : `Collapse ${s.name}`}
												class="w-4 h-4 flex items-center justify-center text-text-muted hover:text-text transition-colors rounded-sm"
												onclick={(e: MouseEvent) => { e.stopPropagation(); toggleCollapse(s.id); }}
											>
												{#if collapsed.has(s.id)}
													<svg class="w-2.5 h-2.5" viewBox="0 0 12 12" fill="currentColor"><path d="M4 2l6 4-6 4V2z"/></svg>
												{:else}
													<svg class="w-2.5 h-2.5" viewBox="0 0 12 12" fill="currentColor"><path d="M2 4l4 6 4-6H2z"/></svg>
												{/if}
											</button>
										{:else}
											<div class="w-4"></div>
										{/if}
									</div>
								{/if}

								<span class="w-1.5 h-1.5 rounded-full shrink-0 {statusDotClass(s)}" style={statusDotStyle(s)}></span>

								<span class="text-text truncate text-[11px] font-medium">{s.name}</span>

								{#if node.hasChildren && collapsed.has(s.id)}
									<span class="shrink-0 text-text-muted/50 text-[9px] bg-bg-tertiary/50 rounded px-1 py-px">+{node.descendantCount}</span>
								{/if}
							</div>

							<!-- Right: duration + model/kind tag + tokens + cost -->
							<div class="flex items-center gap-1.5 shrink-0 pr-2.5">
								<span class="text-[10px] text-text-muted/60 font-mono tabular-nums">{formatDuration(duration)}</span>

								{#if model}
									<span class="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-semibold truncate max-w-[150px]"
										style="background-color: {spanColor(s)}; color: white"
									>{model}</span>
								{:else if kindType(s) !== 'custom'}
									<span class="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-medium truncate"
										style="background-color: {spanColor(s)}20; color: {spanColor(s)}; border: 1px solid {spanColor(s)}30"
									>{kindType(s).replace('_', ' ')}</span>
								{/if}

								{#if showMetadata && s.kind?.type === 'llm_call' && s.kind.provider}
									<span class="text-[9px] text-text-muted/50">{s.kind.provider}</span>
								{/if}

								{#if tokens}
									<span class="text-[10px] text-text-muted/50 font-mono tabular-nums">{tokens}</span>
								{/if}

								{#if cost}
									<span class="text-[10px] font-mono tabular-nums" style="color: {spanColor(s)}">{cost}</span>
								{/if}

								{#if bytes}
									<span class="text-[10px] text-text-muted/50 font-mono tabular-nums">{bytes}</span>
								{/if}

								{#if node.hasChildren}
									<svg class="w-3 h-3 text-text-muted/30 shrink-0 group-hover:text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" /></svg>
								{/if}
							</div>
						</div>
					{/if}
				{/each}
			</div>
		</div>
	{/if}
</div>
