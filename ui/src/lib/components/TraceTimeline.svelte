<script lang="ts">
	import type { Span } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs } from '$lib/api';
	import SpanKindIcon from './SpanKindIcon.svelte';

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
	const ROW_HEIGHT = 34;
	const OVERSCAN = 10;
	const INDENT_PX = 16;
	const MINIMAP_HEIGHT = 48;

	// ── Colors (raw values for canvas) ─────────────────────────────────
	const CANVAS_COLORS: Record<string, string> = {
		fs_read: '#58a6ff',
		fs_write: '#3fb950',
		llm_call: '#a78bfa',
		custom: '#484f58',
		running: '#d29922',
		failed: '#f85149',
		default: '#484f58'
	};

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

	// Count all descendants of a span
	function countDescendants(id: string): number {
		const children = childIndex.get(id);
		if (!children) return 0;
		let count = children.length;
		for (const c of children) count += countDescendants(c.id);
		return count;
	}

	// ── DFS flat tree (respects collapse) ──────────────────────────────
	const flatTree = $derived.by((): TreeNode[] => {
		const result: TreeNode[] = [];
		function walk(parentId: string | null, depth: number) {
			const children = childIndex.get(parentId);
			if (!children) return;
			// Sort children by start time
			const sorted = [...children].sort(
				(a, b) => new Date(spanStartedAt(a)).getTime() - new Date(spanStartedAt(b)).getTime()
			);
			for (const span of sorted) {
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

	// ── Time range ─────────────────────────────────────────────────────
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
	function kindType(s: Span): string {
		if (!s.kind) return 'custom';
		return s.kind.type;
	}

	function canvasColor(s: Span): string {
		const status = spanStatus(s);
		if (status === 'failed') return CANVAS_COLORS.failed;
		if (status === 'running') return CANVAS_COLORS.running;
		return CANVAS_COLORS[kindType(s)] ?? CANVAS_COLORS.default;
	}

	function modelBadge(s: Span): string | null {
		if (s.kind?.type === 'llm_call') return s.kind.model;
		return null;
	}

	function tokenBadge(s: Span): string | null {
		let inp: number | null = null;
		let out: number | null = null;
		if (s.kind?.type === 'llm_call') {
			inp = s.kind.input_tokens ?? null;
			out = s.kind.output_tokens ?? null;
		}
		if (inp == null && out == null) return null;
		const parts: string[] = [];
		if (inp != null) parts.push(`${inp.toLocaleString()}in`);
		if (out != null) parts.push(`${out.toLocaleString()}out`);
		return parts.join(' / ');
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

	function barPercents(s: Span): { left: number; width: number } {
		const start = new Date(spanStartedAt(s)).getTime();
		const end = spanEndedAt(s) ? new Date(spanEndedAt(s)!).getTime() : Date.now();
		const range = timeRange.max - timeRange.min;
		const left = ((start - timeRange.min) / range) * 100;
		const width = Math.max(((end - start) / range) * 100, 0.5);
		return { left, width };
	}

	function statusDotClass(s: Span): string {
		const st = spanStatus(s);
		if (st === 'running') return 'bg-warning animate-pulse';
		if (st === 'failed') return 'bg-danger';
		return '';
	}

	function barColorClass(s: Span): string {
		const st = spanStatus(s);
		if (st === 'failed') return 'bg-danger';
		if (st === 'running') return 'bg-warning';
		const k = kindType(s);
		if (k === 'fs_read') return 'bg-accent';
		if (k === 'fs_write') return 'bg-success';
		if (k === 'llm_call') return 'bg-purple-400';
		return 'bg-text-muted';
	}

	// ── Minimap canvas ─────────────────────────────────────────────────
	let canvas: HTMLCanvasElement | undefined = $state(undefined);
	let canvasWidth = $state(300);
	let dragging = $state(false);

	function drawMinimap() {
		if (!canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		const dpr = window.devicePixelRatio || 1;
		const w = canvasWidth;
		const h = MINIMAP_HEIGHT;
		canvas.width = w * dpr;
		canvas.height = h * dpr;
		ctx.scale(dpr, dpr);

		// Clear
		ctx.clearRect(0, 0, w, h);

		const rows = flatTree.length;
		if (rows === 0) return;

		const rowH = Math.max(h / rows, 1);
		const range = timeRange.max - timeRange.min;

		// Draw span bars
		for (let i = 0; i < flatTree.length; i++) {
			const node = flatTree[i];
			const s = node.span;
			const start = new Date(spanStartedAt(s)).getTime();
			const end = spanEndedAt(s) ? new Date(spanEndedAt(s)!).getTime() : Date.now();
			const x = ((start - timeRange.min) / range) * w;
			const barW = Math.max(((end - start) / range) * w, 1);
			const y = i * rowH;

			ctx.fillStyle = canvasColor(s);
			ctx.globalAlpha = 0.8;
			ctx.fillRect(x, y, barW, Math.max(rowH - 0.5, 0.5));
		}

		// Draw viewport rectangle
		ctx.globalAlpha = 1;
		const vpStartRow = scrollTop / ROW_HEIGHT;
		const vpEndRow = (scrollTop + containerHeight) / ROW_HEIGHT;
		const vpY = (vpStartRow / rows) * h;
		const vpH = Math.max(((vpEndRow - vpStartRow) / rows) * h, 4);

		ctx.fillStyle = 'rgba(88,166,255,0.08)';
		ctx.fillRect(0, vpY, w, vpH);
		ctx.strokeStyle = 'rgba(88,166,255,0.3)';
		ctx.lineWidth = 1;
		ctx.strokeRect(0.5, vpY + 0.5, w - 1, vpH - 1);

		// Highlight selected span row
		if (selectedId) {
			const selIdx = flatTree.findIndex((n) => n.span.id === selectedId);
			if (selIdx >= 0) {
				const selY = (selIdx / rows) * h;
				ctx.fillStyle = 'rgba(88,166,255,0.25)';
				ctx.fillRect(0, selY, w, Math.max(rowH, 2));
			}
		}
	}

	// Observe canvas container width
	function onCanvasResize(entries: ResizeObserverEntry[]) {
		for (const entry of entries) {
			canvasWidth = entry.contentRect.width;
		}
	}

	let resizeObserver: ResizeObserver | undefined = $state(undefined);

	$effect(() => {
		if (canvas) {
			resizeObserver = new ResizeObserver(onCanvasResize);
			resizeObserver.observe(canvas.parentElement!);
			canvasWidth = canvas.parentElement!.clientWidth;
			return () => resizeObserver?.disconnect();
		}
	});

	$effect(() => {
		// Track dependencies for redraw
		void flatTree;
		void timeRange;
		void scrollTop;
		void containerHeight;
		void selectedId;
		void canvasWidth;
		drawMinimap();
	});

	// ── Minimap pointer interactions ───────────────────────────────────
	function minimapSeek(clientY: number) {
		if (!canvas || !scrollContainer) return;
		const rect = canvas.getBoundingClientRect();
		const y = clientY - rect.top;
		const ratio = y / MINIMAP_HEIGHT;
		const targetRow = Math.floor(ratio * flatTree.length);
		const clampedRow = Math.max(0, Math.min(flatTree.length - 1, targetRow));

		// Select the span at target row
		if (flatTree[clampedRow]) {
			onSelect?.(flatTree[clampedRow].span);
		}

		// Scroll to center that row
		const targetScroll = clampedRow * ROW_HEIGHT - containerHeight / 2 + ROW_HEIGHT / 2;
		scrollContainer.scrollTop = Math.max(0, Math.min(targetScroll, totalHeight - containerHeight));
	}

	function onMinimapPointerDown(e: PointerEvent) {
		dragging = true;
		(e.target as HTMLCanvasElement).setPointerCapture(e.pointerId);
		minimapSeek(e.clientY);
	}

	function onMinimapPointerMove(e: PointerEvent) {
		if (!dragging) return;
		minimapSeek(e.clientY);
	}

	function onMinimapPointerUp(e: PointerEvent) {
		dragging = false;
		(e.target as HTMLCanvasElement).releasePointerCapture(e.pointerId);
	}
</script>

<div class="flex flex-col h-full min-h-0">
	<!-- Header controls -->
	<div class="flex items-center justify-between px-3 py-1.5 border-b border-border text-xs text-text-muted shrink-0">
		<div class="flex items-center gap-3">
			<span class="uppercase tracking-wide">Span</span>
			<span class="text-border">|</span>
			<span class="uppercase tracking-wide">Timeline</span>
			<span class="text-border">|</span>
			<span class="uppercase tracking-wide">Duration</span>
		</div>
		<div class="flex items-center gap-2">
			<button class="hover:text-text transition-colors" onclick={expandAll}>expand</button>
			<span class="text-border">/</span>
			<button class="hover:text-text transition-colors" onclick={collapseAll}>collapse</button>
		</div>
	</div>

	<!-- Minimap -->
	<div class="shrink-0 border-b border-border bg-bg-secondary cursor-pointer" style="height: {MINIMAP_HEIGHT}px">
		<canvas
			bind:this={canvas}
			class="w-full block"
			style="height: {MINIMAP_HEIGHT}px"
			onpointerdown={onMinimapPointerDown}
			onpointermove={onMinimapPointerMove}
			onpointerup={onMinimapPointerUp}
		></canvas>
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
				{@const bar = barPercents(s)}
				{@const model = modelBadge(s)}
				{@const tokens = tokenBadge(s)}
				{@const bytes = bytesBadge(s)}
				{@const dotClass = statusDotClass(s)}
				<button
					class="absolute left-0 right-0 flex items-center text-xs transition-colors hover:bg-bg-tertiary
						{selectedId === s.id ? 'bg-bg-tertiary' : ''}"
					style="top: {idx * ROW_HEIGHT}px; height: {ROW_HEIGHT}px"
					onclick={() => onSelect?.(s)}
				>
					<!-- Span info column -->
					<div class="flex items-center gap-1 shrink-0 px-2 overflow-hidden" style="width: 40%; min-width: 200px;">
						<!-- Indent + collapse toggle -->
						<div class="flex items-center shrink-0" style="width: {node.depth * INDENT_PX + 20}px">
							<div style="width: {node.depth * INDENT_PX}px"></div>
							{#if node.hasChildren}
								<!-- svelte-ignore node_invalid_placement_ssr -->
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<span
									role="switch"
									aria-checked={!collapsed.has(s.id)}
									tabindex={-1}
									class="w-5 h-5 flex items-center justify-center text-text-muted hover:text-text transition-colors cursor-pointer"
									onclick={(e: MouseEvent) => { e.stopPropagation(); toggleCollapse(s.id); }}
								>
									{#if collapsed.has(s.id)}
										<svg class="w-3 h-3" viewBox="0 0 12 12" fill="currentColor"><path d="M4 2l6 4-6 4V2z"/></svg>
									{:else}
										<svg class="w-3 h-3" viewBox="0 0 12 12" fill="currentColor"><path d="M2 4l4 6 4-6H2z"/></svg>
									{/if}
								</span>
							{:else}
								<div class="w-5"></div>
							{/if}
						</div>

						<!-- Icon -->
						<div class="shrink-0">
							<SpanKindIcon span={s} />
						</div>

						<!-- Name -->
						<span class="text-text truncate">{s.name}</span>

						<!-- Status dot (running/failed only) -->
						{#if dotClass}
							<span class="w-1.5 h-1.5 rounded-full shrink-0 {dotClass}"></span>
						{/if}

						<!-- Badges -->
						{#if model}
							<span class="shrink-0 text-purple-400 bg-purple-400/10 border border-purple-400/20 rounded px-1 py-0 text-[10px] leading-tight truncate max-w-20">{model}</span>
						{/if}
						{#if tokens}
							<span class="shrink-0 text-text-muted bg-bg-tertiary rounded px-1 py-0 text-[10px] leading-tight">{tokens}</span>
						{/if}
						{#if bytes}
							<span class="shrink-0 text-text-muted bg-bg-tertiary rounded px-1 py-0 text-[10px] leading-tight">{bytes}</span>
						{/if}

						<!-- Collapsed descendant count -->
						{#if node.hasChildren && collapsed.has(s.id)}
							<span class="shrink-0 text-text-muted bg-bg-tertiary rounded px-1 py-0 text-[10px] leading-tight">+{node.descendantCount}</span>
						{/if}
					</div>

					<!-- Timeline bar column -->
					<div class="flex-1 relative h-5 mx-2">
						<div
							class="absolute top-1 h-3 rounded {barColorClass(s)} opacity-80"
							style="left: {bar.left}%; width: {bar.width}%"
						></div>
					</div>

					<!-- Duration column -->
					<div class="shrink-0 w-16 text-right pr-3 text-text-muted font-mono">
						{duration !== null ? `${duration}ms` : '...'}
					</div>
				</button>
			{/each}
		</div>
	</div>
</div>
