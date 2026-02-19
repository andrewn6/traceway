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
</script>

<a
	href="/traces/{traceId}"
	class="grid grid-cols-[1fr_80px_100px_140px_100px_100px_60px] gap-4 items-center py-2 px-3 hover:bg-bg-tertiary rounded text-sm transition-colors border-b border-border"
>
	<span class="font-mono text-accent text-xs truncate">{shortId(traceId)}</span>
	<span class="text-text-secondary text-center">{spans.length}</span>
	<span class="text-center"><StatusBadge {status} /></span>
	<span class="text-text-secondary font-mono text-xs">{started}</span>
	<span class="text-text-secondary font-mono text-xs text-right">
		{totalDuration !== null ? `${totalDuration}ms` : '...'}
	</span>
	<span class="text-text-secondary text-xs truncate">{model ?? '-'}</span>
	<button
		class="text-xs transition-colors {confirmDelete ? 'text-danger font-semibold' : 'text-text-muted hover:text-danger'}"
		onclick={handleDelete}
	>{confirmDelete ? 'confirm?' : 'delete'}</button>
</a>
