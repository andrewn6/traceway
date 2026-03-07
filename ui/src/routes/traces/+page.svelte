<script lang="ts">
	import { goto } from '$app/navigation';
	import { getTraces, getSpans, subscribeEvents, getAuthConfig, createApiKey, type Span, type Trace, type AuthConfig, type ApiKeyCreated } from '$lib/api';
	import { spanDurationMs, spanStatus } from '$lib/api';
	import TraceRow from '$lib/components/TraceRow.svelte';
	import { onMount } from 'svelte';

	let traces: Trace[] = $state([]);
	let traceSpans: Map<string, Span[]> = $state(new Map());
	let filterText = $state('');
	let filterModel = $state('');
	let filterStatus = $state('');
	let quickFilter = $state('');
	let loading = $state(true);

	type TraceStatus = 'failed' | 'running' | 'completed';
	type QuickFilterId = 'failed' | 'running' | 'high-token' | 'slow';
	type TraceInsights = {
		status: TraceStatus;
		totalTokens: number;
		totalDuration: number;
		models: string[];
		searchables: string[];
	};

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
		// In cloud mode, the API is at api.traceway.ai
		if (isCloudMode) return 'https://api.traceway.ai';
		return window.location.origin;
	});

	const apiKeyValue = $derived(createdKey ? createdKey.key : 'tw_sk_...');

	const envSnippet = $derived.by(() => {
		if (isCloudMode || createdKey) {
			return `TRACEWAY_API_KEY="${apiKeyValue}"\nTRACEWAY_URL="${tracewayUrl}"`;
		}
		return `TRACEWAY_URL="${tracewayUrl}"`;
	});

	const codeSnippet = $derived.by(() => {
		return `import os
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
        print(resp.choices[0].message.content)`;
	});

	async function handleCreateKey() {
		creatingKey = true;
		createKeyError = '';
		try {
			createdKey = await createApiKey(keyName.trim() || 'default');
			showKeyForm = false;
		} catch (e: any) {
			createKeyError = e?.message || 'Failed to create API key';
		}
		creatingKey = false;
	}

	function copyEnv() {
		navigator.clipboard.writeText(envSnippet);
		envCopied = true;
		setTimeout(() => (envCopied = false), 2000);
	}

	function copyCode() {
		navigator.clipboard.writeText(codeSnippet);
		codeCopied = true;
		setTimeout(() => (codeCopied = false), 2000);
	}

	async function loadTraces() {
		try {
			// Two calls in parallel instead of N+1
			const [traceResult, spanResult] = await Promise.all([
				getTraces(),
				getSpans()
			]);
			traces = traceResult.items.sort(
				(a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime()
			);

			// Group spans by trace_id client-side
			const allSpans: Span[] = spanResult.items;
			const spanMap = new Map<string, Span[]>();
			for (const span of allSpans) {
				const existing = spanMap.get(span.trace_id) ?? [];
				existing.push(span);
				spanMap.set(span.trace_id, existing);
			}
			traceSpans = spanMap;
		} catch {
			// API not available
		}
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
				if (!traces.some(t => t.id === tid)) {
					// New trace — reload to get trace metadata
					loadTraces();
				}
			} else if (event.type === 'span_completed' || event.type === 'span_failed') {
				const tid = event.span.trace_id;
				const existing = traceSpans.get(tid);
				if (existing) {
					traceSpans.set(
						tid,
						existing.map((s) => (s.id === event.span.id ? event.span : s))
					);
					traceSpans = new Map(traceSpans);
				}
			} else if (event.type === 'trace_deleted') {
				traceSpans.delete(event.trace_id);
				traceSpans = new Map(traceSpans);
				traces = traces.filter((t) => t.id !== event.trace_id);
			} else if (event.type === 'cleared') {
				traceSpans = new Map();
				traces = [];
			}
		});

		return unsub;
	});

	function toInsights(spans: Span[]): TraceInsights {
		const status: TraceStatus = spans.some((s) => spanStatus(s) === 'failed')
			? 'failed'
			: spans.some((s) => spanStatus(s) === 'running')
				? 'running'
				: 'completed';

		let totalTokens = 0;
		let totalDuration = 0;
		const models = new Set<string>();

		for (const span of spans) {
			if (span.kind?.type === 'llm_call') {
				totalTokens += (span.kind.input_tokens ?? 0) + (span.kind.output_tokens ?? 0);
				if (span.kind.model) models.add(span.kind.model);
			}
			const duration = spanDurationMs(span);
			if (duration !== null) totalDuration = Math.max(totalDuration, duration);
		}

		const root = spans.find((s) => !s.parent_id);
		const searchables = [root?.name ?? '', ...spans.map((s) => s.name)];

		return {
			status,
			totalTokens,
			totalDuration,
			models: [...models],
			searchables
		};
	}

	const traceInsights = $derived.by(() => {
		const map = new Map<string, TraceInsights>();
		for (const trace of traces) {
			map.set(trace.id, toInsights(traceSpans.get(trace.id) ?? []));
		}
		return map;
	});

	const quickFilters = $derived.by(() => {
		const entries = [...traceInsights.values()];
		const highTokenThreshold = 8_000;
		const slowThresholdMs = 10_000;

		return [
			{
				id: 'failed' as QuickFilterId,
				label: 'Failed',
				description: 'Trace has at least one failed span',
				count: entries.filter((t) => t.status === 'failed').length
			},
			{
				id: 'running' as QuickFilterId,
				label: 'Running',
				description: 'Trace is currently in progress',
				count: entries.filter((t) => t.status === 'running').length
			},
			{
				id: 'high-token' as QuickFilterId,
				label: 'High token',
				description: `Tokens >= ${highTokenThreshold.toLocaleString()}`,
				count: entries.filter((t) => t.totalTokens >= highTokenThreshold).length
			},
			{
				id: 'slow' as QuickFilterId,
				label: 'Slow',
				description: `Duration >= ${(slowThresholdMs / 1000).toFixed(0)}s`,
				count: entries.filter((t) => t.totalDuration >= slowThresholdMs).length
			}
		];
	});

	const activeFilters = $derived.by(() => {
		const chips: { key: string; label: string; clear: () => void }[] = [];
		if (filterText.trim()) {
			chips.push({ key: 'search', label: `Search: ${filterText.trim()}`, clear: () => (filterText = '') });
		}
		if (filterModel.trim()) {
			chips.push({ key: 'model', label: `Model: ${filterModel.trim()}`, clear: () => (filterModel = '') });
		}
		if (filterStatus) {
			chips.push({ key: 'status', label: `Status: ${filterStatus}`, clear: () => (filterStatus = '') });
		}
		if (quickFilter) {
			const match = quickFilters.find((q) => q.id === quickFilter);
			chips.push({ key: 'quick', label: `Quick: ${match?.label ?? quickFilter}`, clear: () => (quickFilter = '') });
		}
		return chips;
	});

	const filtered = $derived.by(() => {
		return traces.filter((trace) => {
			const spans = traceSpans.get(trace.id) ?? [];
			const insights = traceInsights.get(trace.id) ?? toInsights(spans);
			if (filterText.trim()) {
				const q = filterText.toLowerCase();
				const inNames = insights.searchables.some((name) => name.toLowerCase().includes(q));
				if (!trace.id.toLowerCase().includes(q) && !inNames) {
					return false;
				}
			}
			if (filterModel) {
				const hasModel = insights.models.some((model) => model.includes(filterModel));
				if (!hasModel) return false;
			}
			if (filterStatus) {
				if (insights.status !== filterStatus) return false;
			}
			if (quickFilter) {
				if (quickFilter === 'failed' && insights.status !== 'failed') return false;
				if (quickFilter === 'running' && insights.status !== 'running') return false;
				if (quickFilter === 'high-token' && insights.totalTokens < 8_000) return false;
				if (quickFilter === 'slow' && insights.totalDuration < 10_000) return false;
			}
			return true;
		});
	});
