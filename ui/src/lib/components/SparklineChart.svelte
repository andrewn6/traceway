<script lang="ts">
	let {
		points,
		labels = [],
		unit = '',
		color = '#58a6ff',
		height = 120,
	}: {
		points: number[];
		labels?: string[];
		unit?: string;
		color?: string;
		height?: number;
	} = $props();

	let containerEl: HTMLDivElement | undefined = $state(undefined);
	let hoverIndex: number | null = $state(null);
	let mouseX = $state(0);
	let mouseY = $state(0);

	const maxP = $derived(Math.max(...points));
	const minP = $derived(Math.min(...points));
	const range = $derived(maxP - minP || 1);

	// SVG coordinate space
	const svgW = 200;
	const svgH = 80;
	const padY = 6;

	function yCoord(v: number): number {
		return svgH - ((v - minP) / range) * (svgH - padY * 2) - padY;
	}

	const linePath = $derived(
		points.map((v, i) => {
			const x = (i / (points.length - 1)) * svgW;
			const y = yCoord(v);
			return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
		}).join(' ')
	);

	const areaPath = $derived(
		`${linePath} L${svgW},${svgH} L0,${svgH} Z`
	);

	function handleMouseMove(e: MouseEvent) {
		if (!containerEl) return;
		const rect = containerEl.getBoundingClientRect();
		const relX = e.clientX - rect.left;
		const pct = Math.max(0, Math.min(1, relX / rect.width));
		hoverIndex = Math.round(pct * (points.length - 1));
		mouseX = e.clientX;
		mouseY = e.clientY;
	}

	function handleMouseLeave() {
		hoverIndex = null;
	}

	const hoverX = $derived(hoverIndex !== null ? (hoverIndex / (points.length - 1)) * svgW : 0);
	const hoverY = $derived(hoverIndex !== null ? yCoord(points[hoverIndex]) : 0);

	function formatValue(v: number): string {
		if (v >= 1_000_000) return (v / 1_000_000).toFixed(2) + 'M';
		if (v >= 1_000) return (v / 1_000).toFixed(1) + 'K';
		if (v < 1 && v > 0) return v.toFixed(4);
		return v.toFixed(1);
	}

	// Generate unique gradient ID
	const gradientId = $derived(`spark-grad-${Math.random().toString(36).slice(2, 8)}`);
</script>

<div
	class="relative w-full cursor-crosshair"
	style="height: {height}px;"
	bind:this={containerEl}
	onmousemove={handleMouseMove}
	onmouseleave={handleMouseLeave}
	role="img"
	aria-label="Sparkline chart"
>
	<svg class="w-full h-full" viewBox="0 0 {svgW} {svgH}" preserveAspectRatio="none">
		<defs>
			<linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
				<stop offset="0%" stop-color={color} stop-opacity="0.25" />
				<stop offset="100%" stop-color={color} stop-opacity="0.02" />
			</linearGradient>
		</defs>

		<!-- Grid lines -->
		{#each [0.25, 0.5, 0.75] as frac}
			<line
				x1="0" y1={svgH * frac} x2={svgW} y2={svgH * frac}
				stroke="currentColor" stroke-opacity="0.06" stroke-width="0.5"
				vector-effect="non-scaling-stroke"
			/>
		{/each}

		<!-- Area fill -->
		<path d={areaPath} fill="url(#{gradientId})" />

		<!-- Line -->
		<path
			d={linePath}
			fill="none"
			stroke={color}
			stroke-width="2"
			stroke-linejoin="round"
			vector-effect="non-scaling-stroke"
		/>

		<!-- Hover crosshair -->
		{#if hoverIndex !== null}
			<!-- Vertical line -->
			<line
				x1={hoverX} y1="0" x2={hoverX} y2={svgH}
				stroke={color} stroke-opacity="0.4" stroke-width="1"
				stroke-dasharray="3,3"
				vector-effect="non-scaling-stroke"
			/>
			<!-- Dot -->
			<circle
				cx={hoverX} cy={hoverY} r="4"
				fill={color}
				stroke="var(--color-bg-secondary)"
				stroke-width="2"
				vector-effect="non-scaling-stroke"
			/>
		{/if}
	</svg>

	<!-- Y-axis labels -->
	<div class="absolute inset-y-0 left-0 flex flex-col justify-between pointer-events-none py-1">
		<span class="text-[9px] text-text-muted/60 font-mono">{formatValue(maxP)}{unit}</span>
		<span class="text-[9px] text-text-muted/60 font-mono">{formatValue(minP)}{unit}</span>
	</div>
</div>

<!-- Floating tooltip -->
{#if hoverIndex !== null}
	<div
		class="fixed z-50 pointer-events-none"
		style="left: {mouseX + 14}px; top: {mouseY - 12}px;"
	>
		<div class="bg-bg-tertiary border border-border rounded-lg px-3 py-2 shadow-xl shadow-black/40 text-xs space-y-0.5">
			<div class="flex items-center gap-2">
				<span class="w-2 h-2 rounded-full" style="background: {color};"></span>
				<span class="text-text font-mono font-medium">{formatValue(points[hoverIndex])}{unit}</span>
			</div>
			<div class="text-text-muted">{labels[hoverIndex] ?? `Point ${hoverIndex + 1} of ${points.length}`}</div>
		</div>
	</div>
{/if}
