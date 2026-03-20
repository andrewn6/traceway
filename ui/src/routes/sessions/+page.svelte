<script lang="ts">
	import { getSessions, getTraces, getSpans, type SessionSummary, type Trace, type Span, shortId, spanDurationMs, spanStatus } from '$lib/api';
	import { onMount } from 'svelte';
	import TraceTimeline from '$lib/components/TraceTimeline.svelte';
	import SpanDetail from '$lib/components/SpanDetail.svelte';

	let sessions: SessionSummary[] = $state([]);
	let allTraces: Trace[] = $state([]);
	let allSpans: Map<string, Span[]> = $state(new Map());
	let loading = $state(true);
	let q = $state('');
	let selectedSession: SessionSummary | null = $state(null);
	let selectedTraceId: string | null = $state(null);
	let selectedSpan: Span | null = $state(null);

	const sessionTraces = $derived.by(() => {
		if (!selectedSession) return [];
		return allTraces.filter(t => {
			const tags = t.tags ?? [];
			return tags.some((tag: string) => tag === `session_id:${selectedSession!.id}` || tag === `session:${selectedSession!.id}`);
		}).sort((a, b) => new Date(a.started_at).getTime() - new Date(b.started_at).getTime());
	});

	const selectedTraceSpans = $derived.by(() => {
		if (!selectedTraceId) return [];
		return allSpans.get(selectedTraceId) ?? [];
	});

	const filtered = $derived.by(() => {
		const query = q.trim().toLowerCase();
		if (!query) return sessions;
		return sessions.filter((s) => s.id.toLowerCase().includes(query));
	});

	async function loadData() {
		try {
			const sessionsRes = await getSessions();
			sessions = sessionsRes.items;

			// Only load traces + spans if we have sessions (avoid full table scan)
			if (sessions.length > 0) {
				const [tracesRes, spansRes] = await Promise.all([
					getTraces({ limit: 500 }),
					getSpans({ limit: '2000' } as any)
				]);
				allTraces = tracesRes.items;
				
				const spanMap = new Map<string, Span[]>();
				for (const span of spansRes.items) {
					const existing = spanMap.get(span.trace_id) ?? [];
					existing.push(span);
					spanMap.set(span.trace_id, existing);
				}
				allSpans = spanMap;
			}
		} finally {
			loading = false;
		}
	}

	function selectSession(session: SessionSummary) {
		selectedSession = session;
		selectedTraceId = null;
		selectedSpan = null;
	}

	function selectTrace(traceId: string) {
		selectedTraceId = traceId;
		selectedSpan = null;
	}

	function selectSpan(span: Span) {
		selectedSpan = span;
	}

	function closeSession() {
		selectedSession = null;
		selectedTraceId = null;
		selectedSpan = null;
	}

	function formatTime(iso: string): string {
		const d = new Date(iso);
		const pad = (n: number) => n.toString().padStart(2, '0');
		return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
	}

	function formatDate(iso: string): string {
		const d = new Date(iso);
		return d.toLocaleDateString();
	}

	function getSessionDuration(session: SessionSummary): string {
		if (!session.ended_at) return 'ongoing';
		const start = new Date(session.started_at).getTime();
		const end = new Date(session.ended_at).getTime();
		const diffMs = end - start;
		const mins = Math.floor(diffMs / 60000);
		if (mins < 60) return `${mins}m`;
		const hours = Math.floor(mins / 60);
		return `${hours}h ${mins % 60}m`;
	}

	onMount(loadData);
</script>

