<script lang="ts">
	import { page } from '$app/state';
	import {
		getEvalRun,
		cancelEvalRun,
		deleteEvalRun,
		subscribeEvents,
		shortId,
		type EvalRunDetailResponse,
		type EvalResult,
		type EvalRun
	} from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import EvalScoreBadge from '$lib/components/EvalScoreBadge.svelte';
	import EvalProgressBar from '$lib/components/EvalProgressBar.svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	const datasetId = $derived(page.params.id ?? '');
	const runId = $derived(page.params.runId ?? '');

	let run: EvalRun | null = $state(null);
	let results: EvalResult[] = $state([]);
	let loading = $state(true);
	let expandedRow: string | null = $state(null);
	let filterStatus: 'all' | 'passed' | 'failed' | 'errors' = $state('all');
	let deleteConfirm = $state(false);

	const filteredResults = $derived.by(() => {
		if (filterStatus === 'all') return results;
		if (filterStatus === 'passed') return results.filter((r) => r.status === 'passed');
		if (filterStatus === 'failed') return results.filter((r) => r.status === 'failed');
		return results.filter((r) => r.status === 'error');
	});

	const statusCounts = $derived.by(() => ({
		all: results.length,
		passed: results.filter((r) => r.status === 'passed').length,
		failed: results.filter((r) => r.status === 'failed').length,
		errors: results.filter((r) => r.status === 'error').length
	}));

	const avgLatency = $derived.by(() => {
		const completed = results.filter((r) => r.latency_ms > 0);
		if (completed.length === 0) return null;
		return completed.reduce((sum, r) => sum + r.latency_ms, 0) / completed.length;
	});

	async function load() {
		try {
			const resp = await getEvalRun(runId);
			// EvalRunDetailResponse is flattened — the run fields are at top level
			const { result_items, ...runData } = resp;
			run = runData as EvalRun;
			results = result_items;
		} catch {
			// not found
		}
		loading = false;
	}

	onMount(() => {
		load();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'eval_run_updated' && event.run.id === runId) {
				run = event.run;
				// Reload to get updated results
				getEvalRun(runId).then((resp) => {
					const { result_items, ...runData } = resp;
					run = runData as EvalRun;
					results = result_items;
				}).catch(() => {});
			} else if (event.type === 'eval_run_completed' && event.run.id === runId) {
				run = event.run;
				getEvalRun(runId).then((resp) => {
					const { result_items, ...runData } = resp;
					run = runData as EvalRun;
					results = result_items;
				}).catch(() => {});
			}
		});

		return unsub;
	});

	async function handleCancel() {
		try {
			await cancelEvalRun(runId);
			if (run) run = { ...run, status: 'cancelled' };
		} catch {
			// error
		}
	}

	async function handleDelete() {
		if (!deleteConfirm) {
			deleteConfirm = true;
			return;
		}
		try {
			await deleteEvalRun(runId);
			goto(`/datasets/${datasetId}`);
		} catch {
			// error
		}
	}

	function formatLatency(ms: number): string {
		if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
		return `${ms}ms`;
	}

	function formatJson(value: unknown): string {
		if (value === null || value === undefined) return '(none)';
		if (typeof value === 'string') return value;
		return JSON.stringify(value, null, 2);
	}

	function truncate(s: string, len: number): string {
		return s.length > len ? s.slice(0, len) + '...' : s;
	}

	function durationStr(): string {
		if (!run) return '';
		if (!run.completed_at) {
			const elapsed = Date.now() - new Date(run.created_at).getTime();
			const secs = Math.floor(elapsed / 1000);
			if (secs < 60) return `${secs}s`;
			return `${Math.floor(secs / 60)}m ${secs % 60}s`;
		}
		const dur = new Date(run.completed_at).getTime() - new Date(run.created_at).getTime();
		const secs = Math.floor(dur / 1000);
		if (secs < 60) return `${secs}s`;
		return `${Math.floor(secs / 60)}m ${secs % 60}s`;
	}
