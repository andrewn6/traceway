<script lang="ts">
	let { completed, total }: { completed: number; total: number } = $props();
	const pct = $derived(total > 0 ? (completed / total) * 100 : 0);
</script>

<div class="flex items-center gap-2 text-xs text-text-secondary">
	<div class="flex-1 h-1 bg-bg-tertiary rounded-full overflow-hidden">
		<div
			class="h-full bg-purple-400 rounded-full transition-all duration-300"
			style="width: {pct}%"
		></div>
	</div>
	<span class="font-mono">{completed}/{total}</span>
</div>