<div class="flex h-[calc(100vh-10rem)] -m-4 lg:-m-5 motion-rise-in">
	<!-- Left: Sessions list -->
	<div class="w-80 border-r border-border/55 flex flex-col bg-bg-secondary/20 shrink-0">
		<div class="p-3 border-b border-border/55 space-y-2">
			<h1 class="text-[13px] font-semibold text-text">Sessions</h1>
			<div class="command-input-shell w-full">
				<div class="pl-2.5 pr-1.5 text-text-muted/80">
					<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" /></svg>
				</div>
				<input class="command-input text-[12px]" bind:value={q} placeholder="Search session id..." />
			</div>
		</div>
		<div class="flex-1 overflow-auto">
			{#if loading}
				<div class="p-4 text-center text-sm text-text-muted">Loading sessions...</div>
			{:else if filtered.length === 0}
				<div class="p-4 text-center text-sm text-text-muted">
					{#if q}
						No sessions match "{q}"
					{:else}
						No sessions found.<br/>
						<span class="text-[11px]">Add tag <span class="font-mono">session_id:xxx</span> to traces</span>
					{/if}
				</div>
			{:else}
				{#each filtered as session (session.id)}
					<button
						class="w-full text-left px-3 py-2.5 border-b border-border/30 motion-row
							{selectedSession?.id === session.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'hover:bg-bg-secondary/40'}"
						onclick={() => selectSession(session)}
					>
						<div class="flex items-center justify-between mb-1">
							<span class="font-mono text-[11px] text-accent">{shortId(session.id)}</span>
							<span class="text-[10px] text-text-muted">{getSessionDuration(session)}</span>
						</div>
						<div class="flex items-center gap-3 text-[11px] text-text-muted">
							<span>{session.trace_count} traces</span>
							<span>{session.span_count} spans</span>
							<span>${session.total_cost.toFixed(4)}</span>
						</div>
						<div class="text-[10px] text-text-muted mt-1">
							{formatDate(session.started_at)} · {session.total_tokens.toLocaleString()} tokens
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</div>

	<!-- Right: Session detail -->
	{#if selectedSession}
		<div class="flex-1 flex flex-col min-w-0">
			<!-- Session header -->
			<div class="flex items-center gap-3 px-4 py-2.5 border-b border-border/55 bg-bg-secondary/30 shrink-0">
				<button onclick={closeSession} class="text-[11px] text-text-muted hover:text-text transition-colors flex items-center gap-1">
					<svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M7.5 2L3.5 6l4 4"/></svg>
					Back
				</button>
				<div class="flex-1 min-w-0">
					<div class="text-[13px] font-medium text-text truncate">Session {shortId(selectedSession.id)}</div>
					<div class="text-[10px] text-text-muted">{selectedSession.trace_count} traces · {selectedSession.span_count} spans · {selectedSession.total_tokens.toLocaleString()} tokens · ${selectedSession.total_cost.toFixed(4)}</div>
				</div>
			</div>

			<div class="flex-1 flex min-h-0">
				<!-- Trace list -->
				<div class="flex-1 flex flex-col min-w-0 border-r border-border/55">
					<div class="px-4 py-2 table-head-compact border-b border-border/55 bg-bg-secondary/20 shrink-0">
						<span class="text-[11px] text-text-muted">Traces in this session</span>
					</div>
					<div class="flex-1 overflow-auto">
						{#if sessionTraces.length === 0}
							<div class="py-10 text-center text-sm text-text-muted">No traces in this session</div>
						{:else}
							{#each sessionTraces as trace (trace.id)}
								{@const spans = allSpans.get(trace.id) ?? []}
								{@const rootSpan = spans.find(s => !s.parent_id)}
								{@const duration = Math.max(...spans.map(s => spanDurationMs(s) ?? 0))}
								{@const hasFailed = spans.some(s => spanStatus(s) === 'failed')}
								{@const hasRunning = spans.some(s => spanStatus(s) === 'running')}
								<button
									class="w-full text-left px-4 py-2 border-b border-border/30 motion-row
										{selectedTraceId === trace.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'hover:bg-bg-secondary/40'}"
									onclick={() => selectTrace(trace.id)}
								>
									<div class="flex items-center gap-2 mb-1">
										<span class="w-1.5 h-1.5 rounded-full shrink-0 {hasFailed ? 'bg-danger' : hasRunning ? 'bg-warning animate-pulse' : 'bg-success'}"></span>
										<span class="text-[12px] text-text truncate">{rootSpan?.name ?? trace.name ?? 'trace'}</span>
									</div>
									<div class="flex items-center gap-3 text-[10px] text-text-muted">
										<span class="font-mono">{formatTime(trace.started_at)}</span>
										<span>{spans.length} spans</span>
										<span>{duration ? `${(duration/1000).toFixed(1)}s` : '-'}</span>
									</div>
								</button>
							{/each}
						{/if}
					</div>
				</div>

				<!-- Trace timeline or span detail -->
				{#if selectedTraceId}
					<div class="w-[480px] shrink-0 flex flex-col overflow-hidden motion-slide-in-right">
						{#if selectedSpan}
							<div class="flex items-center px-3 py-2 border-b border-border/40 shrink-0 bg-bg-secondary/20">
								<button
									class="text-[11px] text-text-muted hover:text-text transition-colors flex items-center gap-1"
									onclick={() => selectedSpan = null}
								>
									<svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M7.5 2L3.5 6l4 4"/></svg>
									Back to timeline
								</button>
							</div>
							<SpanDetail
								span={selectedSpan}
								onClose={() => selectedSpan = null}
								onSpanAction={() => {}}
								allSpans={selectedTraceSpans}
							/>
						{:else}
							<div class="px-3 py-2 border-b border-border/40 shrink-0 bg-bg-secondary/20">
								<div class="text-[11px] text-text-muted">Trace timeline · {selectedTraceSpans.length} spans</div>
							</div>
							<div class="flex-1 overflow-auto">
								<TraceTimeline
									spans={selectedTraceSpans}
									selectedId={null}
									onSelect={selectSpan}
									showMetadata={true}
								/>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{:else}
		<!-- Empty state -->
		<div class="flex-1 flex items-center justify-center">
			<div class="text-center">
				<div class="text-text-muted text-sm mb-2">Select a session to view its traces</div>
				<div class="text-[11px] text-text-muted">Sessions are created by tagging traces with <span class="font-mono">session_id:xxx</span></div>
			</div>
		</div>
	{/if}
</div>
