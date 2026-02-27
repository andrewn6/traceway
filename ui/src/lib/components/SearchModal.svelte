<script lang="ts">
	import { getSpans, spanStatus, spanStartedAt, spanDurationMs, shortId, type Span, type SpanFilter } from '$lib/api';
	import SpanKindIcon from './SpanKindIcon.svelte';
	import StatusBadge from './StatusBadge.svelte';
	import { goto } from '$app/navigation';

	let { open = $bindable(false) }: { open: boolean } = $props();

	let query = $state('');
	let results: Span[] = $state([]);
	let loading = $state(false);
	let searched = $state(false);
	let selectedIdx = $state(0);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let inputEl: HTMLInputElement | undefined = $state(undefined);

	$effect(() => {
		if (open && inputEl) {
			// Focus input when modal opens
			setTimeout(() => inputEl?.focus(), 50);
		}
		if (!open) {
			query = '';
			results = [];
			searched = false;
			selectedIdx = 0;
		}
	});

	function close() {
		open = false;
	}

	function search(text: string) {
		if (debounceTimer) clearTimeout(debounceTimer);
		if (!text.trim()) {
			results = [];
			searched = false;
			return;
		}
		debounceTimer = setTimeout(async () => {
			loading = true;
			try {
				const filter: SpanFilter = { text_contains: text.trim() };
				const res = await getSpans(filter);
				results = res.items.slice(0, 20);
				searched = true;
				selectedIdx = 0;
			} catch {
				results = [];
			}
			loading = false;
		}, 250);
	}

	$effect(() => {
		search(query);
	});

	function navigateToResult(span: Span) {
		close();
		goto(`/traces/${span.trace_id}`);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			close();
			e.preventDefault();
		} else if (e.key === 'ArrowDown') {
			selectedIdx = Math.min(selectedIdx + 1, results.length - 1);
			e.preventDefault();
		} else if (e.key === 'ArrowUp') {
			selectedIdx = Math.max(selectedIdx - 1, 0);
			e.preventDefault();
		} else if (e.key === 'Enter' && results.length > 0) {
			navigateToResult(results[selectedIdx]);
			e.preventDefault();
		}
	}

	// ── Helpers ────────────────────────────────────────────────────────
	function formatDuration(ms: number | null): string {
		if (ms === null) return '...';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function kindLabel(s: Span): string {
		if (!s.kind) return 'unknown';
		if (s.kind.type === 'custom') return s.kind.kind;
		return s.kind.type;
	}

	function extractSnippet(span: Span, needle: string): string | null {
		const lowerNeedle = needle.toLowerCase();
		for (const field of [span.input, span.output]) {
			if (!field) continue;
			const text = typeof field === 'string' ? field : JSON.stringify(field);
			const lowerText = text.toLowerCase();
			const idx = lowerText.indexOf(lowerNeedle);
			if (idx >= 0) {
				const start = Math.max(0, idx - 40);
				const end = Math.min(text.length, idx + needle.length + 60);
				const prefix = start > 0 ? '...' : '';
				const suffix = end < text.length ? '...' : '';
				return prefix + text.slice(start, end) + suffix;
			}
		}
		return null;
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[200] flex items-start justify-center pt-[15vh] bg-black/60"
		onclick={(e) => { if (e.target === e.currentTarget) close(); }}
		onkeydown={handleKeydown}
	>
		<div class="w-full max-w-xl bg-bg-secondary border border-border rounded-xl shadow-2xl overflow-hidden">
			<!-- Search input -->
			<div class="flex items-center gap-3 px-4 py-3 border-b border-border">
				<svg class="w-5 h-5 text-text-muted shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
					<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
				</svg>
				<input
					bind:this={inputEl}
					bind:value={query}
					type="text"
					placeholder="Search span content..."
					class="flex-1 bg-transparent text-sm text-text placeholder:text-text-muted/50 outline-none"
				/>
				{#if loading}
					<div class="w-4 h-4 border-2 border-accent/30 border-t-accent rounded-full animate-spin"></div>
				{/if}
				<kbd class="text-[10px] text-text-muted bg-bg-tertiary rounded px-1.5 py-0.5 border border-border">Esc</kbd>
			</div>

			<!-- Results -->
			<div class="max-h-80 overflow-y-auto">
				{#if !searched && !query.trim()}
					<div class="px-4 py-6 text-center text-xs text-text-muted">
						Search across all span inputs and outputs
					</div>
				{:else if searched && results.length === 0}
					<div class="px-4 py-6 text-center text-xs text-text-muted">
						No results for "{query}"
					</div>
				{:else}
					{#each results as span, idx (span.id)}
						{@const snippet = extractSnippet(span, query)}
						<button
							class="w-full flex flex-col gap-0.5 px-4 py-2.5 text-left border-b border-border/30 transition-colors
								{idx === selectedIdx ? 'bg-accent/10' : 'hover:bg-bg-tertiary/50'}"
							onclick={() => navigateToResult(span)}
							onmouseenter={() => selectedIdx = idx}
						>
							<div class="flex items-center gap-2">
								<SpanKindIcon {span} />
								<span class="text-xs font-medium text-text truncate">{span.name}</span>
								{#if span.kind?.type === 'llm_call'}
									<span class="text-[10px] text-purple-400 bg-purple-400/10 rounded px-1 py-px">{span.kind.model}</span>
								{/if}
								<div class="flex-1"></div>
								<StatusBadge status={spanStatus(span)} />
								<span class="text-[10px] text-text-muted font-mono">{formatDuration(spanDurationMs(span))}</span>
							</div>
							{#if snippet}
								<div class="text-[11px] text-text-muted/80 font-mono truncate pl-6">{snippet}</div>
							{/if}
							<div class="flex items-center gap-2 text-[10px] text-text-muted pl-6">
								<span>Trace {shortId(span.trace_id)}</span>
								<span class="text-text-muted/40">&middot;</span>
								<span>{new Date(spanStartedAt(span)).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</span>
							</div>
						</button>
					{/each}
				{/if}
			</div>

			<!-- Footer -->
			<div class="flex items-center gap-4 px-4 py-2 border-t border-border text-[10px] text-text-muted">
				<span><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border">↑</kbd><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border ml-0.5">↓</kbd> navigate</span>
				<span><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border">Enter</kbd> open trace</span>
				<span><kbd class="bg-bg-tertiary rounded px-1 py-px border border-border">Esc</kbd> close</span>
			</div>
		</div>
	</div>
{/if}
