<script lang="ts">
	import type { Span } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs } from '$lib/api';
	import SpanKindIcon from './SpanKindIcon.svelte';

	let {
		spans,
		selectedId = null,
		onSelect,
		searchQuery = ''
	}: {
		spans: Span[];
		selectedId?: string | null;
		onSelect?: (span: Span) => void;
		searchQuery?: string;
	} = $props();

	// ── Constants ──────────────────────────────────────────────────────
	const ROW_HEIGHT = 36;
	const INDENT_PX = 16;
	const LABEL_WIDTH = 260; // px for the left label column
	const MIN_BAR_PX = 3;

	// ── Zoom ──────────────────────────────────────────────────────────
	let zoom = $state(1);
	let scrollContainer: HTMLDivElement | undefined = $state(undefined);

	function zoomIn() { zoom = Math.min(zoom * 1.5, 20); }
	function zoomOut() { zoom = Math.max(zoom / 1.5, 0.5); }
	function zoomReset() { zoom = 1; }

	// ── Child index ───────────────────────────────────────────────────
	const childIndex = $derived.by(() => {
		const idx = new Map<string | null, Span[]>();
		for (const s of spans) {
			const key = s.parent_id ?? null;
			let arr = idx.get(key);
			if (!arr) { arr = []; idx.set(key, arr); }
			arr.push(s);
		}
		return idx;
	});

	// ── Time range ────────────────────────────────────────────────────
	const timeRange = $derived.by(() => {
		if (spans.length === 0) return { min: 0, max: 1, duration: 1 };
		const starts = spans.map((s) => new Date(spanStartedAt(s)).getTime());
		const ends = spans.map((s) => {
			const end = spanEndedAt(s);
			return end ? new Date(end).getTime() : Date.now();
		});
		const min = Math.min(...starts);
		const max = Math.max(...ends);
		const duration = max === min ? 1 : max - min;
		return { min, max, duration };
	});

	// ── Flatten spans into rows (tree order) ──────────────────────────
	interface TimelineRow {
		span: Span;
		depth: number;
		startPct: number;  // 0-100
		widthPct: number;  // 0-100
		durationMs: number | null;
	}

	const rows = $derived.by((): TimelineRow[] => {
		const query = searchQuery.toLowerCase().trim();
		const result: TimelineRow[] = [];

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

				const startMs = new Date(spanStartedAt(span)).getTime() - timeRange.min;
				const endMs = (spanEndedAt(span) ? new Date(spanEndedAt(span)!).getTime() : Date.now()) - timeRange.min;
				const durMs = endMs - startMs;
				const startPct = (startMs / timeRange.duration) * 100;
				const widthPct = Math.max((durMs / timeRange.duration) * 100, 0.1);

				result.push({
					span,
					depth,
					startPct,
					widthPct,
					durationMs: spanDurationMs(span),
				});

				walk(span.id, depth + 1);
			}
		}
		walk(null, 0);
		return result;
	});

	// ── Time axis ticks ───────────────────────────────────────────────
	const ticks = $derived.by(() => {
		const totalMs = timeRange.duration;
		const tickCount = Math.min(10, Math.max(4, Math.floor(zoom * 6)));
		const result: { pct: number; label: string }[] = [];

		for (let i = 0; i <= tickCount; i++) {
			const pct = (i / tickCount) * 100;
			const ms = (i / tickCount) * totalMs;
			result.push({ pct, label: formatTickLabel(ms) });
		}
		return result;
	});

	// ── Tooltip ───────────────────────────────────────────────────────
	let tooltip: { x: number; y: number; span: Span } | null = $state(null);

	function showTooltip(e: MouseEvent, span: Span) {
		tooltip = { x: e.clientX, y: e.clientY, span };
	}

	function hideTooltip() {
		tooltip = null;
	}

	// ── Helpers ────────────────────────────────────────────────────────
	function kindLabel(s: Span): string {
		if (!s.kind) return 'unknown';
		if (s.kind.type === 'custom') return s.kind.kind;
		return s.kind.type;
	}

	function barColor(s: Span): string {
		const status = spanStatus(s);
		if (status === 'failed') return 'bg-danger/80 border-danger';
		if (status === 'running') return 'bg-warning/65 border-warning animate-pulse';
		if (!s.kind) return 'bg-text-muted/30 border-text-muted/50';
		switch (s.kind.type) {
			case 'llm_call': return 'bg-accent/70 border-accent';
			case 'fs_read': return 'bg-emerald-500/55 border-emerald-500/80';
			case 'fs_write': return 'bg-emerald-500/55 border-emerald-500/80';
			case 'custom': return 'bg-text-muted/35 border-text-muted/50';
			default: return 'bg-text-muted/35 border-text-muted/50';
		}
	}

	function formatDuration(ms: number | null): string {
		if (ms === null) return '...';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function formatTickLabel(ms: number): string {
		if (ms === 0) return '0';
		if (ms < 1000) return `${Math.round(ms)}ms`;
		if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
		return `${(ms / 60000).toFixed(1)}m`;
	}

	function tokenCount(s: Span): string | null {
		if (s.kind?.type !== 'llm_call') return null;
		const inp = s.kind.input_tokens ?? null;
		const out = s.kind.output_tokens ?? null;
		if (inp == null && out == null) return null;
		return ((inp ?? 0) + (out ?? 0)).toLocaleString();
	}

	function costBadge(s: Span): string | null {
		if (s.kind?.type === 'llm_call' && s.kind.cost != null) return `$${s.kind.cost.toFixed(4)}`;
		return null;
	}

	function statusDotClass(s: Span): string {
		const st = spanStatus(s);
		if (st === 'running') return 'bg-warning animate-pulse';
		if (st === 'failed') return 'bg-danger';
		return 'bg-success';
	}
</script>

<div class="flex flex-col h-full min-h-0 bg-bg/10">
	<!-- Toolbar -->
	<div class="flex items-center gap-2 px-3 py-2 border-b border-border/70 shrink-0 bg-bg-secondary/45">
		<span class="text-[11px] text-text-muted">{rows.length} spans</span>
		<div class="flex-1"></div>
		<div class="flex items-center gap-0.5 text-[10px]">
			<button class="px-1.5 py-0.5 text-text-muted hover:text-text rounded hover:bg-bg-tertiary transition-colors" onclick={zoomOut}>-</button>
			<button class="px-1.5 py-0.5 text-text-muted hover:text-text rounded hover:bg-bg-tertiary transition-colors text-[9px]" onclick={zoomReset}>{Math.round(zoom * 100)}%</button>
			<button class="px-1.5 py-0.5 text-text-muted hover:text-text rounded hover:bg-bg-tertiary transition-colors" onclick={zoomIn}>+</button>
		</div>
	</div>

	<!-- Time axis -->
	<div class="flex shrink-0 border-b border-border/70 bg-bg-secondary/35" style="height: 26px">
		<div class="shrink-0 sticky left-0 z-20 bg-bg-secondary/85 border-r border-border/60" style="width: {LABEL_WIDTH}px"></div>
		<div class="flex-1 relative min-w-0 overflow-hidden">
			<div class="relative h-full" style="width: {zoom * 100}%">
				{#each ticks as tick}
					<div class="absolute top-0 bottom-0 border-l border-border/40" style="left: {tick.pct}%">
						<span class="absolute top-1 left-1 text-[9px] text-text-muted/80 whitespace-nowrap">{tick.label}</span>
					</div>
				{/each}
			</div>
		</div>
	</div>

	<!-- Rows -->
	<div
		bind:this={scrollContainer}
		class="flex-1 min-h-0 overflow-auto"
		onwheel={(e) => {
			if (e.ctrlKey || e.metaKey) {
				e.preventDefault();
				if (e.deltaY < 0) zoomIn();
				else zoomOut();
			}
		}}
	>
		{#if rows.length === 0}
			<div class="text-text-muted text-xs text-center py-12">No spans</div>
		{:else}
			{#each rows as row (row.span.id)}
				{@const s = row.span}
				{@const tokens = tokenCount(s)}
				{@const cost = costBadge(s)}
				<button
					class="flex items-center w-full transition-colors group border-b border-border/25
						{selectedId === s.id ? 'bg-accent/10' : 'hover:bg-bg-tertiary/35'}"
					style="height: {ROW_HEIGHT}px"
					onclick={() => onSelect?.(s)}
				>
					<!-- Label column -->
					<div
						class="shrink-0 sticky left-0 z-10 flex items-center gap-1.5 px-2 overflow-hidden border-r border-border/40
							{selectedId === s.id ? 'bg-bg-secondary/95' : 'bg-bg/95 group-hover:bg-bg-secondary/88'}"
						style="width: {LABEL_WIDTH}px; padding-left: {row.depth * INDENT_PX + 8}px"
					>
						<span class="w-1.5 h-1.5 rounded-full shrink-0 {statusDotClass(s)}"></span>
						<div class="shrink-0 opacity-60"><SpanKindIcon span={s} /></div>
						<span class="text-[11px] text-text truncate font-medium">{s.name}</span>
					</div>

					<!-- Bar area -->
					<div class="flex-1 relative min-w-0 h-full">
						<div class="relative h-full" style="width: {zoom * 100}%">
							<!-- Grid lines -->
							{#each ticks as tick}
								<div class="absolute top-0 bottom-0 border-l border-border/20" style="left: {tick.pct}%"></div>
							{/each}

							<!-- Bar -->
							<div
								class="absolute top-1 bottom-1 rounded-sm border {barColor(s)} flex items-center overflow-hidden cursor-pointer
									{selectedId === s.id ? 'ring-1 ring-accent ring-offset-1 ring-offset-bg' : ''}"
								style="left: {row.startPct}%; width: max({MIN_BAR_PX}px, {row.widthPct}%)"
								role="button"
								tabindex={-1}
								onmouseenter={(e) => showTooltip(e, s)}
								onmouseleave={hideTooltip}
							>
								<!-- Inline label (only when bar is wide enough) -->
								<span class="text-[9px] text-text font-medium truncate px-1.5 whitespace-nowrap opacity-90">
									{formatDuration(row.durationMs)}
									{#if tokens}
										<span class="text-text-muted/70 ml-1">{tokens}tok</span>
									{/if}
									{#if cost}
										<span class="text-success/70 ml-1">{cost}</span>
									{/if}
								</span>
							</div>
						</div>
					</div>
				</button>
			{/each}
		{/if}
	</div>
</div>

<!-- Tooltip -->
{#if tooltip}
	{@const s = tooltip.span}
	<div
		class="fixed z-50 pointer-events-none bg-bg-secondary border border-border rounded-lg shadow-xl px-3 py-2 text-xs space-y-1 max-w-72"
		style="left: {tooltip.x + 12}px; top: {tooltip.y + 12}px"
	>
		<div class="font-semibold text-text">{s.name}</div>
		<div class="flex items-center gap-2 text-text-muted">
			<span>{kindLabel(s)}</span>
			{#if s.kind?.type === 'llm_call'}
				<span class="text-purple-400">{s.kind.model}</span>
			{/if}
		</div>
		<div class="flex items-center gap-3">
			<span class="text-text-secondary">{formatDuration(spanDurationMs(s))}</span>
			{#if tokenCount(s)}
				<span class="text-text-muted">{tokenCount(s)} tokens</span>
			{/if}
			{#if costBadge(s)}
				<span class="text-success">{costBadge(s)}</span>
			{/if}
		</div>
		{#if spanStatus(s) === 'failed'}
			<div class="text-danger text-[10px]">Failed</div>
		{/if}
	</div>
{/if}
