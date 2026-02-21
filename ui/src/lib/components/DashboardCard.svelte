<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		children,
		colSpan = 1,
		rowSpan = 1,
		accent = '#58a6ff',
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
	class="dashboard-card bg-bg-secondary border border-border rounded-lg flex flex-col overflow-hidden transition-all duration-300"
	style="grid-column: span {colSpan}; grid-row: span {rowSpan}; animation-delay: {index * 60}ms;
		{hovered ? `border-color: ${accent}33; box-shadow: 0 0 20px ${accent}10, inset 0 1px 0 ${accent}15;` : ''}"
	onmouseenter={() => (hovered = true)}
	onmouseleave={() => (hovered = false)}
>
	<!-- Top accent line -->
	<div
		class="h-[2px] transition-all duration-500"
		style="background: linear-gradient(90deg, transparent, {accent}{hovered ? '60' : '20'}, transparent);"
	></div>

	<!-- Card header -->
	<div class="flex items-center gap-2 px-4 pt-3 pb-1">
		<!-- Drag handle -->
		<svg
			class="w-4 h-4 shrink-0 cursor-grab transition-colors duration-200"
			style="color: {hovered ? accent + '80' : 'var(--color-text-muted)'};"
			viewBox="0 0 16 16" fill="currentColor"
		>
			<circle cx="4" cy="3" r="1.2" />
			<circle cx="4" cy="8" r="1.2" />
			<circle cx="4" cy="13" r="1.2" />
			<circle cx="10" cy="3" r="1.2" />
			<circle cx="10" cy="8" r="1.2" />
			<circle cx="10" cy="13" r="1.2" />
		</svg>
		<span class="text-text text-sm font-medium flex-1 transition-colors duration-200"
			style="{hovered ? `color: ${accent};` : ''}"
		>{title}</span>
		<!-- Menu button -->
		<button
			class="text-text-muted hover:text-text transition-colors p-0.5 cursor-pointer opacity-0 group-hover:opacity-100"
			class:opacity-100={hovered}
			onclick={onmenu}
			aria-label="Card menu"
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
	<div class="flex justify-end pr-1.5 pb-1.5 transition-opacity duration-200" class:opacity-100={hovered} class:opacity-30={!hovered}>
		<svg class="w-3 h-3 text-text-muted/50" viewBox="0 0 12 12">
			<path d="M11 5v6H5" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
			<path d="M11 1v10H1" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
		</svg>
	</div>
</div>

<style>
	.dashboard-card {
		animation: card-enter 0.4s cubic-bezier(0.16, 1, 0.3, 1) backwards;
	}

	@keyframes card-enter {
		from {
			opacity: 0;
			transform: translateY(8px) scale(0.98);
		}
	}
</style>
