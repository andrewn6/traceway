<script lang="ts">
	import { page } from '$app/state';
	import {
		getComparison,
		getDataset,
		type ComparisonResponse,
		type ComparisonDatapoint,
		type EvalRun,
		type DatasetWithCount
	} from '$lib/api';
	import EvalScoreBadge from '$lib/components/EvalScoreBadge.svelte';
	import { onMount } from 'svelte';

	const datasetId = $derived(page.params.id ?? '');
	const runIds = $derived((page.url.searchParams.get('runs') ?? '').split(',').filter(Boolean));

	let dataset: DatasetWithCount | null = $state(null);
	let comparison: ComparisonResponse | null = $state(null);
	let loading = $state(true);
	let filterMode: 'all' | 'regressions' | 'improvements' | 'disagreements' = $state('all');
	let expandedCell: string | null = $state(null);

	const runColors = ['text-purple-400', 'text-accent', 'text-amber-400', 'text-sky-400'];
	const runBgColors = ['bg-purple-400/10', 'bg-accent/10', 'bg-amber-400/10', 'bg-sky-400/10'];
	const runBorderColors = ['border-purple-400/20', 'border-accent/20', 'border-amber-400/20', 'border-sky-400/20'];

	const filteredDatapoints = $derived.by(() => {
		if (!comparison) return [];
		const dps = comparison.datapoints;
		if (filterMode === 'all') return dps;

		const runs = comparison.runs;
		if (runs.length < 2) return dps;

		const firstRunId = runs[0].id;
		const lastRunId = runs[runs.length - 1].id;

		if (filterMode === 'regressions') {
			return dps.filter((dp) => {
				const first = dp.results[firstRunId];
				const last = dp.results[lastRunId];
				if (!first || !last) return false;
				return (last.score ?? 0) < (first.score ?? 0);
			});
		}
		if (filterMode === 'improvements') {
			return dps.filter((dp) => {
				const first = dp.results[firstRunId];
				const last = dp.results[lastRunId];
				if (!first || !last) return false;
				return (last.score ?? 0) > (first.score ?? 0);
			});
		}
		// disagreements: runs disagree on pass/fail
		return dps.filter((dp) => {
			const statuses = runs.map((r) => dp.results[r.id]?.status).filter(Boolean);
			const passed = statuses.filter((s) => s === 'passed').length;
			const failed = statuses.filter((s) => s === 'failed').length;
			return passed > 0 && failed > 0;
		});
	});

	const filterCounts = $derived.by(() => {
		if (!comparison) return { all: 0, regressions: 0, improvements: 0, disagreements: 0 };
		const dps = comparison.datapoints;
		const runs = comparison.runs;
		const firstRunId = runs[0]?.id;
		const lastRunId = runs[runs.length - 1]?.id;

		return {
			all: dps.length,
			regressions: firstRunId && lastRunId ? dps.filter((dp) => {
				const first = dp.results[firstRunId]; const last = dp.results[lastRunId];
				return first && last && (last.score ?? 0) < (first.score ?? 0);
			}).length : 0,
			improvements: firstRunId && lastRunId ? dps.filter((dp) => {
				const first = dp.results[firstRunId]; const last = dp.results[lastRunId];
				return first && last && (last.score ?? 0) > (first.score ?? 0);
			}).length : 0,
			disagreements: runs.length >= 2 ? dps.filter((dp) => {
				const statuses = runs.map((r) => dp.results[r.id]?.status).filter(Boolean);
				return statuses.filter((s) => s === 'passed').length > 0 && statuses.filter((s) => s === 'failed').length > 0;
			}).length : 0
		};
	});

	async function load() {
		try {
			const [ds, comp] = await Promise.all([
				getDataset(datasetId),
				getComparison(datasetId, runIds)
			]);
			dataset = ds;
			comparison = comp;
		} catch {
			// error
		}
		loading = false;
	}

	onMount(() => { load(); });

	function formatLatency(ms: number): string {
		if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
		return `${ms}ms`;
	}

	function truncate(s: string, len: number): string {
		return s.length > len ? s.slice(0, len) + '...' : s;
	}

	function formatOutput(val: unknown): string {
		if (val === null || val === undefined) return '';
		if (typeof val === 'string') return val;
		return JSON.stringify(val);
	}

	function rowBgClass(dp: ComparisonDatapoint): string {
		if (!comparison || comparison.runs.length < 2) return '';
		const first = dp.results[comparison.runs[0].id];
		const last = dp.results[comparison.runs[comparison.runs.length - 1].id];
		if (!first || !last) return '';
		if ((last.score ?? 0) < (first.score ?? 0)) return 'bg-danger/5';
		if ((last.score ?? 0) > (first.score ?? 0)) return 'bg-success/5';
		return '';
	}

	function totalTokensForRun(run: EvalRun): string {
		if (!comparison) return '\u2014';
		const runResults = comparison.datapoints.map((dp) => dp.results[run.id]).filter(Boolean);
		// We don't have token data in comparison cells, so skip
		return '\u2014';
	}
