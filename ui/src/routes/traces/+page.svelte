<script lang="ts">
	import { goto } from '$app/navigation';
	import { getTraces, getSpans, subscribeEvents, getAuthConfig, createApiKey, shortId, type Span, type Trace, type AuthConfig, type ApiKeyCreated } from '$lib/api';
	import { spanDurationMs, spanStatus } from '$lib/api';
	import SpanDetail from '$lib/components/SpanDetail.svelte';
	import TraceTimeline from '$lib/components/TraceTimeline.svelte';
	import { onMount } from 'svelte';

	let traces: Trace[] = $state([]);
	let traceSpans: Map<string, Span[]> = $state(new Map());
	let filterText = $state('');
	let loading = $state(true);
	let rangeDays: '5m' | '1h' | '1d' | '7d' | '30d' = $state('7d');
	let viewMode: 'traces' | 'spans' = $state('spans');
	let selectedDetailSpan: Span | null = $state(null);
	let selectedTraceId: string | null = $state(null);
	// When true, show span detail; when false, show trace tree
	let showSpanDetail = $state(false);

	type TraceStatus = 'failed' | 'running' | 'completed';
	type TraceInsights = { status: TraceStatus; totalTokens: number; totalDuration: number; totalCost: number; models: string[]; searchables: string[] };

	// Onboarding state
	let authConfig = $state<AuthConfig | null>(null);
	let createdKey = $state<ApiKeyCreated | null>(null);
	let creatingKey = $state(false);
	let createKeyError = $state('');
	let keyName = $state('default');
	let envCopied = $state(false);
	let codeCopied = $state(false);
	let showKeyForm = $state(false);

	const isCloudMode = $derived(authConfig?.mode === 'cloud');
	const tracewayUrl = $derived.by(() => {
		if (typeof window === 'undefined') return 'http://localhost:3000';
		const viteUrl = import.meta.env.VITE_API_URL as string;
		if (viteUrl) return viteUrl.replace(/\/api$/, '');
		if (isCloudMode) return 'https://api.traceway.ai';
		return window.location.origin;
	});

	const apiKeyValue = $derived(createdKey ? createdKey.key : 'tw_sk_...');
	const envSnippet = $derived.by(() => {
		if (isCloudMode || createdKey) return `TRACEWAY_API_KEY="${apiKeyValue}"\nTRACEWAY_URL="${tracewayUrl}"`;
		return `TRACEWAY_URL="${tracewayUrl}"`;
	});
	const codeSnippet = $derived.by(() => `import os
from openai import OpenAI
from traceway import Traceway

client = Traceway()
openai = OpenAI()

with client.trace("summarize-doc") as t:
    with t.llm_call("generate-summary", model="gpt-4o"):
        resp = openai.chat.completions.create(
            model="gpt-4o",
            messages=[{"role": "user", "content": "Summarize: ..."}],
        )
        print(resp.choices[0].message.content)`);

	async function handleCreateKey() {
		creatingKey = true; createKeyError = '';
		try { createdKey = await createApiKey(keyName.trim() || 'default'); showKeyForm = false; }
		catch (e: any) { createKeyError = e?.message || 'Failed to create API key'; }
		creatingKey = false;
	}
	function copyEnv() { navigator.clipboard.writeText(envSnippet); envCopied = true; setTimeout(() => (envCopied = false), 2000); }
	function copyCode() { navigator.clipboard.writeText(codeSnippet); codeCopied = true; setTimeout(() => (codeCopied = false), 2000); }

	async function loadTraces() {
		try {
			let allTraces: Trace[] = [];
			let cursor: string | null = null;
			for (let i = 0; i < 20; i++) {
				const page = await getTraces(cursor ? { cursor } : undefined);
				allTraces = allTraces.concat(page.items);
				if (!page.has_more || !page.next_cursor) break;
				cursor = page.next_cursor;
			}
			const spanResult = await getSpans();
			traces = allTraces.sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime());
			const allSpans: Span[] = spanResult.items;
			const spanMap = new Map<string, Span[]>();
			for (const span of allSpans) {
				const existing = spanMap.get(span.trace_id) ?? [];
				existing.push(span);
				spanMap.set(span.trace_id, existing);
			}
			traceSpans = spanMap;
		} catch {}
		loading = false;
	}

	onMount(() => {
		loadTraces();
		getAuthConfig().then((c) => (authConfig = c)).catch(() => {});
		const unsub = subscribeEvents((event) => {
			if (event.type === 'span_created') {
				const tid = event.span.trace_id;
				const existing = traceSpans.get(tid) ?? [];
				traceSpans.set(tid, [...existing, event.span]);
				traceSpans = new Map(traceSpans);
				if (!traces.some(t => t.id === tid)) loadTraces();
			} else if (event.type === 'span_completed' || event.type === 'span_failed') {
				const tid = event.span.trace_id;
				const existing = traceSpans.get(tid);
				if (existing) {
					traceSpans.set(tid, existing.map((s) => (s.id === event.span.id ? event.span : s)));
					traceSpans = new Map(traceSpans);
				}
				if (selectedDetailSpan?.id === event.span.id) selectedDetailSpan = event.span;
			} else if (event.type === 'trace_deleted') {
				traceSpans.delete(event.trace_id);
				traceSpans = new Map(traceSpans);
				traces = traces.filter((t) => t.id !== event.trace_id);
				if (selectedTraceId === event.trace_id) { selectedTraceId = null; selectedDetailSpan = null; showSpanDetail = false; }
			} else if (event.type === 'cleared') {
				traceSpans = new Map(); traces = []; selectedDetailSpan = null; selectedTraceId = null; showSpanDetail = false;
			}
		});
		return unsub;
	});

	function toInsights(spans: Span[]): TraceInsights {
		const status: TraceStatus = spans.some((s) => spanStatus(s) === 'failed') ? 'failed' : spans.some((s) => spanStatus(s) === 'running') ? 'running' : 'completed';
		let totalTokens = 0, totalDuration = 0, totalCost = 0;
		const models = new Set<string>();
		for (const span of spans) {
			if (span.kind?.type === 'llm_call') {
				totalTokens += (span.kind.input_tokens ?? 0) + (span.kind.output_tokens ?? 0);
				totalCost += span.kind.cost ?? 0;
				if (span.kind.model) models.add(span.kind.model);
			}
			const duration = spanDurationMs(span);
			if (duration !== null) totalDuration = Math.max(totalDuration, duration);
		}
		const root = spans.find((s) => !s.parent_id);
		return { status, totalTokens, totalDuration, totalCost, models: [...models], searchables: [root?.name ?? '', ...spans.map((s) => s.name)] };
	}

	const traceInsights = $derived.by(() => {
		const map = new Map<string, TraceInsights>();
		for (const trace of traces) map.set(trace.id, toInsights(traceSpans.get(trace.id) ?? []));
		return map;
	});

	// Flat list of all spans
	const allSpansFlat = $derived.by(() => {
		const all: Span[] = [];
		for (const [, spans] of traceSpans) all.push(...spans);
		return all.sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime());
	});

	const filteredSpans = $derived.by(() => {
		const query = filterText.trim().toLowerCase();
		if (!query) return allSpansFlat;
		return allSpansFlat.filter(s =>
			s.name.toLowerCase().includes(query)
			|| s.id.toLowerCase().includes(query)
			|| s.trace_id.toLowerCase().includes(query)
			|| s.kind.type.toLowerCase().includes(query)
			|| (s.kind.type === 'llm_call' && s.kind.model?.toLowerCase().includes(query))
		);
	});

	const filteredTraces = $derived.by(() => {
		const query = filterText.trim().toLowerCase();
		if (!query) return traces;
		return traces.filter((trace) => {
			const insights = traceInsights.get(trace.id) ?? toInsights([]);
			const haystack = [trace.id.toLowerCase(), ...insights.searchables.map(s => s.toLowerCase()), ...insights.models.map(m => m.toLowerCase()), insights.status];
			return haystack.some((h) => h.includes(query));
		});
	});

	const totalCostAll = $derived.by(() => {
		let cost = 0;
		for (const s of allSpansFlat) {
			if (s.kind?.type === 'llm_call' && s.kind.cost != null) cost += s.kind.cost;
		}
		return cost;
	});

	function selectSpanRow(span: Span) {
		selectedDetailSpan = span;
		selectedTraceId = span.trace_id;
		showSpanDetail = true;
	}

	function selectTraceRow(traceId: string) {
		selectedTraceId = traceId;
		selectedDetailSpan = null;
		showSpanDetail = false;
	}

	function onSpanAction() { loadTraces(); }

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

	function spanCostValue(s: Span): number {
		if (s.kind?.type !== 'llm_call' || s.kind.cost == null) return 0;
		return s.kind.cost;
	}
