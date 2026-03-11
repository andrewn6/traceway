<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		children,
		colSpan = 1,
		rowSpan = 1,
		accent = '#6ee7b7',
		index = 0,
		onmenu,
	}: {
		title: string;
		children: Snippet;
		colSpan?: number;
		rowSpan?: number;
		accent?: string;
		index?: number;
		onmenu?: () => void;
	} = $props();

	let hovered = $state(false);
</script>

<div
	class="table-float flex flex-col overflow-hidden transition-colors duration-150"
	style="grid-column: span {colSpan}; grid-row: span {rowSpan};
		{hovered ? `border-color: color-mix(in oklab, ${accent} 30%, var(--color-border));` : ''}"
	role="presentation"
	onmouseenter={() => (hovered = true)}
	onmouseleave={() => (hovered = false)}
>
	<!-- Card header -->
	<div class="flex items-center gap-2 px-4 pt-3.5 pb-1">
		<span class="text-text-secondary text-sm font-medium flex-1">{title}</span>
		{#if onmenu}
			<button
				class="text-text-muted hover:text-text transition-colors p-0.5 cursor-pointer"
				class:opacity-100={hovered}
				class:opacity-0={!hovered}
				onclick={onmenu}
				aria-label="Card menu"
			>
				<svg class="w-4 h-4" viewBox="0 0 16 16" fill="currentColor">
					<circle cx="8" cy="3" r="1.3" />
					<circle cx="8" cy="8" r="1.3" />
					<circle cx="8" cy="13" r="1.3" />
				</svg>
			</button>
		{/if}
	</div>

	<!-- Card body -->
	<div class="flex-1 px-4 pb-3.5 min-h-0">
		{@render children()}
	</div>
</div>
