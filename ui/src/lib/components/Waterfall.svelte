<script lang="ts">
	import type { Span } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs, spanKindColor } from '$lib/api';

	let {
		spans,
		selectedId = null,
		onSelect
	}: {
		spans: Span[];
		selectedId?: string | null;
		onSelect?: (span: Span) => void;
	} = $props();

	const sorted = $derived(
		[...spans].sort(
			(a, b) => new Date(spanStartedAt(a)).getTime() - new Date(spanStartedAt(b)).getTime()
		)
	);

	const timeRange = $derived.by(() => {
		if (sorted.length === 0) return { min: 0, max: 1 };
		const starts = sorted.map((s) => new Date(spanStartedAt(s)).getTime());
		const ends = sorted.map((s) => {
			const end = spanEndedAt(s);
			return end ? new Date(end).getTime() : Date.now();
		});
		const min = Math.min(...starts);
		const max = Math.max(...ends);
		return { min, max: max === min ? min + 1 : max };
	});

	function barStyle(span: Span): string {
		const start = new Date(spanStartedAt(span)).getTime();
		const end = spanEndedAt(span) ? new Date(spanEndedAt(span)!).getTime() : Date.now();
		const range = timeRange.max - timeRange.min;
		const left = ((start - timeRange.min) / range) * 100;
		const width = Math.max(((end - start) / range) * 100, 0.5);
		return `left: ${left}%; width: ${width}%;`;
	}

	const statusColors: Record<string, string> = {
		running: 'bg-warning',
		completed: 'bg-success',
		failed: 'bg-danger'
	};
</script>

<div class="space-y-1">
	{#each sorted as span (span.id)}
		{@const status = spanStatus(span)}
		{@const duration = spanDurationMs(span)}
		{@const barColor = span.kind ? spanKindColor(span) : statusColors[status]}
		<button
			class="w-full flex items-center gap-2 py-1 px-2 hover:bg-bg-tertiary rounded text-xs transition-colors
				{selectedId === span.id ? 'bg-bg-tertiary' : ''}"
			onclick={() => onSelect?.(span)}
		>
			<span class="text-text-secondary font-mono w-24 shrink-0 text-left truncate">{span.name}</span>
			<div class="flex-1 relative h-5">
				<div
					class="absolute top-1 h-3 rounded {barColor} opacity-80"
					style={barStyle(span)}
				></div>
			</div>
			<span class="text-text-muted font-mono w-16 text-right shrink-0">
				{duration !== null ? `${duration}ms` : '...'}
			</span>
		</button>
	{/each}
</div>
