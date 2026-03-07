<script lang="ts">
	import type { Span } from '$lib/api';
	import { spanStatus, spanStartedAt, spanDurationMs, shortId, deleteTrace } from '$lib/api';
	import StatusBadge from './StatusBadge.svelte';

	let { traceId, spans, onDelete }: { traceId: string; spans: Span[]; onDelete?: (traceId: string) => void } = $props();

	let confirmDelete = $state(false);

	async function handleDelete(e: Event) {
		e.preventDefault();
		e.stopPropagation();
		if (!confirmDelete) {
			confirmDelete = true;
			setTimeout(() => (confirmDelete = false), 3000);
			return;
		}
		try {
			await deleteTrace(traceId);
			onDelete?.(traceId);
		} catch {
			// error
		}
		confirmDelete = false;
	}

	const firstSpan = $derived(spans[0]);
	const status = $derived.by(() => {
		if (spans.some((s) => spanStatus(s) === 'failed')) return 'failed';
		if (spans.some((s) => spanStatus(s) === 'running')) return 'running';
		return 'completed';
	});
	const model = $derived.by(() => {
		for (const s of spans) {
			if (s.kind?.type === 'llm_call') return s.kind.model;
		}
		return null;
	});
	const started = $derived(
		firstSpan ? new Date(spanStartedAt(firstSpan)).toLocaleString() : ''
	);
	const totalDuration = $derived.by(() => {
		const durations = spans.map(spanDurationMs).filter((d): d is number => d !== null);
		if (durations.length === 0) return null;
		return Math.max(...durations);
	});
	const totalTokens = $derived.by(() => {
		let total = 0;
		for (const s of spans) {
			if (s.kind?.type === 'llm_call') {
				total += (s.kind.input_tokens ?? 0) + (s.kind.output_tokens ?? 0);
			}
		}
		return total || null;
	});
	const totalCost = $derived.by(() => {
		let total = 0;
		for (const s of spans) {
			if (s.kind?.type === 'llm_call' && s.kind.cost != null) {
				total += s.kind.cost;
			}
		}
		return total || null;
	});
	const rootSpanName = $derived.by(() => {
		const root = spans.find((s) => !s.parent_id);
		return root?.name ?? firstSpan?.name ?? null;
	});
	const statusTone = $derived.by(() => {
		if (status === 'failed') {
			return {
				row: 'border-danger/30 bg-danger/[0.04] hover:bg-danger/[0.08] hover:border-danger/45',
				traceId: 'text-danger'
			};
		}
		if (status === 'running') {
			return {
				row: 'border-warning/25 bg-warning/[0.03] hover:bg-warning/[0.08] hover:border-warning/40',
				traceId: 'text-warning'
			};
		}
		return {
			row: 'border-border/60 bg-bg-secondary/30 hover:bg-success/[0.05] hover:border-success/30',
			traceId: 'text-accent'
		};
	});
</script>

<a
	href="/traces/{traceId}"
	class="group grid grid-cols-[1fr_140px_80px_80px_80px_80px_80px_60px] gap-3 items-center py-3 px-3.5 rounded-xl text-sm transition-colors border glass-soft hover-lift {statusTone.row} focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/50"
>
	<div class="min-w-0">
		<div class="font-mono text-xs truncate transition-colors {statusTone.traceId}">{shortId(traceId)}</div>
		{#if rootSpanName}
			<div class="text-text-muted text-[11px] truncate group-hover:text-text-secondary transition-colors">{rootSpanName}</div>
		{/if}
	</div>
	<span class="text-text-secondary font-mono text-xs group-hover:text-text">{started}</span>
	<span class="text-center"><StatusBadge {status} /></span>
	<span class="font-mono text-xs text-right tabular-nums {totalDuration && totalDuration >= 10000 ? 'text-warning' : 'text-text-secondary'}">
		{#if totalDuration !== null}
			{totalDuration < 1000 ? `${totalDuration}ms` : `${(totalDuration / 1000).toFixed(1)}s`}
		{:else}
			...
		{/if}
	</span>
	<span class="font-mono text-xs text-right tabular-nums {totalTokens && totalTokens >= 8000 ? 'text-warning' : 'text-text-secondary'}">
		{totalTokens ? totalTokens.toLocaleString() : '-'}
	</span>
	<span class="text-xs text-right tabular-nums font-mono {totalCost ? 'text-success' : 'text-text-muted'}">
		{totalCost ? `$${totalCost.toFixed(4)}` : '-'}
	</span>
	<span class="text-text-secondary text-xs truncate group-hover:text-text">{model ?? '-'}</span>
	<button
		class="text-xs transition-colors text-right rounded px-1 py-0.5 {confirmDelete ? 'text-danger font-semibold bg-danger/10' : 'text-text-muted hover:text-danger hover:bg-danger/10'}"
		onclick={handleDelete}
	>{confirmDelete ? 'yes?' : 'del'}</button>
</a>
