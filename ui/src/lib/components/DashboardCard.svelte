<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		children,
		colSpan = 1,
		rowSpan = 1,
		onmenu,
	}: {
		title: string;
		children: Snippet;
		colSpan?: number;
		rowSpan?: number;
		onmenu?: () => void;
	} = $props();
</script>

<div
	class="bg-bg-secondary border border-border rounded-lg flex flex-col overflow-hidden"
	style="grid-column: span {colSpan}; grid-row: span {rowSpan};"
>
	<!-- Card header -->
	<div class="flex items-center gap-2 px-4 pt-3 pb-1">
		<!-- Drag handle -->
		<svg class="w-4 h-4 text-text-muted shrink-0 cursor-grab" viewBox="0 0 16 16" fill="currentColor">
			<circle cx="4" cy="3" r="1.2" />
			<circle cx="4" cy="8" r="1.2" />
			<circle cx="4" cy="13" r="1.2" />
			<circle cx="10" cy="3" r="1.2" />
			<circle cx="10" cy="8" r="1.2" />
			<circle cx="10" cy="13" r="1.2" />
		</svg>
		<span class="text-text text-sm font-medium flex-1">{title}</span>
		<!-- Menu button -->
		<button
			class="text-text-muted hover:text-text transition-colors p-0.5 cursor-pointer"
			onclick={onmenu}
		>
			<svg class="w-4 h-4" viewBox="0 0 16 16" fill="currentColor">
				<circle cx="8" cy="3" r="1.3" />
				<circle cx="8" cy="8" r="1.3" />
				<circle cx="8" cy="13" r="1.3" />
			</svg>
		</button>
	</div>

	<!-- Card body -->
	<div class="flex-1 px-4 pb-3 min-h-0">
		{@render children()}
	</div>

	<!-- Resize handle -->
	<div class="flex justify-end pr-1 pb-1">
		<svg class="w-3 h-3 text-text-muted/40" viewBox="0 0 12 12" fill="currentColor">
			<path d="M11 1v10H1" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
		</svg>
	</div>
</div>
