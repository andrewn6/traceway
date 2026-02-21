<script lang="ts">
	let {
		items,
		maxItems = 5,
		accentColor = '#4673d1',
	}: {
		items: { label: string; value: number; color?: string }[];
		maxItems?: number;
		accentColor?: string;
	} = $props();

	const displayed = $derived(items.slice(0, maxItems));
	const maxValue = $derived(Math.max(...displayed.map((i) => i.value), 1));
	const totalValue = $derived(displayed.reduce((sum, i) => sum + i.value, 0));

	let hoveredIndex: number | null = $state(null);
	let tooltipX = $state(0);
	let tooltipY = $state(0);

	function formatValue(v: number): string {
		if (v >= 1_000_000_000) return (v / 1_000_000_000).toFixed(3) + 'B';
		if (v >= 1_000_000) return (v / 1_000_000).toFixed(1) + 'M';
		if (v >= 1_000) return (v / 1_000).toFixed(v >= 10_000 ? 1 : 3) + 'K';
		return v.toLocaleString();
	}

	function formatExact(v: number): string {
		return v.toLocaleString();
	}

	function handleMouseMove(e: MouseEvent) {
		tooltipX = e.clientX;
		tooltipY = e.clientY;
	}
</script>

<div class="space-y-2">
	{#each displayed as item, i}
		{@const pct = totalValue > 0 ? ((item.value / totalValue) * 100).toFixed(1) : '0.0'}
		{@const barPct = Math.max((item.value / maxValue) * 100, 8)}
		<div
			class="group relative flex items-center gap-3"
			role="listitem"
			onmouseenter={() => (hoveredIndex = i)}
			onmouseleave={() => (hoveredIndex = null)}
			onmousemove={handleMouseMove}
		>
			<div class="flex-1 min-w-0">
				<div
					class="h-7 rounded flex items-center px-2.5 text-xs font-medium text-white truncate transition-all duration-200"
					style="width: {barPct}%;
						background: linear-gradient(90deg, {item.color ?? accentColor}, {item.color ?? accentColor}cc);
						{hoveredIndex === i ? `box-shadow: 0 0 12px ${item.color ?? accentColor}55; filter: brightness(1.2);` : ''}"
				>
					{item.label}
				</div>
			</div>
			<span class="text-text text-xs font-mono w-16 text-right shrink-0 transition-colors duration-150
				{hoveredIndex === i ? 'text-accent' : ''}">{formatValue(item.value)}</span>
		</div>
	{/each}
</div>

<!-- Floating tooltip -->
{#if hoveredIndex !== null && displayed[hoveredIndex]}
	{@const item = displayed[hoveredIndex]}
	{@const pct = totalValue > 0 ? ((item.value / totalValue) * 100).toFixed(1) : '0.0'}
	<div
		class="fixed z-50 pointer-events-none"
		style="left: {tooltipX + 12}px; top: {tooltipY - 8}px;"
	>
		<div class="bg-bg-tertiary border border-border rounded-lg px-3 py-2 shadow-xl shadow-black/40 text-xs space-y-1 min-w-[140px]">
			<div class="text-text font-medium truncate">{item.label}</div>
			<div class="flex items-center justify-between gap-4">
				<span class="text-text-muted">Value</span>
				<span class="text-text font-mono">{formatExact(item.value)}</span>
			</div>
			<div class="flex items-center justify-between gap-4">
				<span class="text-text-muted">Share</span>
				<span class="text-accent font-mono">{pct}%</span>
			</div>
			<div class="w-full bg-bg-secondary rounded-full h-1 mt-1 overflow-hidden">
				<div class="h-full rounded-full transition-all duration-300" style="width: {pct}%; background: {item.color ?? accentColor};"></div>
			</div>
		</div>
	</div>
{/if}