</script>

<div class="max-w-6xl mx-auto space-y-4">
	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading...</div>
	{:else if traces.length === 0}
		<!-- Empty state: centered onboarding -->
		<div class="flex justify-center py-12">
			<div class="space-y-5 w-full max-w-2xl">
				<div class="space-y-1">
					<h1 class="text-xl font-bold text-text">Traces</h1>
					<div class="flex items-center gap-2">
						<span class="w-1.5 h-1.5 rounded-full bg-success animate-pulse"></span>
						<span class="text-xs text-text-muted">Listening for traces</span>
					</div>
				</div>

				<!-- API Key creation (cloud mode only) -->
				{#if isCloudMode && !createdKey}
					<div class="space-y-2">
						{#if !showKeyForm}
							<button
								onclick={() => (showKeyForm = true)}
								class="px-3 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors"
							>
								Create API key
							</button>
							<p class="text-[11px] text-text-muted">
								Or use an existing key from <a href="/settings/api-keys" class="text-accent hover:underline">Settings &rarr; API Keys</a>
							</p>
						{:else}
							<div class="bg-bg-secondary border border-border rounded p-3 space-y-3">
								<div>
									<label for="onboard-key-name" class="block text-xs text-text-secondary mb-1">Key name</label>
									<input
										id="onboard-key-name"
										type="text"
										bind:value={keyName}
										placeholder="e.g. development"
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
									/>
								</div>
								{#if createKeyError}
									<p class="text-[11px] text-danger">{createKeyError}</p>
								{/if}
								<div class="flex gap-2">
									<button
										onclick={handleCreateKey}
										disabled={creatingKey}
										class="px-3 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50"
									>
										{creatingKey ? 'Creating...' : 'Create key'}
									</button>
									<button
										onclick={() => (showKeyForm = false)}
										class="px-3 py-1.5 text-sm bg-bg-tertiary text-text rounded hover:bg-bg-secondary transition-colors"
									>
										Cancel
									</button>
								</div>
							</div>
						{/if}
					</div>
				{/if}

				<!-- .env snippet (with real key if just created) -->
				<div class="space-y-1.5">
					<div class="flex items-center justify-between">
						<span class="text-xs text-text-muted font-medium">.env</span>
						<button
							onclick={copyEnv}
							class="text-[11px] text-text-muted hover:text-text transition-colors"
						>
							{envCopied ? 'Copied!' : 'Copy'}
						</button>
					</div>
					<pre class="bg-bg-tertiary border border-border rounded p-3 text-[13px] text-text-secondary font-mono leading-relaxed">{envSnippet}</pre>
					{#if createdKey}
						<p class="text-[11px] text-text-muted">Your key is shown once. Copy the .env now.</p>
					{/if}
				</div>

				<!-- Code snippet -->
				<div class="space-y-1.5">
					<div class="flex items-center justify-between">
						<span class="text-xs text-text-muted font-medium">quickstart.py</span>
						<button
							onclick={copyCode}
							class="text-[11px] text-text-muted hover:text-text transition-colors"
						>
							{codeCopied ? 'Copied!' : 'Copy'}
						</button>
					</div>
					<pre class="bg-bg-tertiary border border-border rounded p-3 text-[13px] text-text-secondary font-mono leading-relaxed">{codeSnippet}</pre>
				</div>

				<a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="inline-block text-accent text-xs hover:underline">
					Read the docs &rarr;
				</a>
			</div>
		</div>
	{:else}
		<div class="space-y-4">
			<div class="flex items-center justify-between">
				<div>
					<h1 class="text-xl font-bold">Traces</h1>
					<p class="text-xs text-text-muted mt-1">Search by trace id, span name, model, and status.</p>
				</div>
				<div class="text-[11px] text-text-muted">{filtered.length} / {traces.length} visible</div>
			</div>

			<div class="glass-surface rounded-2xl p-3 sm:p-3.5 sticky bottom-3 z-20 space-y-3">
				<div class="flex flex-wrap items-center gap-2">
					<span class="text-[10px] uppercase tracking-[0.16em] text-text-muted">Quick filters</span>
					{#each quickFilters as chip (chip.id)}
						<button
							title={chip.description}
							onclick={() => (quickFilter = quickFilter === chip.id ? '' : chip.id)}
							disabled={chip.count === 0}
							class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full border text-[11px] transition-colors disabled:opacity-40 disabled:cursor-not-allowed {quickFilter === chip.id ? 'border-accent/80 bg-accent/15 text-accent' : 'border-border/70 bg-bg-tertiary/55 text-text-secondary hover:text-text'}"
						>
							<span>{chip.label}</span>
							<span class="font-mono text-[10px] text-text-muted">{chip.count}</span>
						</button>
					{/each}
				</div>

				<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-[minmax(0,1fr)_150px_150px_auto] gap-2.5 items-end">
					<label class="space-y-1">
						<span class="text-[10px] uppercase tracking-[0.15em] text-text-muted">Search</span>
						<input
							type="text"
							placeholder="Trace id or span name"
							bind:value={filterText}
							class="w-full bg-bg-tertiary/65 border border-border/60 rounded-lg px-3 py-2 text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-accent/70"
						/>
					</label>
					<label class="space-y-1">
						<span class="text-[10px] uppercase tracking-[0.15em] text-text-muted">Model contains</span>
						<input
							type="text"
							placeholder="gpt-4o"
							bind:value={filterModel}
							class="w-full bg-bg-tertiary/65 border border-border/60 rounded-lg px-3 py-2 text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-accent/70"
						/>
					</label>
					<label class="space-y-1">
						<span class="text-[10px] uppercase tracking-[0.15em] text-text-muted">Status</span>
						<select
							bind:value={filterStatus}
							id="filter-status"
							class="w-full bg-bg-tertiary/65 border border-border/60 rounded-lg px-3 py-2 text-xs text-text focus:outline-none focus:border-accent/70"
						>
							<option value="">Any status</option>
							<option value="running">Running</option>
							<option value="completed">Completed</option>
							<option value="failed">Failed</option>
						</select>
					</label>
					<button
						onclick={() => {
							filterText = '';
							filterModel = '';
							filterStatus = '';
							quickFilter = '';
						}}
						class="h-9 px-3 text-xs rounded-lg border border-border/60 text-text-muted hover:text-text hover:border-border transition-colors"
					>
						Reset all
					</button>
				</div>

				<div class="flex flex-wrap items-center gap-2 min-h-6">
					{#if activeFilters.length > 0}
						<span class="text-[10px] uppercase tracking-[0.16em] text-text-muted">Active</span>
						{#each activeFilters as active (active.key)}
							<button
								onclick={active.clear}
								class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-accent/10 border border-accent/35 text-[11px] text-accent hover:bg-accent/15 transition-colors"
							>
								<span>{active.label}</span>
								<span class="text-[10px]">x</span>
							</button>
						{/each}
					{:else}
						<span class="text-[11px] text-text-muted">No active filters</span>
					{/if}
				</div>
			</div>

			<div class="space-y-2">
				<div class="grid grid-cols-[1fr_140px_80px_80px_80px_80px_80px_60px] gap-3 px-3.5 text-[10px] text-text-muted uppercase tracking-[0.15em]">
					<span>Trace</span>
					<span>Timestamp</span>
					<span class="text-center">Status</span>
					<span class="text-right">Duration</span>
					<span class="text-right">Tokens</span>
					<span class="text-right">Cost</span>
					<span>Model</span>
					<span></span>
				</div>

				{#if filtered.length === 0}
					<div class="glass-soft rounded-xl text-text-muted text-sm text-center py-8">No traces match filters</div>
				{:else}
					<div class="space-y-2">
						{#each filtered as trace (trace.id)}
							<TraceRow traceId={trace.id} spans={traceSpans.get(trace.id) ?? []} onDelete={(id) => {
								traces = traces.filter(t => t.id !== id);
								traceSpans.delete(id);
								traceSpans = new Map(traceSpans);
							}} />
						{/each}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
