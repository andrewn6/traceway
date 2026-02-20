<script lang="ts">
	let {
		items,
		maxItems = 5,
	}: {
		items: { label: string; value: number; color?: string }[];
		maxItems?: number;
	} = $props();

	const displayed = $derived(items.slice(0, maxItems));
	const maxValue = $derived(Math.max(...displayed.map((i) => i.value), 1));

	function formatValue(v: number): string {
		if (v >= 1_000_000_000) return (v / 1_000_000_000).toFixed(3) + 'B';
		if (v >= 1_000_000) return (v / 1_000_000).toFixed(1) + 'M';
		if (v >= 1_000) return (v / 1_000).toFixed(v >= 10_000 ? 1 : 3) + 'K';
		return v.toLocaleString();
	}
</script>

<div class="space-y-2">
	{#each displayed as item}
		<div class="flex items-center gap-3">
			<div class="flex-1 min-w-0">
				<div
					class="h-7 rounded flex items-center px-2 text-xs font-medium text-white truncate"
					style="width: {Math.max((item.value / maxValue) * 100, 8)}%; background: {item.color ?? '#4673d1'};"
				>
					{item.label}
				</div>
			</div>
			<span class="text-text text-xs font-mono w-16 text-right shrink-0">{formatValue(item.value)}</span>
		</div>
	{/each}
</div>