</script>

<div class="max-w-[1160px] mx-auto space-y-4">
	<div class="grid grid-cols-1 lg:grid-cols-[170px_minmax(0,1fr)] gap-4 items-start">
		<aside class="hidden lg:block">
			<div class="app-toolbar-shell rounded-xl p-2 space-y-1 sticky top-18">
				<a href="/datasets/{datasetId}" class="block px-2 py-1.5 text-xs rounded-lg border border-border/70 bg-bg-tertiary/60 text-text">Dataset</a>
				<a href="/datasets/{datasetId}/compare" class="block px-2 py-1.5 text-xs rounded-lg text-text-muted hover:text-text hover:bg-bg-tertiary/35 transition-colors">Compare runs</a>
			</div>
		</aside>

		<div class="space-y-4">
			<a href="/datasets/{datasetId}" class="text-text-secondary hover:text-text text-sm">&larr; Back to dataset</a>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading comparison...</div>
	{:else if !comparison || comparison.runs.length === 0}
		<div class="text-text-muted text-sm text-center py-8">No comparison data found</div>
	{:else}
		<!-- Header -->
		<div>
			<h1 class="text-lg font-bold">
				Compare: {comparison.runs.map((r) => r.name ?? r.config.model).join('  vs  ')}
			</h1>
			{#if dataset}
				<p class="text-xs text-text-muted mt-1">Dataset: {dataset.name} ({dataset.datapoint_count} datapoints)</p>
			{/if}
		</div>

		<!-- Summary cards -->
		<div class="grid gap-3" style="grid-template-columns: repeat({comparison.runs.length}, minmax(0, 1fr));">
			{#each comparison.runs as run, i (run.id)}
				<div class="bg-bg-secondary border rounded p-3 {runBorderColors[i % runColors.length]}">
					<div class="text-sm font-semibold {runColors[i % runColors.length]} mb-2">{run.name ?? run.config.model}</div>
					<div class="space-y-1 text-xs">
						<div class="flex justify-between">
							<span class="text-text-muted">Score:</span>
							<EvalScoreBadge score={run.results.scores.mean} size="xs" />
						</div>
						<div class="flex justify-between">
							<span class="text-text-muted">Pass:</span>
							<span class="text-text font-mono">{run.results.scores.pass_rate != null ? `${Math.round(run.results.scores.pass_rate * 100)}%` : '\u2014'}</span>
						</div>
						<div class="flex justify-between">
							<span class="text-text-muted">Completed:</span>
							<span class="text-text font-mono">{run.results.completed}/{run.results.total}</span>
						</div>
					</div>
				</div>
			{/each}
		</div>

		<!-- Filter pills -->
		<div class="app-toolbar-shell rounded-xl p-2 flex items-center gap-1.5 flex-wrap">
			<button
				class="query-chip {filterMode === 'all' ? 'query-chip-active' : ''}"
				onclick={() => (filterMode = 'all')}
			>All ({filterCounts.all})</button>
			<button
				class="query-chip {filterMode === 'regressions' ? 'query-chip-active' : ''}"
				onclick={() => (filterMode = 'regressions')}
			>Regressions ({filterCounts.regressions})</button>
			<button
				class="query-chip {filterMode === 'improvements' ? 'query-chip-active' : ''}"
				onclick={() => (filterMode = 'improvements')}
			>Improvements ({filterCounts.improvements})</button>
			<button
				class="query-chip {filterMode === 'disagreements' ? 'query-chip-active' : ''}"
				onclick={() => (filterMode = 'disagreements')}
			>Disagreements ({filterCounts.disagreements})</button>
		</div>

		<!-- Comparison table -->
		<div class="table-float overflow-x-auto">
			<div class="min-w-[800px]">
				<!-- Header -->
				<div class="grid gap-3 px-3 text-xs text-text-muted uppercase items-center"
					style="grid-template-columns: minmax(180px, 1fr) {comparison.runs.map(() => 'minmax(150px, 1fr)').join(' ')};">
					<span>Input</span>
					{#each comparison.runs as run, i (run.id)}
						<span class="{runColors[i % runColors.length]}">{run.name ?? run.config.model}</span>
					{/each}
				</div>

				<!-- Rows -->
				{#if filteredDatapoints.length === 0}
					<div class="text-text-muted text-sm text-center py-8">No datapoints match this filter</div>
				{:else}
					{#each filteredDatapoints as dp (dp.datapoint_id)}
						<div class="border-b border-border/50 {rowBgClass(dp)}">
							<div class="grid gap-3 items-start px-3 py-2 text-sm"
								style="grid-template-columns: minmax(180px, 1fr) {comparison.runs.map(() => 'minmax(150px, 1fr)').join(' ')};">
								<!-- Input column -->
								<div class="text-xs text-text-secondary font-mono truncate">
									{truncate(formatOutput(dp.input), 120)}
								</div>

								<!-- Run result cells -->
								{#each comparison.runs as run, i (run.id)}
									{@const cell = dp.results[run.id]}
									{#if cell}
										<div
											class="text-xs p-1.5 rounded {runBgColors[i % runBgColors.length]} cursor-pointer"
											role="button"
											tabindex={0}
											aria-label={`Toggle details for ${run.name ?? run.id}`}
											onclick={() => (expandedCell = expandedCell === `${dp.datapoint_id}-${run.id}` ? null : `${dp.datapoint_id}-${run.id}`)}
											onkeydown={(e) => {
												if (e.key === 'Enter' || e.key === ' ') {
													e.preventDefault();
													expandedCell = expandedCell === `${dp.datapoint_id}-${run.id}` ? null : `${dp.datapoint_id}-${run.id}`;
												}
											}}
										>
											<div class="flex items-center gap-1 mb-0.5">
												{#if cell.status === 'passed'}
													<span class="text-success">&check;</span>
												{:else if cell.status === 'failed'}
													<span class="text-danger">&cross;</span>
												{:else}
													<span class="text-warning">!</span>
												{/if}
												<EvalScoreBadge score={cell.score} size="xs" />
												<span class="text-text-muted ml-auto">{formatLatency(cell.latency_ms)}</span>
											</div>
											<div class="text-text-secondary font-mono truncate">{truncate(formatOutput(cell.output), 80)}</div>

											{#if expandedCell === `${dp.datapoint_id}-${run.id}`}
												<div class="mt-2 p-2 bg-bg-tertiary rounded">
													<div class="text-text-muted uppercase mb-1">Full Output</div>
													<pre class="whitespace-pre-wrap text-text-secondary max-h-48 overflow-auto">{formatOutput(cell.output)}</pre>
												</div>
											{/if}
										</div>
									{:else}
										<div class="text-xs text-text-muted p-1.5">&mdash;</div>
									{/if}
								{/each}
							</div>
						</div>
					{/each}
				{/if}
			</div>
		</div>
		{/if}
		</div>
	</div>
</div>