</script>

<div class="max-w-6xl mx-auto space-y-4">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<a href="/datasets/{datasetId}" class="text-text-secondary hover:text-text text-sm">&larr; Back to dataset</a>
		{#if run}
			<div class="flex items-center gap-2">
				{#if run.status === 'running'}
					<button
						class="px-3 py-1.5 text-xs bg-warning/10 text-warning border border-warning/20 rounded hover:bg-warning/20 transition-colors"
						onclick={handleCancel}
					>Cancel Run</button>
				{/if}
				<button
					class="px-3 py-1.5 text-xs bg-danger/10 text-danger border border-danger/20 rounded hover:bg-danger/20 transition-colors"
					onclick={handleDelete}
				>{deleteConfirm ? 'Confirm Delete' : 'Delete'}</button>
			</div>
		{/if}
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading...</div>
	{:else if !run}
		<div class="text-text-muted text-sm text-center py-8">Eval run not found</div>
	{:else}
		<!-- Run info -->
		<div>
			<h1 class="text-lg font-bold">{run.name ?? run.config.model}</h1>
			<div class="text-xs text-text-muted mt-1 flex items-center gap-2 flex-wrap">
				<span>Model: <span class="text-purple-400">{run.config.model}</span></span>
				{#if run.config.provider}
					<span>&middot;</span>
					<span>Provider: {run.config.provider}</span>
				{/if}
				<span>&middot;</span>
				<span>Scoring: {run.scoring}</span>
				<span>&middot;</span>
				<span>Started: {new Date(run.created_at).toLocaleString()}</span>
				<span>&middot;</span>
				<span>Duration: {durationStr()}</span>
			</div>
		</div>

		<!-- Stats cards -->
		<div class="grid grid-cols-4 gap-3">
			<div class="bg-bg-secondary border border-border rounded p-3">
				<div class="text-xs text-text-muted uppercase mb-1">Score</div>
				<div class="text-xl font-mono">
					<EvalScoreBadge score={run.results.scores.mean} />
				</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-3">
				<div class="text-xs text-text-muted uppercase mb-1">Pass Rate</div>
				<div class="text-xl font-mono text-text">
					{run.results.scores.pass_rate != null ? `${Math.round(run.results.scores.pass_rate * 100)}%` : '\u2014'}
				</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-3">
				<div class="text-xs text-text-muted uppercase mb-1">Completed</div>
				<div class="text-xl font-mono text-text">{run.results.completed}/{run.results.total}</div>
			</div>
			<div class="bg-bg-secondary border border-border rounded p-3">
				<div class="text-xs text-text-muted uppercase mb-1">Avg Latency</div>
				<div class="text-xl font-mono text-text">
					{avgLatency != null ? formatLatency(avgLatency) : '\u2014'}
				</div>
			</div>
		</div>

		<!-- Progress bar if running -->
		{#if run.status === 'running'}
			<div class="flex items-center gap-3">
				<span class="text-sm text-text-secondary">Running...</span>
				<div class="flex-1">
					<EvalProgressBar completed={run.results.completed} total={run.results.total} />
				</div>
				<span class="text-sm text-text-secondary">
					{run.results.total > 0 ? Math.round((run.results.completed / run.results.total) * 100) : 0}%
				</span>
			</div>
		{/if}

		<!-- Filter pills -->
		<div class="flex items-center gap-2">
			<button
				class="px-2 py-1 text-xs rounded border cursor-pointer transition-colors
					{filterStatus === 'all' ? 'bg-text-muted/20 text-text border-text-muted/30' : 'bg-transparent text-text-muted border-transparent hover:text-text'}"
				onclick={() => (filterStatus = 'all')}
			>All ({statusCounts.all})</button>
			<button
				class="px-2 py-1 text-xs rounded border cursor-pointer transition-colors
					{filterStatus === 'passed' ? 'bg-success/20 text-success border-success/30' : 'bg-transparent text-text-muted border-transparent hover:text-text'}"
				onclick={() => (filterStatus = 'passed')}
			>Passed ({statusCounts.passed})</button>
			<button
				class="px-2 py-1 text-xs rounded border cursor-pointer transition-colors
					{filterStatus === 'failed' ? 'bg-danger/20 text-danger border-danger/30' : 'bg-transparent text-text-muted border-transparent hover:text-text'}"
				onclick={() => (filterStatus = 'failed')}
			>Failed ({statusCounts.failed})</button>
			<button
				class="px-2 py-1 text-xs rounded border cursor-pointer transition-colors
					{filterStatus === 'errors' ? 'bg-warning/20 text-warning border-warning/30' : 'bg-transparent text-text-muted border-transparent hover:text-text'}"
				onclick={() => (filterStatus = 'errors')}
			>Errors ({statusCounts.errors})</button>
		</div>

		<!-- Results table -->
		<div class="grid grid-cols-[minmax(200px,1fr)_minmax(200px,1fr)_80px_80px] gap-3 px-3 text-xs text-text-muted uppercase items-center">
			<span>Input</span>
			<span>Output</span>
			<span>Score</span>
			<span>Latency</span>
		</div>

		{#if filteredResults.length === 0}
			<div class="text-text-muted text-sm text-center py-8">
				{results.length === 0 ? 'No results yet' : 'No results match this filter'}
			</div>
		{:else}
			<div class="space-y-0">
				{#each filteredResults as result (result.id)}
					<div
						class="border-b border-border/50 hover:bg-bg-secondary transition-colors cursor-pointer"
						onclick={() => (expandedRow = expandedRow === result.id ? null : result.id)}
					>
						<div class="grid grid-cols-[minmax(200px,1fr)_minmax(200px,1fr)_80px_80px] gap-3 items-center px-3 py-2 text-sm">
							<span class="text-text-secondary text-xs truncate font-mono">{truncate(formatJson(result.actual_output), 100)}</span>
							<div class="text-xs">
								<span class="text-text-secondary truncate font-mono block">{truncate(formatJson(result.actual_output), 100)}</span>
							</div>
							<div>
								{#if result.status === 'error'}
									<span class="text-xs px-1.5 py-0.5 bg-warning/20 text-warning border border-warning/30 rounded">err</span>
								{:else}
									<EvalScoreBadge score={result.score} size="xs" />
								{/if}
							</div>
							<span class="text-xs text-text-muted font-mono">{result.latency_ms > 0 ? formatLatency(result.latency_ms) : '\u2014'}</span>
						</div>

						<!-- Expanded view -->
						{#if expandedRow === result.id}
							<div class="px-3 pb-3 space-y-2 bg-bg-secondary">
								<div class="grid grid-cols-2 gap-3">
									<div>
										<div class="text-xs text-text-muted uppercase mb-1">Full Output</div>
										<pre class="text-xs bg-bg-tertiary rounded p-2 overflow-auto max-h-48 font-mono text-text-secondary whitespace-pre-wrap">{formatJson(result.actual_output)}</pre>
									</div>
									<div>
										{#if result.score_reason}
											<div class="text-xs text-text-muted uppercase mb-1">Score Reason</div>
											<pre class="text-xs bg-bg-tertiary rounded p-2 overflow-auto max-h-48 font-mono text-text-secondary whitespace-pre-wrap">{result.score_reason}</pre>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-4 text-xs text-text-muted">
									{#if result.input_tokens}<span>Input tokens: {result.input_tokens}</span>{/if}
									{#if result.output_tokens}<span>Output tokens: {result.output_tokens}</span>{/if}
									{#if result.error}
										<span class="text-danger">Error: {result.error}</span>
									{/if}
									<span>Datapoint: {shortId(result.datapoint_id)}</span>
								</div>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>
