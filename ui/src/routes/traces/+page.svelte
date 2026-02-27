<script lang="ts">
	import { goto } from '$app/navigation';
	import { getTraces, getSpans, subscribeEvents, getAuthConfig, createApiKey, type Span, type Trace, type AuthConfig, type ApiKeyCreated } from '$lib/api';
	import { spanStatus } from '$lib/api';
	import TraceRow from '$lib/components/TraceRow.svelte';
	import { onMount } from 'svelte';

	let traces: Trace[] = $state([]);
	let traceSpans: Map<string, Span[]> = $state(new Map());
	let filterModel = $state('');
	let filterStatus = $state('');
	let loading = $state(true);

	// Onboarding state
	let authConfig: AuthConfig | null = $state(null);
	let createdKey: ApiKeyCreated | null = $state(null);
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

	const filtered = $derived.by(() => {
		return traces.filter((trace) => {
			const spans = traceSpans.get(trace.id) ?? [];
			if (filterModel) {
				const hasModel = spans.some((s) =>
					s.kind?.type === 'llm_call' && s.kind.model?.includes(filterModel)
				);
				if (!hasModel) return false;
			}
			if (filterStatus) {
				const traceStatus = spans.some((s) => spanStatus(s) === 'failed')
					? 'failed'
					: spans.some((s) => spanStatus(s) === 'running')
						? 'running'
						: 'completed';
				if (traceStatus !== filterStatus) return false;
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
		<!-- Has traces: show header + filters + table -->
		<div class="flex items-center justify-between">
			<h1 class="text-xl font-bold">Traces</h1>
			<div class="flex items-center gap-2 text-sm">
				<input
					type="text"
					placeholder="Filter model..."
					bind:value={filterModel}
					class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs text-text placeholder:text-text-muted w-36"
				/>
				<select
					bind:value={filterStatus}
					id="filter-status"
					class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs text-text"
				>
					<option value="">All statuses</option>
					<option value="running">Running</option>
					<option value="completed">Completed</option>
					<option value="failed">Failed</option>
				</select>
			</div>
		</div>

		<!-- Table header -->
		<div class="grid grid-cols-[1fr_140px_80px_80px_80px_80px_80px_60px] gap-3 px-3 text-xs text-text-muted uppercase">
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
			<div class="text-text-muted text-sm text-center py-8">No traces match filters</div>
		{:else}
			<div class="space-y-0">
				{#each filtered as trace (trace.id)}
					<TraceRow traceId={trace.id} spans={traceSpans.get(trace.id) ?? []} onDelete={(id) => {
						traces = traces.filter(t => t.id !== id);
						traceSpans.delete(id);
						traceSpans = new Map(traceSpans);
					}} />
				{/each}
			</div>
		{/if}
	{/if}
</div>
