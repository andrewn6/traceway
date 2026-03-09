<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	type WidthMode = 'compact' | 'default' | 'wide';

	let {
		open = false,
		title = 'Inspector',
		subtitle = '',
		width = 'default' as WidthMode,
		showBackdrop = true,
		children
	} = $props();

	const dispatch = createEventDispatcher<{
		close: void;
		width: { width: WidthMode };
	}>();

	function cycleWidth() {
		const next: WidthMode = width === 'compact' ? 'default' : width === 'default' ? 'wide' : 'compact';
		dispatch('width', { width: next });
	}

	function close() {
		dispatch('close');
	}

	const widthClass = $derived.by(() => {
		if (width === 'compact') return 'w-[min(380px,calc(100vw-1rem))]';
		if (width === 'wide') return 'w-[min(700px,calc(100vw-1rem))]';
		return 'w-[min(520px,calc(100vw-1rem))]';
	});
</script>

<svelte:window
	onkeydown={(e) => {
		if (!open) return;
		if (e.key === 'Escape') close();
		if (e.key === '[') dispatch('width', { width: 'compact' });
		if (e.key === ']') dispatch('width', { width: 'wide' });
	}}
/>

{#if open}
	{#if showBackdrop}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="fixed inset-0 z-[120] bg-black/25"
			onclick={close}
			onkeydown={(e) => e.key === 'Escape' && close()}
		></div>
	{/if}

	<aside class="fixed right-3 top-[5.2rem] bottom-16 z-[130] {widthClass} pointer-events-auto">
		<div class="h-full surface-panel overflow-hidden flex flex-col">
			<div class="px-3.5 py-2.5 border-b border-border/60 flex items-center gap-2">
				<div class="min-w-0 flex-1">
					<div class="text-[14px] font-semibold text-text truncate">{title}</div>
					{#if subtitle}
						<div class="text-[11px] text-text-muted truncate">{subtitle}</div>
					{/if}
				</div>
				<button class="query-icon-button" onclick={cycleWidth} title="Resize panel" aria-label="Resize panel">{width === 'compact' ? 'S' : width === 'default' ? 'M' : 'L'}</button>
				<button class="query-icon-button" onclick={close} title="Close panel" aria-label="Close panel">
					<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
				</button>
			</div>
			<div class="flex-1 min-h-0 overflow-y-auto p-3.5">
				{@render children?.()}
			</div>
		</div>
	</aside>
{/if}
