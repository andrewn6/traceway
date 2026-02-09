<script lang="ts">
	let { status }: { status: 'running' | 'completed' | 'failed' } = $props();

	const colors = {
		running: 'bg-warning/20 text-warning border-warning/30',
		completed: 'bg-success/20 text-success border-success/30',
		failed: 'bg-danger/20 text-danger border-danger/30'
	};
</script>

<span class="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-xs border {colors[status]}">
	{#if status === 'running'}
		<span class="inline-block w-1.5 h-1.5 rounded-full bg-warning animate-pulse"></span>
	{:else if status === 'completed'}
		<span class="inline-block w-1.5 h-1.5 rounded-full bg-success"></span>
	{:else}
		<span class="inline-block w-1.5 h-1.5 rounded-full bg-danger"></span>
	{/if}
	{status}
</span>
