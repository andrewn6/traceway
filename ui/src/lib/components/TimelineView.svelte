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
	const ROW_HEIGHT = 40;
	const INDENT_PX = 16;
	const LABEL_WIDTH = 340;
	const MIN_BAR_PX = 3;

	// ── Zoom ──────────────────────────────────────────────────────────
	let zoom = $state(1);
	let scrollContainer: HTMLDivElement | undefined = $state(undefined);
	let viewportScrollLeft = $state(0);
	type FilterMode = 'all' | 'llm' | 'file' | 'custom' | 'failed';
	let filterMode: FilterMode = $state('all');
	let showMetadata = $state(true);

	function zoomIn() { zoom = Math.min(zoom * 1.5, 16); }
	function zoomOut() { zoom = Math.max(zoom / 1.5, 0.5); }
	function zoomReset() { zoom = 1; }

	function onViewportScroll(e: Event) {
		const el = e.currentTarget as HTMLDivElement;
		viewportScrollLeft = el.scrollLeft;
	}

	function matchesFilter(s: Span): boolean {
		if (filterMode === 'all') return true;
		if (filterMode === 'failed') return spanStatus(s) === 'failed';
		if (!s.kind) return false;
		if (filterMode === 'llm') return s.kind.type === 'llm_call';
		if (filterMode === 'file') return s.kind.type === 'fs_read' || s.kind.type === 'fs_write';
		if (filterMode === 'custom') return s.kind.type === 'custom';
		return true;
	}

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
				if ((query && !span.name.toLowerCase().includes(query) && !kindLabel(span).toLowerCase().includes(query)) || !matchesFilter(span)) {
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

	const maxDepth = $derived.by(() => rows.reduce((max, r) => Math.max(max, r.depth), 0));

	// ── Time axis ticks ───────────────────────────────────────────────
	const ticks = $derived.by(() => {
		const totalMs = timeRange.duration;
		const tickCount = Math.min(12, Math.max(4, Math.floor(zoom * 5)));
		const result: { pct: number; label: string }[] = [];
		const step = pickTickStep(totalMs, tickCount);

		for (let ms = 0; ms <= totalMs + 1; ms += step) {
			const pct = (ms / totalMs) * 100;
			result.push({ pct, label: formatTickLabel(ms) });
		}

		if (result.length === 0 || result[result.length - 1].pct < 99.9) {
			result.push({ pct: 100, label: formatTickLabel(totalMs) });
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
		if (status === 'failed') return 'bg-danger/75 border-danger/90';
		if (status === 'running') return 'bg-warning/70 border-warning/90 animate-pulse';
		if (!s.kind) return 'bg-text-muted/45 border-text-muted/70';
		switch (s.kind.type) {
			case 'llm_call': return 'bg-accent/80 border-accent';
			case 'fs_read': return 'bg-emerald-500/65 border-emerald-400';
			case 'fs_write': return 'bg-teal-500/65 border-teal-300';
			case 'custom': return 'bg-slate-500/60 border-slate-300/70';
			default: return 'bg-text-muted/45 border-text-muted/70';
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
		if (ms < 10000) return `${(ms / 1000).toFixed(1)}s`;
		if (ms < 60000) return `${Math.round(ms / 1000)}s`;
		return `${(ms / 60000).toFixed(1)}m`;
	}

	function pickTickStep(totalMs: number, targetTicks: number): number {
		if (totalMs <= 0) return 1;
		const rough = totalMs / targetTicks;
		const magnitude = 10 ** Math.floor(Math.log10(rough));
		const residual = rough / magnitude;
		if (residual <= 1) return 1 * magnitude;
		if (residual <= 2) return 2 * magnitude;
		if (residual <= 5) return 5 * magnitude;
		return 10 * magnitude;
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

	function providerBadge(s: Span): string | null {
		if (s.kind?.type === 'llm_call') return s.kind.provider ?? null;
		return null;
	}

	function modelBadge(s: Span): string | null {
		if (s.kind?.type === 'llm_call') return s.kind.model;
		return null;
	}

	function compactTokens(s: string | null): string | null {
		if (!s) return null;
		const n = Number(s.replace(/,/g, ''));
		if (!Number.isFinite(n)) return s;
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 10_000 ? 0 : 1)}K`;
		return `${n}`;
	}

	function statusDotClass(s: Span): string {
		const st = spanStatus(s);
		if (st === 'running') return 'bg-warning animate-pulse';
		if (st === 'failed') return 'bg-danger';
		return 'bg-success';
	}

	function showBarLabel(startPct: number, widthPct: number): boolean {
		void startPct;
		return widthPct * zoom > 8;
	}
</script>

<div class="flex flex-col h-full min-h-0 bg-bg/10 text-[13px] min-w-0">
	<!-- Toolbar -->
	<div class="flex items-center gap-2 px-3 py-2.5 border-b border-border/70 shrink-0 bg-bg-secondary/45 flex-wrap min-w-0">
		<span class="text-[12px] text-text-muted">{rows.length} spans</span>
		<span class="text-[11px] text-text-muted/70">depth {maxDepth + 1}</span>
		<div class="flex-1"></div>
		<div class="flex items-center gap-0.5 text-[11px] shrink-0">
			<button class="px-2 py-0.5 text-text-muted hover:text-text rounded hover:bg-bg-tertiary transition-colors" onclick={zoomOut}>-</button>
			<button class="px-2 py-0.5 text-text-muted hover:text-text rounded hover:bg-bg-tertiary transition-colors text-[11px]" onclick={zoomReset}>{Math.round(zoom * 100)}%</button>
			<button class="px-2 py-0.5 text-text-muted hover:text-text rounded hover:bg-bg-tertiary transition-colors" onclick={zoomIn}>+</button>
		</div>
	</div>

	<!-- Control strip -->
	<div class="flex items-center gap-2 px-3 py-2 border-b border-border/65 shrink-0 bg-bg-secondary/35 flex-wrap min-w-0">
		<span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-accent/35 bg-accent/10 text-accent text-[12px]">
			<svg class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor"><path d="M3 4h14v2H3V4Zm0 5h10v2H3V9Zm0 5h14v2H3v-2Z"/></svg>
			Tree
		</span>
		<label class="inline-flex items-center gap-1.5 px-2 py-1 rounded-md border border-border/60 bg-bg-tertiary/35 text-[12px] text-text-muted whitespace-nowrap">
			<svg class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor"><path d="M3 5h14v2H3V5Zm3 4h8v2H6V9Zm2 4h4v2H8v-2Z"/></svg>
			Filters
			<select bind:value={filterMode} class="bg-transparent text-text focus:outline-none">
				<option value="all">all</option>
				<option value="llm">llm</option>
				<option value="file">file</option>
				<option value="custom">custom</option>
				<option value="failed">failed</option>
			</select>
		</label>
		<div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-border/60 bg-bg-tertiary/35 text-[12px] text-text-muted min-w-0 max-w-[240px]">
			<svg class="w-3.5 h-3.5 shrink-0" viewBox="0 0 20 20" fill="currentColor"><path d="m14.32 13.26 3.2 3.2-1.06 1.06-3.2-3.2a7 7 0 1 1 1.06-1.06ZM8.5 14a5.5 5.5 0 1 0 0-11 5.5 5.5 0 0 0 0 11Z"/></svg>
			<span class="truncate">{searchQuery.trim() ? `Search: ${searchQuery}` : 'Search'}</span>
		</div>
		<button
			class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-[12px] transition-colors duration-150 {showMetadata ? 'border-accent/35 bg-accent/10 text-accent' : 'border-border/60 bg-bg-tertiary/35 text-text-muted hover:text-text'}"
			onclick={() => showMetadata = !showMetadata}
		>
			<svg class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor"><path d="M4 3h12a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1Zm1 3v2h10V6H5Zm0 4v4h4v-4H5Zm6 0v1h4v-1h-4Zm0 3v1h4v-1h-4Z"/></svg>
			Metadata
		</button>
	</div>

	<!-- Time axis -->
	<div class="flex shrink-0 border-b border-border/70 bg-bg-secondary/35" style="height: 32px">
		<div class="shrink-0 sticky left-0 z-20 bg-bg-secondary/85 border-r border-border/60" style="width: {LABEL_WIDTH}px"></div>
		<div class="flex-1 relative min-w-0 overflow-hidden">
			<div class="relative h-full" style="width: {zoom * 100}%; transform: translateX(-{viewportScrollLeft}px)">
				{#each ticks as tick}
					<div class="absolute top-0 bottom-0 border-l border-border/50" style="left: {tick.pct}%">
						<span class="absolute top-1 left-1 text-[11px] text-text-muted/85 whitespace-nowrap bg-bg/70 px-1 rounded">{tick.label}</span>
					</div>
				{/each}
			</div>
		</div>
	</div>

	<!-- Rows -->
	<div
		bind:this={scrollContainer}
		class="flex-1 min-h-0 overflow-auto"
		onscroll={onViewportScroll}
		onwheel={(e) => {
			if (e.ctrlKey || e.metaKey) {
				e.preventDefault();
				if (e.deltaY < 0) zoomIn();
				else zoomOut();
			}
		}}
	>
		{#if rows.length === 0}
			<div class="text-text-muted text-[13px] text-center py-12">No spans</div>
		{:else}
			{#each rows as row (row.span.id)}
				{@const s = row.span}
				{@const tokens = tokenCount(s)}
				{@const compactTokenCount = compactTokens(tokens)}
				{@const cost = costBadge(s)}
				{@const model = modelBadge(s)}
				{@const provider = providerBadge(s)}
				<button
					class="flex items-center w-full transition-colors group border-b border-border/20
						{selectedId === s.id ? 'bg-accent/10' : 'hover:bg-bg-tertiary/30 odd:bg-bg-secondary/10'}"
					style="height: {ROW_HEIGHT}px"
					onclick={() => onSelect?.(s)}
				>
					<!-- Label column -->
					<div
						class="shrink-0 sticky left-0 z-10 flex items-center gap-1.5 px-2 overflow-hidden border-r border-border/40
							{selectedId === s.id ? 'bg-bg-secondary/96' : 'bg-bg/96 group-hover:bg-bg-secondary/90'}"
						style="width: {LABEL_WIDTH}px; padding-left: {row.depth * INDENT_PX + 8}px"
					>
						<span class="w-1.5 h-1.5 rounded-full shrink-0 {statusDotClass(s)}"></span>
						<div class="shrink-0 opacity-60"><SpanKindIcon span={s} /></div>
						<span class="text-[12px] text-text truncate font-medium {showMetadata ? 'max-w-[32%]' : ''}">{s.name}</span>
						{#if showMetadata}
							<span class="shrink-0 text-[10px] text-text-secondary bg-bg-tertiary/60 border border-border/45 rounded px-1.5 py-px font-mono">{formatDuration(row.durationMs)}</span>
							{#if model}
								<span class="shrink-0 text-[10px] text-accent bg-accent/10 border border-accent/30 rounded px-1.5 py-px max-w-[24%] truncate">{model}</span>
							{/if}
							{#if provider}
								<span class="shrink-0 text-[10px] text-text-muted bg-bg-tertiary/45 border border-border/45 rounded px-1.5 py-px">{provider}</span>
							{/if}
							{#if compactTokenCount}
								<span class="shrink-0 text-[10px] text-text-muted bg-bg-tertiary/45 border border-border/45 rounded px-1.5 py-px">{compactTokenCount} tok</span>
							{/if}
							{#if cost}
								<span class="shrink-0 text-[10px] text-success bg-success/10 border border-success/25 rounded px-1.5 py-px">{cost}</span>
							{/if}
						{/if}
					</div>

					<!-- Bar area -->
					<div class="flex-1 relative min-w-0 h-full">
						<div class="relative h-full" style="width: {zoom * 100}%">
							<!-- Grid lines -->
							{#each ticks as tick}
								<div class="absolute top-0 bottom-0 border-l border-border/28" style="left: {tick.pct}%"></div>
							{/each}

							<!-- Bar -->
							<div
								class="absolute top-1.5 bottom-1.5 rounded-[5px] border {barColor(s)} flex items-center overflow-hidden cursor-pointer shadow-[inset_0_1px_0_rgba(255,255,255,0.18)]
									{selectedId === s.id ? 'ring-1 ring-accent ring-offset-1 ring-offset-bg' : ''}"
								style="left: {row.startPct}%; width: max({MIN_BAR_PX}px, {row.widthPct}%)"
								role="button"
								tabindex={-1}
								onmouseenter={(e) => showTooltip(e, s)}
								onmouseleave={hideTooltip}
							>
								<!-- Inline label (only when bar is wide enough) -->
								{#if showBarLabel(row.startPct, row.widthPct) || selectedId === s.id}
									<span class="text-[10px] text-text font-medium truncate px-1.5 whitespace-nowrap opacity-95">
										{s.name}
									</span>
								{/if}
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
				<span class="text-accent">{s.kind.model}</span>
				{#if s.kind.provider}
					<span class="text-text-muted">{s.kind.provider}</span>
				{/if}
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
