<script lang="ts">
	import { queryAnalytics, type AnalyticsQuery, type GroupByField } from '$lib/api';
	import SparklineChart from './SparklineChart.svelte';
	import HorizontalBarList from './HorizontalBarList.svelte';

	let {
		open = $bindable(false),
		editConfig = null,
		timeRange = '1d',
		onSave = (_config: CustomChartConfig) => {},
	}: {
		open: boolean;
		editConfig?: CustomChartConfig | null;
		timeRange?: string;
		onSave?: (config: CustomChartConfig) => void;
	} = $props();

	export interface CustomChartConfig {
		id: string;
		name: string;
		chart_type: 'line' | 'bar' | 'horizontal_bar';
		metric: string;
		aggregation: string;
		group_by: string;
	}

	let name = $state('');
	let chartType: 'line' | 'bar' | 'horizontal_bar' = $state('line');
	let metric = $state('total_cost');
	let aggregation = $state('sum');
	let groupBy = $state('none');

	let previewData: number[] = $state([]);
	let previewLabels: string[] = $state([]);
	let previewBarItems: { label: string; value: number }[] = $state([]);
	let previewLoading = $state(false);

	const metrics = [
		{ key: 'total_cost', label: 'Total Cost' },
		{ key: 'total_tokens', label: 'Tokens' },
		{ key: 'avg_latency_ms', label: 'Avg Latency' },
		{ key: 'span_count', label: 'Span Count' },
		{ key: 'error_count', label: 'Error Count' },
	];

	const aggregations = [
		{ key: 'sum', label: 'Sum' },
		{ key: 'avg', label: 'Average' },
		{ key: 'min', label: 'Min' },
		{ key: 'max', label: 'Max' },
		{ key: 'p90', label: 'P90' },
		{ key: 'p95', label: 'P95' },
		{ key: 'p99', label: 'P99' },
	];

	const groupByOptions = [
		{ key: 'none', label: 'None' },
		{ key: 'model', label: 'Model' },
		{ key: 'provider', label: 'Provider' },
		{ key: 'kind', label: 'Kind' },
		{ key: 'status', label: 'Status' },
	];

	// Initialize from editConfig when opening
	$effect(() => {
		if (open && editConfig) {
			name = editConfig.name;
			chartType = editConfig.chart_type;
			metric = editConfig.metric;
			aggregation = editConfig.aggregation;
			groupBy = editConfig.group_by;
		} else if (open && !editConfig) {
			name = '';
			chartType = 'line';
			metric = 'total_cost';
			aggregation = 'sum';
			groupBy = 'none';
		}
	});

	// Live preview
	$effect(() => {
		if (!open) return;
		void metric;
		void groupBy;
		void chartType;
		loadPreview();
	});

	async function loadPreview() {
		previewLoading = true;
		try {
			const ms: Record<string, number> = {
				'1h': 60 * 60 * 1000,
				'6h': 6 * 60 * 60 * 1000,
				'1d': 24 * 60 * 60 * 1000,
				'7d': 7 * 24 * 60 * 60 * 1000,
				'30d': 30 * 24 * 60 * 60 * 1000,
			};
			const now = new Date();
			const since = new Date(now.getTime() - (ms[timeRange] ?? ms['1d'])).toISOString();
			const until = now.toISOString();

			const gb: GroupByField[] = groupBy === 'none' ? ['hour'] : [groupBy as GroupByField];
			const result = await queryAnalytics({
				filter: { since, until },
				group_by: gb,
				metrics: [metric],
			});

			if (groupBy === 'none' || chartType === 'line') {
				const sorted = [...result.groups].sort((a: any, b: any) => {
					const aKey = Object.values(a.key)[0] ?? '';
					const bKey = Object.values(b.key)[0] ?? '';
					return String(aKey).localeCompare(String(bKey));
				});
				previewData = sorted.map((g: any) => (g.metrics as any)[metric] ?? 0);
				previewLabels = sorted.map((g: any) => Object.values(g.key)[0] as string ?? '');
			}

			if (chartType === 'horizontal_bar' || chartType === 'bar') {
				previewBarItems = result.groups
					.map((g: any) => ({
						label: Object.values(g.key)[0] as string ?? 'unknown',
						value: (g.metrics as any)[metric] ?? 0,
					}))
					.sort((a: any, b: any) => b.value - a.value)
					.slice(0, 10);
			}
		} catch {
			previewData = [];
			previewBarItems = [];
		}
		previewLoading = false;
	}

	function handleSave() {
		const config: CustomChartConfig = {
			id: editConfig?.id ?? crypto.randomUUID(),
			name: name.trim() || `${metrics.find(m => m.key === metric)?.label} chart`,
			chart_type: chartType,
			metric,
			aggregation,
			group_by: groupBy,
		};
		onSave(config);
		open = false;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
		onclick={(e) => { if (e.target === e.currentTarget) open = false; }}
		onkeydown={handleKeydown}
	>
		<div class="bg-bg-secondary border border-border rounded-lg shadow-xl w-full max-w-lg mx-4">
			<div class="px-5 pt-5 pb-3 border-b border-border">
				<h3 class="text-sm font-semibold text-text">{editConfig ? 'Edit Chart' : 'Add Chart'}</h3>
				<p class="text-xs text-text-muted mt-1">Configure a custom analytics chart.</p>
			</div>

			<div class="px-5 py-4 space-y-4">
				<!-- Name -->
				<div>
					<label for="chart-name" class="block text-xs font-medium text-text-secondary mb-1">Name</label>
					<input
						id="chart-name"
						type="text"
						bind:value={name}
						placeholder="e.g. Cost over time"
						class="control-input"
					/>
				</div>

				<!-- Chart type -->
				<div>
					<span class="block text-xs font-medium text-text-secondary mb-1.5">Chart type</span>
					<div class="flex gap-1">
						{#each [['line', 'Line'], ['bar', 'Bar'], ['horizontal_bar', 'Horizontal Bar']] as [key, label]}
							<button
								class="px-3 py-1.5 text-xs rounded-md border transition-colors cursor-pointer
									{chartType === key ? 'bg-accent text-white border-accent' : 'bg-bg-tertiary text-text-secondary border-border hover:text-text'}"
								onclick={() => (chartType = key as any)}
							>
								{label}
							</button>
						{/each}
					</div>
				</div>

				<!-- Metric & Aggregation -->
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label for="chart-metric" class="block text-xs font-medium text-text-secondary mb-1">Metric</label>
						<select id="chart-metric" bind:value={metric} class="control-select">
							{#each metrics as m}
								<option value={m.key}>{m.label}</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="chart-agg" class="block text-xs font-medium text-text-secondary mb-1">Aggregation</label>
						<select id="chart-agg" bind:value={aggregation} class="control-select">
							{#each aggregations as a}
								<option value={a.key}>{a.label}</option>
							{/each}
						</select>
					</div>
				</div>

				<!-- Group by -->
				<div>
					<label for="chart-group" class="block text-xs font-medium text-text-secondary mb-1">Group by</label>
					<select id="chart-group" bind:value={groupBy} class="control-select">
						{#each groupByOptions as g}
							<option value={g.key}>{g.label}</option>
						{/each}
					</select>
				</div>

				<!-- Preview -->
				<div>
					<div class="text-xs font-medium text-text-secondary mb-2">Preview</div>
					<div class="bg-bg border border-border rounded-md p-3 min-h-[120px]">
						{#if previewLoading}
							<div class="flex items-center justify-center h-[100px] text-text-muted text-xs">Loading...</div>
						{:else if chartType === 'line' && previewData.length > 1}
							<SparklineChart points={previewData} labels={previewLabels} unit="" color="#2f9c88" height={100} />
						{:else if (chartType === 'horizontal_bar' || chartType === 'bar') && previewBarItems.length > 0}
							<HorizontalBarList items={previewBarItems} accentColor="#2f9c88" maxItems={5} />
						{:else}
							<div class="flex items-center justify-center h-[100px] text-text-muted text-xs">No preview data</div>
						{/if}
					</div>
				</div>
			</div>

			<div class="px-5 py-3 border-t border-border flex justify-end gap-2">
				<button
					onclick={() => (open = false)}
					class="btn-ghost text-xs cursor-pointer"
				>
					Cancel
				</button>
				<button
					onclick={handleSave}
					class="btn-primary text-xs cursor-pointer"
				>
					{editConfig ? 'Save Changes' : 'Add Chart'}
				</button>
			</div>
		</div>
	</div>
{/if}
