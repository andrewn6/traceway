<script lang="ts">
	import type { Span } from '$lib/api';
	import { spanStatus, spanDurationMs, spanKindLabel } from '$lib/api';
	import StatusBadge from './StatusBadge.svelte';
	import SpanKindIcon from './SpanKindIcon.svelte';
	import SpanTree from './SpanTree.svelte';

	let {
		spans,
		parentId = null,
		depth = 0,
		selectedId = null,
		onSelect
	}: {
		spans: Span[];
		parentId?: string | null;
		depth?: number;
		selectedId?: string | null;
		onSelect?: (span: Span) => void;
	} = $props();

	const children = $derived(spans.filter((s) => s.parent_id === parentId));
</script>

{#each children as span (span.id)}
	{@const status = spanStatus(span)}
	{@const duration = spanDurationMs(span)}
	{@const kindLabel = spanKindLabel(span)}
	<button
		class="w-full text-left py-1.5 px-2 hover:bg-bg-tertiary rounded text-sm transition-colors
			{selectedId === span.id ? 'bg-bg-tertiary border-l-2 border-accent' : ''}"
		style="padding-left: {depth * 20 + 8}px"
		onclick={() => onSelect?.(span)}
	>
		<div class="flex items-center gap-2">
			<SpanKindIcon {span} />
			<span class="text-text truncate flex-1">{span.name}</span>
			{#if kindLabel}
				<span class="text-text-muted text-xs px-1 py-0.5 bg-bg-tertiary rounded">{kindLabel}</span>
			{/if}
			<StatusBadge {status} />
			{#if duration !== null}
				<span class="text-text-muted text-xs font-mono">{duration}ms</span>
			{/if}
		</div>
	</button>
	<SpanTree
		{spans}
		parentId={span.id}
		depth={depth + 1}
		{selectedId}
		{onSelect}
	/>
{/each}