</script>

{#if loading}
	<div class="text-text-muted text-sm text-center py-8">Loading...</div>
{:else if traces.length === 0}
	<!-- Empty state: onboarding -->
	<div class="flex justify-center py-12">
		<div class="space-y-5 w-full max-w-5xl">
			<div class="space-y-1">
				<h1 class="text-xl font-bold text-text">Traces</h1>
				<div class="flex items-center gap-2">
					<span class="w-1.5 h-1.5 rounded-full bg-success animate-pulse"></span>
					<span class="text-xs text-text-muted">Listening for traces</span>
				</div>
			</div>
			{#if isCloudMode && !createdKey}
				<div class="space-y-2">
					{#if !showKeyForm}
						<button onclick={() => (showKeyForm = true)} class="px-3 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors">Create API key</button>
						<p class="text-[11px] text-text-muted">Or use an existing key from <a href="/settings/api-keys" class="text-accent hover:underline">Settings &rarr; API Keys</a></p>
					{:else}
						<div class="table-float p-3 space-y-3">
							<div>
								<label for="onboard-key-name" class="block text-xs text-text-secondary mb-1">Key name</label>
								<input id="onboard-key-name" type="text" bind:value={keyName} placeholder="e.g. development" class="w-full bg-bg-tertiary border border-border rounded-lg px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent" />
							</div>
							{#if createKeyError}<p class="text-[11px] text-danger">{createKeyError}</p>{/if}
							<div class="flex gap-2">
								<button onclick={handleCreateKey} disabled={creatingKey} class="px-3 py-1.5 text-sm bg-accent text-bg font-semibold rounded-lg hover:bg-accent/80 transition-colors disabled:opacity-50">{creatingKey ? 'Creating...' : 'Create key'}</button>
								<button onclick={() => (showKeyForm = false)} class="px-3 py-1.5 text-sm bg-bg-tertiary text-text rounded-lg hover:bg-bg-secondary transition-colors">Cancel</button>
							</div>
						</div>
					{/if}
				</div>
			{/if}
			<div class="space-y-1.5">
				<div class="flex items-center justify-between"><span class="text-xs text-text-muted font-medium">.env</span><button onclick={copyEnv} class="text-[11px] text-text-muted hover:text-text transition-colors">{envCopied ? 'Copied!' : 'Copy'}</button></div>
				<pre class="bg-bg-tertiary border border-border rounded p-3 text-[13px] text-text-secondary font-mono leading-relaxed">{envSnippet}</pre>
				{#if createdKey}<p class="text-[11px] text-text-muted">Your key is shown once. Copy the .env now.</p>{/if}
			</div>
			<div class="space-y-1.5">
				<div class="flex items-center justify-between"><span class="text-xs text-text-muted font-medium">quickstart.py</span><button onclick={copyCode} class="text-[11px] text-text-muted hover:text-text transition-colors">{codeCopied ? 'Copied!' : 'Copy'}</button></div>
				<pre class="bg-bg-tertiary border border-border rounded p-3 text-[13px] text-text-secondary font-mono leading-relaxed">{codeSnippet}</pre>
			</div>
			<a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="inline-block text-accent text-xs hover:underline">Read the docs &rarr;</a>
		</div>
	</div>
{:else}
	<!-- Main data view: split pane -->
	<div class="flex flex-col h-[calc(100vh-10rem)] -m-4 lg:-m-5 rounded-xl overflow-hidden border border-border/40 bg-bg-secondary/30">
		<!-- Filter bar -->
		<div class="flex items-center gap-2 px-4 py-2 border-b border-border/55 bg-bg-secondary/40 shrink-0">
			<div class="flex items-center gap-0.5 rounded-lg border border-border/50 bg-bg-tertiary/35 p-0.5">
				<button class="px-3 py-1 text-[12px] rounded-md transition-colors {viewMode === 'spans' ? 'bg-bg-tertiary text-text border border-border' : 'text-text-muted hover:text-text'}" onclick={() => (viewMode = 'spans')}>Spans</button>
				<button class="px-3 py-1 text-[12px] rounded-md transition-colors {viewMode === 'traces' ? 'bg-bg-tertiary text-text border border-border' : 'text-text-muted hover:text-text'}" onclick={() => (viewMode = 'traces')}>Traces</button>
			</div>

			<div class="flex-1"></div>

			<select bind:value={rangeDays} class="control-select h-8 w-[110px] text-[12px]">
				<option value="5m">Last 5 mins</option>
				<option value="1h">Last 1 hour</option>
				<option value="1d">Last 1 day</option>
				<option value="7d">Last 7 days</option>
				<option value="30d">Last 30 days</option>
			</select>

			<input class="control-input h-8 text-[12px] w-64" placeholder="Search name, model, id..." bind:value={filterText} />
		</div>

		<!-- Split: table + detail -->
		<div class="flex-1 flex min-h-0">
			<!-- Left: Table -->
			<div class="flex-1 flex flex-col min-w-0">
				{#if viewMode === 'spans'}
					<!-- Spans table header -->
					<div class="grid grid-cols-[140px_1fr_80px_120px_70px_70px_70px] gap-2 px-4 py-2 table-head-compact border-b border-border/55 bg-bg-secondary/30 shrink-0">
						<span>Time</span>
						<span>Name</span>
						<span>Kind</span>
						<span>Model</span>
						<span class="text-right">Tokens</span>
						<span class="text-right">Cost</span>
						<span class="text-right">Latency</span>
					</div>
					<!-- Spans table body -->
					<div class="flex-1 overflow-auto">
						{#if filteredSpans.length === 0}
							<div class="py-8 text-center text-sm text-text-muted">No spans match current filters</div>
						{:else}
							{#each filteredSpans as s (s.id)}
								{@const dur = spanDurationMs(s)}
								{@const st = spanStatus(s)}
								<button
									class="grid grid-cols-[140px_1fr_80px_120px_70px_70px_70px] gap-2 px-4 py-2 border-b border-border/30 w-full text-left text-[12px] transition-colors items-center
										{selectedDetailSpan?.id === s.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'hover:bg-bg-secondary/40'}"
									onclick={() => selectSpanRow(s)}
								>
									<span class="font-mono text-text-muted text-[11px] truncate">{formatTime(s.started_at)}</span>
									<span class="truncate text-text flex items-center gap-1.5">
										<span class="w-1.5 h-1.5 rounded-full shrink-0 {st === 'failed' ? 'bg-danger' : st === 'running' ? 'bg-warning animate-pulse' : 'bg-success'}"></span>
										{s.name}
									</span>
									<span class="text-text-muted truncate">{s.kind.type === 'llm_call' ? 'LLM' : s.kind.type === 'fs_read' ? 'Read' : s.kind.type === 'fs_write' ? 'Write' : 'Custom'}</span>
									<span class="text-text-muted truncate font-mono">{s.kind.type === 'llm_call' ? s.kind.model : '-'}</span>
									<span class="text-right text-text-muted font-mono">{spanTokenTotal(s) || '-'}</span>
									<span class="text-right font-mono {spanCostValue(s) > 0 ? 'text-success' : 'text-text-muted'}">{spanCostValue(s) > 0 ? `$${spanCostValue(s).toFixed(4)}` : '-'}</span>
									<span class="text-right text-text-muted font-mono">{formatDuration(dur)}</span>
								</button>
							{/each}
						{/if}
					</div>
				{:else}
					<!-- Traces table header -->
					<div class="grid grid-cols-[140px_1fr_80px_120px_70px_70px_70px_50px] gap-2 px-4 py-2 table-head-compact border-b border-border/55 bg-bg-secondary/30 shrink-0">
						<span>Time</span>
						<span>Name</span>
						<span>Status</span>
						<span>Model</span>
						<span class="text-right">Tokens</span>
						<span class="text-right">Cost</span>
						<span class="text-right">Latency</span>
						<span class="text-right">Spans</span>
					</div>
					<!-- Traces table body -->
					<div class="flex-1 overflow-auto">
						{#if filteredTraces.length === 0}
							<div class="py-8 text-center text-sm text-text-muted">No traces match current filters</div>
						{:else}
							{#each filteredTraces as trace (trace.id)}
								{@const insights = traceInsights.get(trace.id) ?? toInsights([])}
								{@const spans = traceSpans.get(trace.id) ?? []}
								{@const root = spans.find(s => !s.parent_id)}
								<button
									class="grid grid-cols-[140px_1fr_80px_120px_70px_70px_70px_50px] gap-2 px-4 py-2 border-b border-border/30 w-full text-left text-[12px] transition-colors items-center
										{selectedTraceId === trace.id ? 'bg-accent/8 border-l-2 border-l-accent' : 'hover:bg-bg-secondary/40'}"
									onclick={() => selectTraceRow(trace.id)}
								>
									<span class="font-mono text-text-muted text-[11px] truncate">{formatTime(trace.started_at)}</span>
									<span class="truncate text-text flex items-center gap-1.5">
										<span class="w-1.5 h-1.5 rounded-full shrink-0 {insights.status === 'failed' ? 'bg-danger' : insights.status === 'running' ? 'bg-warning animate-pulse' : 'bg-success'}"></span>
										{root?.name ?? 'trace'}
									</span>
									<span class="capitalize text-[11px] {insights.status === 'failed' ? 'text-danger' : insights.status === 'running' ? 'text-warning' : 'text-success'}">{insights.status}</span>
									<span class="text-text-muted truncate font-mono">{insights.models[0] ?? '-'}</span>
									<span class="text-right text-text-muted font-mono">{insights.totalTokens || '-'}</span>
									<span class="text-right font-mono {insights.totalCost > 0 ? 'text-success' : 'text-text-muted'}">{insights.totalCost > 0 ? `$${insights.totalCost.toFixed(4)}` : '-'}</span>
									<span class="text-right text-text-muted font-mono">{formatDuration(insights.totalDuration)}</span>
									<span class="text-right text-text-muted">{spans.length}</span>
								</button>
							{/each}
						{/if}
					</div>
				{/if}

				<!-- Footer -->
				<div class="flex items-center px-4 py-2 border-t border-border/55 bg-bg-secondary/30 shrink-0 text-[11px] text-text-muted gap-4">
					<span>Rows per page: 100</span>
					<div class="flex-1"></div>
					<span class="font-mono">Total cost: ${totalCostAll.toFixed(2)}</span>
					<span>{viewMode === 'spans' ? `${filteredSpans.length} of ${allSpansFlat.length} spans` : `${filteredTraces.length} of ${traces.length} traces`}</span>
				</div>
			</div>

			<!-- Right: Detail panel -->
			{#if selectedDetailSpan && showSpanDetail}
				<div class="w-[560px] shrink-0 border-l border-border/55 overflow-hidden flex flex-col motion-slide-in-right bg-bg-secondary/20">
					{#if selectedTraceId && viewMode === 'traces'}
						<button
							class="flex items-center gap-1.5 px-3 py-2 text-[11px] text-text-muted hover:text-text border-b border-border/40 shrink-0 transition-colors"
							onclick={() => { showSpanDetail = false; selectedDetailSpan = null; }}
						>
							<svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M7.5 2L3.5 6l4 4"/></svg>
							Back to tree
						</button>
					{/if}
					<SpanDetail
						span={selectedDetailSpan}
						onClose={() => { selectedDetailSpan = null; selectedTraceId = null; showSpanDetail = false; }}
						{onSpanAction}
						allSpans={selectedTraceId ? (traceSpans.get(selectedTraceId) ?? []) : allSpansFlat}
					/>
				</div>
			{:else if selectedTraceId && !showSpanDetail}
				{@const traceSpanList = traceSpans.get(selectedTraceId) ?? []}
				{@const rootSpan = traceSpanList.find(s => !s.parent_id)}
				<div class="w-[560px] shrink-0 border-l border-border/55 overflow-hidden flex flex-col motion-slide-in-right bg-bg-secondary/20">
					<!-- Trace tree header -->
					<div class="flex items-center gap-2 px-3 py-2.5 border-b border-border/55 shrink-0 bg-bg-secondary/30">
						<div class="flex-1 min-w-0">
							<div class="text-[13px] font-medium text-text truncate">{rootSpan?.name ?? 'Trace'}</div>
							<div class="text-[10px] text-text-muted font-mono">{shortId(selectedTraceId)}</div>
						</div>
						<button
							class="text-[11px] text-accent hover:text-accent/80 transition-colors shrink-0"
							onclick={() => goto(`/traces/${selectedTraceId}`)}
						>Open full view</button>
						<button
							aria-label="Close trace tree"
							class="w-6 h-6 flex items-center justify-center text-text-muted hover:text-text transition-colors rounded shrink-0"
							onclick={() => { selectedTraceId = null; selectedDetailSpan = null; }}
						>
							<svg class="w-3.5 h-3.5" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 3l8 8M11 3l-8 8"/></svg>
						</button>
					</div>
					<!-- Trace tree -->
					<TraceTimeline
						spans={traceSpanList}
						selectedId={selectedDetailSpan?.id ?? null}
						onSelect={(span) => { selectedDetailSpan = span; showSpanDetail = true; }}
						showMetadata={false}
					/>
				</div>
			{/if}
		</div>
	</div>
{/if}
