<script lang="ts">
	let { score, size = 'sm' }: { score: number | null | undefined; size?: 'xs' | 'sm' } = $props();
	const colorClass = $derived(
		score == null
			? 'bg-bg-tertiary text-text-secondary border-border'
			: score >= 0.7
				? 'bg-success/20 text-success border-success/30'
				: score >= 0.4
					? 'bg-warning/20 text-warning border-warning/30'
					: 'bg-danger/20 text-danger border-danger/30'
	);
	const textSize = $derived(size === 'xs' ? 'text-xs' : 'text-sm');
</script>

{#if score != null}
	<span class="inline-flex items-center rounded px-1.5 py-0.5 {textSize} border font-mono {colorClass}">
		{score.toFixed(2)}
	</span>
{:else}
	<span class="inline-flex items-center rounded px-1.5 py-0.5 {textSize} text-text-muted">&mdash;</span>
{/if}
