<script lang="ts">
	import { getSpans, type Span } from '$lib/api';
	import { spanDurationMs, spanStatus, shortId } from '$lib/api';
	import SpanDetail from '$lib/components/SpanDetail.svelte';
	import { onMount } from 'svelte';

	let spans: Span[] = $state([]);
	let loading = $state(true);
	let q = $state('');
	let selectedSpan: Span | null = $state(null);

	const filtered = $derived.by(() => {
		const query = q.trim().toLowerCase();
		if (!query) return spans;
		return spans.filter((s) => s.name.toLowerCase().includes(query) || s.trace_id.toLowerCase().includes(query) || s.kind.type.toLowerCase().includes(query) || (s.kind.type === 'llm_call' && s.kind.model?.toLowerCase().includes(query)));
	});

	onMount(async () => {
		try {
			const res = await getSpans();
			spans = res.items.sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime());
		} finally {
			loading = false;
		}
	});

	function formatTime(iso: string): string {
		const d = new Date(iso);
		const pad = (n: number) => n.toString().padStart(2, '0');
		return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
	}

	function formatDuration(ms: number | null): string {
		if (ms == null) return '-';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function spanTokenTotal(s: Span): number {
		if (s.kind?.type !== 'llm_call') return 0;
		return (s.kind.input_tokens ?? 0) + (s.kind.output_tokens ?? 0);
	}
</script>

<div class="flex flex-col h-[calc(100vh-10rem)] -m-4 lg:-m-5 rounded-xl overflow-hidden border border-border/40 bg-bg-secondary/30">
	<div class="flex items-center gap-2 px-4 py-2 border-b border-border/55 bg-bg-secondary/40 shrink-0">
		<div class="flex items-center gap-0.5 rounded-lg border border-border/50 bg-bg-tertiary/35 p-0.5">
			<a href="/traces" class="px-3 py-1 text-[12px] rounded-md text-text-muted hover:text-text transition-colors">Traces</a>
			<button class="px-3 py-1 text-[12px] rounded-md bg-bg-tertiary text-text border border-border">Spans</button>
		</div>
		<div class="flex-1"></div>
		<div class="command-input-shell w-64">
			<div class="pl-2.5 pr-1.5 text-text-muted/80">
				<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" /></svg>
			</div>
			<input class="command-input text-[12px]" bind:value={q} placeholder="Search spans..." />
		</div>
	</div>

	<div class="flex-1 flex min-h-0">
		<div class="flex-1 flex flex-col min-w-0">
			<div class="grid grid-cols-[140px_1fr_80px_120px_70px_70px] gap-2 px-4 py-2 table-head-compact border-b border-border/55 bg-bg-secondary/30 shrink-0">
				<span>Time</span><span>Name</span><span>Kind</span><span>Model</span><span class="text-right">Tokens</span><span class="text-right">Latency</span>
			</div>
			<div class="flex-1 overflow-auto">
				{#if loading}
					<div class="py-10 text-center text-sm text-text-muted">Loading spans...</div>
				{:else if filtered.length === 0}
					<div class="py-10 text-center text-sm text-text-muted">No spans found</div>
				{:else}
					{#each filtered as s (s.id)}
						{@const dur = spanDurationMs(s)}
						{@const st = spanStatus(s)}
						<button
							class="grid grid-cols-[140px_1fr_80px_120px_70px_70px] gap-2 px-4 py-2 border-b border-border/30 w-full text-left text-[12px] motion-row items-center
								{selectedSpan?.id === s.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'hover:bg-bg-secondary/40'}"
							onclick={() => (selectedSpan = s)}
						>
							<span class="font-mono text-text-muted text-[11px] truncate">{formatTime(s.started_at)}</span>
							<span class="truncate text-text flex items-center gap-1.5">
								<span class="w-1.5 h-1.5 rounded-full shrink-0 {st === 'failed' ? 'bg-danger' : st === 'running' ? 'bg-warning animate-pulse' : 'bg-success'}"></span>
								{s.name}
							</span>
							<span class="text-text-muted truncate">{s.kind.type === 'llm_call' ? 'LLM' : s.kind.type === 'fs_read' ? 'Read' : s.kind.type === 'fs_write' ? 'Write' : 'Custom'}</span>
							<span class="text-text-muted truncate font-mono">{s.kind.type === 'llm_call' ? s.kind.model : '-'}</span>
							<span class="text-right text-text-muted font-mono">{spanTokenTotal(s) || '-'}</span>
							<span class="text-right text-text-muted font-mono">{formatDuration(dur)}</span>
						</button>
					{/each}
				{/if}
			</div>
			<div class="flex items-center px-4 py-2 border-t border-border/55 bg-bg-secondary/30 shrink-0 text-[11px] text-text-muted">
				<span>{filtered.length} of {spans.length} spans</span>
			</div>
		</div>

		{#if selectedSpan}
			<div class="w-[560px] shrink-0 border-l border-border/55 overflow-hidden flex flex-col motion-slide-in-right bg-bg-secondary/20">
				<SpanDetail span={selectedSpan} onClose={() => (selectedSpan = null)} allSpans={spans} />
			</div>
		{/if}
	</div>
</div>
