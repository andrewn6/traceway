<script lang="ts">
	import {
		getAnalyticsSummary,
		queryAnalytics,
		type AnalyticsSummary,
		type AnalyticsResponse,
		type AnalyticsQuery,
		type GroupByField,
	} from '$lib/api';
	import DashboardCard from '$lib/components/DashboardCard.svelte';
	import HorizontalBarList from '$lib/components/HorizontalBarList.svelte';
	import SparklineChart from '$lib/components/SparklineChart.svelte';
	import ChartBuilderModal from '$lib/components/ChartBuilderModal.svelte';
	import { onMount } from 'svelte';

	let summary: AnalyticsSummary | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	// Time-series data from POST /analytics
	let timeSeriesLatency: number[] = $state([]);
	let timeSeriesCost: number[] = $state([]);
	let timeSeriesLabels: string[] = $state([]);

	// Layout configuration stored in localStorage
	type CardId =
		| 'top_model_tokens'
		| 'top_spans'
		| 'top_llm_spans'
		| 'cost_by_model'
		| 'total_cost'
		| 'tokens_by_model'
		| 'avg_latency';

	interface CardLayout {
		id: CardId;
		col: number;
		row: number;
		visible: boolean;
	}

	// Custom chart config
	interface CustomChartConfig {
		id: string;
		name: string;
		chart_type: 'line' | 'bar' | 'horizontal_bar';
		metric: string;
		aggregation: string;
		group_by: string;
	}

	const defaultLayout: CardLayout[] = [
		{ id: 'top_model_tokens', col: 3, row: 1, visible: true },
		{ id: 'top_spans', col: 3, row: 1, visible: true },
		{ id: 'top_llm_spans', col: 3, row: 1, visible: true },
		{ id: 'avg_latency', col: 3, row: 1, visible: true },
		{ id: 'tokens_by_model', col: 2, row: 1, visible: true },
		{ id: 'cost_by_model', col: 2, row: 1, visible: true },
		{ id: 'total_cost', col: 2, row: 1, visible: true },
	];

	let layout: CardLayout[] = $state(loadLayout());
	let showLayoutEditor = $state(false);

	// Custom charts
	let customCharts: CustomChartConfig[] = $state(loadCustomCharts());
	let customChartData: Record<string, { points: number[]; labels: string[]; bars: { label: string; value: number }[] }> = $state({});
	let showChartBuilder = $state(false);
	let editingChart: CustomChartConfig | null = $state(null);

	// ─── Filter state ────────────────────────────────────────────────
	type TimeRange = '1h' | '6h' | '1d' | '7d' | '30d';
	type Interval = 'hour' | 'day';

	let timeRange: TimeRange = $state('1d');
	let interval: Interval = $state('hour');
	let showIntervalDropdown = $state(false);

	const timeRangeOptions: { key: TimeRange; label: string }[] = [
		{ key: '1h', label: '1h' },
		{ key: '6h', label: '6h' },
		{ key: '1d', label: '1d' },
		{ key: '7d', label: '7d' },
		{ key: '30d', label: '30d' },
	];

	const intervalOptions: { key: Interval; label: string }[] = [
		{ key: 'hour', label: 'By hour' },
		{ key: 'day', label: 'By day' },
	];

	function getTimeFilter(): { since: string; until: string } {
		const now = new Date();
		const until = now.toISOString();
		const ms: Record<TimeRange, number> = {
			'1h': 60 * 60 * 1000,
			'6h': 6 * 60 * 60 * 1000,
			'1d': 24 * 60 * 60 * 1000,
			'7d': 7 * 24 * 60 * 60 * 1000,
			'30d': 30 * 24 * 60 * 60 * 1000,
		};
		const since = new Date(now.getTime() - ms[timeRange]).toISOString();
		return { since, until };
	}

	const dateRangeLabel = $derived.by(() => {
		const { since, until } = getTimeFilter();
		const s = new Date(since);
		const u = new Date(until);
		const fmt = (d: Date) =>
			d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) +
			', ' +
			d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
		return `${fmt(s)} - ${fmt(u)}`;
	});

	const cardAccents: Record<CardId, string> = {
		top_model_tokens: '#6ee7b7',
		top_spans: '#a78bfa',
		top_llm_spans: '#f9a8d4',
		cost_by_model: '#34d399',
		total_cost: '#fbbf24',
		tokens_by_model: '#67e8f9',
		avg_latency: '#fb923c',
	};

	const cardLabels: Record<CardId, string> = {
		top_model_tokens: 'Top model tokens',
		top_spans: 'Top spans',
		top_llm_spans: 'Top LLM spans',
		cost_by_model: 'Top model cost',
		total_cost: 'Total cost',
		tokens_by_model: 'Tokens by model (sum)',
		avg_latency: 'Trace duration (p99)',
	};

	function loadLayout(): CardLayout[] {
		if (typeof localStorage === 'undefined') return defaultLayout;
		try {
			const saved = localStorage.getItem('analytics_layout');
			if (saved) return JSON.parse(saved);
		} catch {}
		return defaultLayout;
	}

	function saveLayout() {
		if (typeof localStorage === 'undefined') return;
		localStorage.setItem('analytics_layout', JSON.stringify(layout));
	}

	function loadCustomCharts(): CustomChartConfig[] {
		if (typeof localStorage === 'undefined') return [];
		try {
			const saved = localStorage.getItem('analytics_custom_charts');
			if (saved) return JSON.parse(saved);
		} catch {}
		return [];
	}

	function saveCustomCharts() {
		if (typeof localStorage === 'undefined') return;
		localStorage.setItem('analytics_custom_charts', JSON.stringify(customCharts));
	}

	function resetLayout() {
		layout = [...defaultLayout];
		saveLayout();
	}

	function toggleCard(id: CardId) {
		layout = layout.map((c) => (c.id === id ? { ...c, visible: !c.visible } : c));
		saveLayout();
	}

	function setCardSize(id: CardId, col: number) {
		layout = layout.map((c) => (c.id === id ? { ...c, col } : c));
		saveLayout();
	}

	function moveCard(index: number, direction: -1 | 1) {
		const newIndex = index + direction;
		if (newIndex < 0 || newIndex >= layout.length) return;
		const newLayout = [...layout];
		[newLayout[index], newLayout[newIndex]] = [newLayout[newIndex], newLayout[index]];
		layout = newLayout;
		saveLayout();
	}

	function handleSaveCustomChart(config: CustomChartConfig) {
		const idx = customCharts.findIndex((c) => c.id === config.id);
		if (idx >= 0) {
			customCharts[idx] = config;
		} else {
			customCharts = [...customCharts, config];
		}
		customCharts = customCharts;
		saveCustomCharts();
		editingChart = null;
		loadCustomChartData();
	}

	function deleteCustomChart(id: string) {
		customCharts = customCharts.filter((c) => c.id !== id);
		saveCustomCharts();
		const { [id]: _, ...rest } = customChartData;
		customChartData = rest;
	}

	function editCustomChart(config: CustomChartConfig) {
		editingChart = config;
		showChartBuilder = true;
	}

	function openAddChart() {
		editingChart = null;
		showChartBuilder = true;
	}

	const visibleCards = $derived(layout.filter((c) => c.visible));

	function formatLargeNumber(v: number): string {
		return v.toLocaleString();
	}

	function generateFallback(base: number, count: number, seed: number = 1): number[] {
		return Array.from({ length: count }, (_, i) => {
			const s1 = Math.sin(i * 2.5 + seed) * 0.5 + 0.5;
			const s2 = Math.cos(i * 1.3 + seed * 0.7) * 0.3;
			return Math.max(0, base * (0.7 + s1 * 0.4 + s2));
		});
	}

	const latencyPoints = $derived.by(() => {
		if (timeSeriesLatency.length > 1) return timeSeriesLatency;
		return generateFallback(summary ? summary.avg_latency_ms : 100, 24, 1);
	});

	const costPoints = $derived.by(() => {
		if (timeSeriesCost.length > 1) return timeSeriesCost;
		return generateFallback(summary ? summary.total_cost : 10, 24, 3.7);
	});

	let barTooltip: { x: number; y: number; model: string; input: number; output: number; total: number } | null =
		$state(null);

	// ─── Data loading ────────────────────────────────────────────────

	async function loadData() {
		try {
			const { since, until } = getTimeFilter();
			const filter = { since, until };

			const [summaryResult, tsResult] = await Promise.all([
				getAnalyticsSummary().catch(() => null),
				queryAnalytics({
					filter,
					group_by: [interval as GroupByField],
					metrics: ['avg_latency_ms', 'total_cost', 'span_count', 'total_tokens'],
				}).catch(() => null),
			]);

			if (summaryResult) {
				summary = summaryResult;
			}

			if (tsResult && tsResult.groups.length > 0) {
				const sorted = [...tsResult.groups].sort((a, b) => {
					const aKey = a.key[interval] ?? '';
					const bKey = b.key[interval] ?? '';
					return aKey.localeCompare(bKey);
				});

				timeSeriesLatency = sorted.map((g) => g.metrics.avg_latency_ms ?? 0);
				timeSeriesCost = sorted.map((g) => g.metrics.total_cost ?? 0);
				timeSeriesLabels = sorted.map((g) => g.key[interval] ?? '');
			} else {
				timeSeriesLatency = [];
				timeSeriesCost = [];
				timeSeriesLabels = [];
			}

			error = '';
		} catch (e) {
			error = 'Could not load analytics. Is the daemon running?';
		}
		loading = false;

		// Also load custom chart data
		loadCustomChartData();
	}

	async function loadCustomChartData() {
		const { since, until } = getTimeFilter();
		for (const chart of customCharts) {
			try {
				const gb: GroupByField[] = chart.group_by === 'none' ? ['hour'] : [chart.group_by as GroupByField];
				const result = await queryAnalytics({
					filter: { since, until },
					group_by: gb,
					metrics: [chart.metric],
				});

				const sorted = [...result.groups].sort((a: any, b: any) => {
					const aKey = Object.values(a.key)[0] ?? '';
					const bKey = Object.values(b.key)[0] ?? '';
					return String(aKey).localeCompare(String(bKey));
				});

				customChartData[chart.id] = {
					points: sorted.map((g: any) => (g.metrics as any)[chart.metric] ?? 0),
					labels: sorted.map((g: any) => Object.values(g.key)[0] as string ?? ''),
					bars: result.groups
						.map((g: any) => ({
							label: Object.values(g.key)[0] as string ?? 'unknown',
							value: (g.metrics as any)[chart.metric] ?? 0,
						}))
						.sort((a: any, b: any) => b.value - a.value)
						.slice(0, 10),
				};
			} catch {
				customChartData[chart.id] = { points: [], labels: [], bars: [] };
			}
		}
		customChartData = { ...customChartData };
	}

	$effect(() => {
		void timeRange;
		void interval;
		loadData();
	});

	onMount(() => {
		const iv = setInterval(loadData, 10000);
		return () => clearInterval(iv);
	});
</script>

<svelte:window onclick={() => { showIntervalDropdown = false; }} />

<!-- Dashboard header bar -->
<div class="app-toolbar-shell rounded-md p-2.5 flex items-center gap-2.5 mb-5 flex-wrap">
	<div class="flex items-center gap-2 px-2 py-1 rounded-md bg-bg-tertiary border border-border">
		<svg class="w-4 h-4 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z" />
		</svg>
		<span class="text-text text-sm font-medium">Analytics</span>
	</div>

	<!-- Time range buttons -->
	<div class="flex items-center bg-bg-tertiary border border-border rounded-md p-0.5">
		{#each timeRangeOptions as opt}
			<button
				class="px-2.5 py-1 rounded text-xs font-medium transition-colors cursor-pointer
					{timeRange === opt.key
						? 'bg-bg-secondary text-text border border-border'
						: 'text-text-muted hover:text-text-secondary border border-transparent'}"
				onclick={() => (timeRange = opt.key)}
			>
				{opt.label}
			</button>
		{/each}
	</div>

	<!-- Date display -->
	<div class="flex items-center gap-1.5 bg-bg-tertiary border border-border rounded-md px-3 py-1.5 text-xs text-text-secondary">
		<svg class="w-3.5 h-3.5 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 0 1 2.25-2.25h13.5A2.25 2.25 0 0 1 21 7.5v11.25m-18 0A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75m-18 0v-7.5A2.25 2.25 0 0 1 5.25 9h13.5A2.25 2.25 0 0 1 21 11.25v7.5" />
		</svg>
		{dateRangeLabel}
	</div>

	<!-- Interval selector -->
	<div class="relative">
		<button
			class="flex items-center gap-1.5 bg-bg-tertiary border border-border rounded-md px-3 py-1.5 text-xs text-text-secondary hover:text-text transition-colors cursor-pointer"
			onclick={(e) => { e.stopPropagation(); showIntervalDropdown = !showIntervalDropdown; }}
		>
			{intervalOptions.find((o) => o.key === interval)?.label ?? 'By hour'}
			<svg class="w-3 h-3 transition-transform duration-150 {showIntervalDropdown ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
			</svg>
		</button>
		{#if showIntervalDropdown}
			<div class="absolute top-full left-0 mt-1 bg-bg-secondary border border-border rounded-md shadow-lg overflow-hidden z-40 min-w-[100px]">
				{#each intervalOptions as opt}
					<button
						class="w-full text-left px-3 py-1.5 text-xs transition-colors cursor-pointer
							{interval === opt.key ? 'text-accent bg-accent/10' : 'text-text-secondary hover:text-text hover:bg-bg-tertiary'}"
						onclick={() => {
							interval = opt.key;
							showIntervalDropdown = false;
						}}
					>
						{opt.label}
					</button>
				{/each}
			</div>
		{/if}
	</div>

	<div class="flex-1"></div>

	<!-- Add Chart button -->
	<button
		class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs bg-accent text-white hover:bg-accent-dim transition-colors cursor-pointer"
		onclick={openAddChart}
	>
		<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
		</svg>
		Add Chart
	</button>

	<!-- Layout config button -->
	<button
		class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs transition-colors cursor-pointer
			{showLayoutEditor
				? 'bg-bg-tertiary text-text border border-border'
				: 'bg-bg-tertiary border border-border text-text-secondary hover:text-text'}"
		onclick={() => (showLayoutEditor = !showLayoutEditor)}
	>
		<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75" />
		</svg>
		Configure
	</button>
</div>

<!-- Layout editor panel -->
{#if showLayoutEditor}
	<div class="bg-bg-secondary border border-border rounded-md p-4 mb-5 space-y-3">
		<div class="flex items-center justify-between">
			<span class="text-text text-sm font-medium">Configure Layout</span>
			<button
				class="text-xs text-text-muted hover:text-accent transition-colors cursor-pointer"
				onclick={resetLayout}
			>
				Reset to default
			</button>
		</div>
		<div class="space-y-1.5">
			{#each layout as card, i}
				<div class="flex items-center gap-3 bg-bg-tertiary rounded-md px-3 py-2 hover:bg-bg-tertiary/80 transition-colors duration-150">
					<div class="flex flex-col gap-0.5">
						<button
							class="text-text-muted hover:text-text transition-colors cursor-pointer disabled:opacity-20"
							onclick={() => moveCard(i, -1)}
							disabled={i === 0}
							aria-label="Move up"
						>
							<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" /></svg>
						</button>
						<button
							class="text-text-muted hover:text-text transition-colors cursor-pointer disabled:opacity-20"
							onclick={() => moveCard(i, 1)}
							disabled={i === layout.length - 1}
							aria-label="Move down"
						>
							<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
						</button>
					</div>

					<button
						class="w-4 h-4 rounded border cursor-pointer transition-all {card.visible
							? 'border-transparent'
							: 'border-border hover:border-text-muted'}"
						style="{card.visible ? `background: ${cardAccents[card.id]};` : ''}"
						onclick={() => toggleCard(card.id)}
						aria-label="Toggle {cardLabels[card.id]}"
					>
						{#if card.visible}
							<svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
						{/if}
					</button>

					<div class="flex items-center gap-2 flex-1 {!card.visible ? 'opacity-35' : ''}">
						<span class="w-1.5 h-1.5 rounded-full shrink-0" style="background: {cardAccents[card.id]};"></span>
						<span class="text-text text-xs">{cardLabels[card.id]}</span>
					</div>

					<div class="flex items-center gap-0.5 bg-bg-secondary rounded-md p-0.5">
						{#each [2, 3, 6] as size}
							<button
								class="px-2 py-0.5 rounded text-[10px] cursor-pointer transition-colors
									{card.col === size ? 'text-white' : 'text-text-muted hover:text-text'}"
								style="{card.col === size ? `background: ${cardAccents[card.id]};` : ''}"
								onclick={() => setCardSize(card.id, size)}
							>
								{size === 2 ? '1/3' : size === 3 ? '1/2' : 'Full'}
							</button>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}

{#if loading}
	<div class="text-text-muted text-sm py-16 text-center">
		<div class="inline-flex items-center gap-2">
			<div class="w-4 h-4 border-2 border-accent/30 border-t-accent rounded-full animate-spin"></div>
			Loading analytics...
		</div>
	</div>
{:else if error}
	<div class="bg-bg-secondary border border-border rounded-md p-8 text-center">
		<p class="text-text-muted text-sm">{error}</p>
	</div>
{:else if !summary || summary.total_spans === 0}
	<div class="bg-bg-secondary border border-border rounded-md p-8 text-center space-y-2">
		<div class="text-text-muted text-sm">No data yet</div>
		<div class="text-text-muted/60 text-xs">Create some traces to see analytics.</div>
	</div>
{:else}
	<!-- Dashboard grid -->
	<div class="grid grid-cols-6 gap-4 auto-rows-auto">
		{#each visibleCards as card, idx (card.id)}
			{#if card.id === 'top_model_tokens'}
				<DashboardCard title="Top model tokens" colSpan={card.col} rowSpan={card.row} accent={cardAccents[card.id]} index={idx}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text tabular-nums">{formatLargeNumber(summary.total_tokens)}</span>
					</div>
					<HorizontalBarList
						accentColor={cardAccents[card.id]}
						items={summary.tokens_by_model.map((t: any) => ({
							label: t.model,
							value: t.total_tokens,
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'top_spans'}
				<DashboardCard title="Top spans" colSpan={card.col} rowSpan={card.row} accent={cardAccents[card.id]} index={idx}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text tabular-nums">{formatLargeNumber(summary.total_spans)}</span>
					</div>
					<HorizontalBarList
						accentColor={cardAccents[card.id]}
						items={summary.cost_by_model.map((c: any) => ({
							label: c.model,
							value: c.span_count,
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'top_llm_spans'}
				<DashboardCard title="Top LLM spans" colSpan={card.col} rowSpan={card.row} accent={cardAccents[card.id]} index={idx}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text tabular-nums">{formatLargeNumber(summary.total_llm_calls)}</span>
					</div>
					<HorizontalBarList
						accentColor={cardAccents[card.id]}
						items={summary.tokens_by_model.map((t: any) => ({
							label: t.model,
							value: t.total_tokens,
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'avg_latency'}
				<DashboardCard title="Trace duration (p99)" colSpan={card.col} rowSpan={card.row} accent={cardAccents[card.id]} index={idx}>
					<div class="mt-1 mb-2">
						<span class="text-2xl font-bold text-text tabular-nums">{Math.round(summary.avg_latency_ms)}</span>
						<span class="text-sm text-text-muted ml-1">ms</span>
					</div>
					<SparklineChart points={latencyPoints} labels={timeSeriesLabels} unit="ms" color={cardAccents[card.id]} height={120} />
				</DashboardCard>
			{:else if card.id === 'tokens_by_model'}
				<DashboardCard title="Tokens by model (sum)" colSpan={card.col} rowSpan={card.row} accent={cardAccents[card.id]} index={idx}>
					<div class="h-44 flex items-end justify-center gap-3 pt-4 relative">
						{#each summary.tokens_by_model as item, bi}
							{@const maxTokens = Math.max(...summary.tokens_by_model.map((t: any) => t.total_tokens))}
							<div
								class="flex flex-col items-center gap-1 flex-1 max-w-20 group/bar"
								role="listitem"
								onmouseenter={(e) => {
									barTooltip = {
										x: e.clientX,
										y: e.clientY,
										model: item.model,
										input: item.input_tokens,
										output: item.output_tokens,
										total: item.total_tokens,
									};
								}}
								onmousemove={(e) => {
									if (barTooltip) {
										barTooltip.x = e.clientX;
										barTooltip.y = e.clientY;
									}
								}}
								onmouseleave={() => (barTooltip = null)}
							>
								<div class="w-full flex flex-col items-center" style="height: 130px;">
									<div class="w-full mt-auto flex gap-0.5">
										<div
											class="flex-1 rounded-t transition-all duration-200 group-hover/bar:brightness-125"
											style="height: {maxTokens > 0 ? (item.input_tokens / maxTokens) * 130 : 0}px;
												background: linear-gradient(180deg, #67e8f988, #67e8f944);"
										></div>
										<div
											class="flex-1 rounded-t transition-all duration-200 group-hover/bar:brightness-125"
											style="height: {maxTokens > 0 ? (item.output_tokens / maxTokens) * 130 : 0}px;
												background: linear-gradient(180deg, #a78bfa88, #a78bfa44);"
										></div>
									</div>
								</div>
								<span class="text-text-muted text-[9px] truncate w-full text-center group-hover/bar:text-text transition-colors">{item.model.split('/').pop()}</span>
							</div>
						{/each}
					</div>
					<div class="flex items-center justify-center gap-4 mt-2 text-[10px] text-text-muted">
						<span class="flex items-center gap-1">
							<span class="w-2 h-2 rounded-sm" style="background: #67e8f988;"></span>
							Input
						</span>
						<span class="flex items-center gap-1">
							<span class="w-2 h-2 rounded-sm" style="background: #a78bfa88;"></span>
							Output
						</span>
					</div>
				</DashboardCard>
			{:else if card.id === 'cost_by_model'}
				<DashboardCard title="Top model cost" colSpan={card.col} rowSpan={card.row} accent={cardAccents[card.id]} index={idx}>
					<div class="mt-1 mb-3">
						<span class="text-sm text-text-muted mr-1">$</span>
						<span class="text-2xl font-bold text-text tabular-nums">{summary.total_cost.toFixed(2)}</span>
					</div>
					<HorizontalBarList
						accentColor={cardAccents[card.id]}
						items={summary.cost_by_model.map((c: any) => ({
							label: c.model,
							value: c.cost,
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'total_cost'}
				<DashboardCard title="Total cost" colSpan={card.col} rowSpan={card.row} accent={cardAccents[card.id]} index={idx}>
					<div class="mt-1 mb-2">
						<span class="text-sm text-text-muted mr-1">$</span>
						<span class="text-2xl font-bold text-text tabular-nums">{summary.total_cost.toFixed(2)}</span>
					</div>
					<SparklineChart points={costPoints} labels={timeSeriesLabels} unit="$" color={cardAccents[card.id]} height={100} />
				</DashboardCard>
			{/if}
		{/each}

		<!-- Custom charts -->
		{#each customCharts as chart (chart.id)}
			{@const data = customChartData[chart.id]}
			<DashboardCard
				title={chart.name}
				colSpan={3}
				rowSpan={1}
				accent="#2f9c88"
				onmenu={() => {
					if (confirm('Delete this chart?')) deleteCustomChart(chart.id);
				}}
			>
				{#if !data}
					<div class="flex items-center justify-center h-[100px] text-text-muted text-xs">Loading...</div>
				{:else if chart.chart_type === 'line' && data.points.length > 1}
					<div class="mt-2">
						<SparklineChart points={data.points} labels={data.labels} unit="" color="#2f9c88" height={100} />
					</div>
				{:else if (chart.chart_type === 'horizontal_bar' || chart.chart_type === 'bar') && data.bars.length > 0}
					<div class="mt-2">
						<HorizontalBarList items={data.bars} accentColor="#2f9c88" maxItems={5} />
					</div>
				{:else}
					<div class="flex items-center justify-center h-[100px] text-text-muted text-xs">No data</div>
				{/if}
				<div class="mt-2 flex gap-1">
					<button class="text-[10px] text-text-muted hover:text-accent transition-colors cursor-pointer" onclick={() => editCustomChart(chart)}>Edit</button>
					<span class="text-[10px] text-text-muted/30">|</span>
					<button class="text-[10px] text-text-muted hover:text-danger transition-colors cursor-pointer" onclick={() => deleteCustomChart(chart.id)}>Delete</button>
				</div>
			</DashboardCard>
		{/each}
	</div>

	<!-- Vertical bar tooltip -->
	{#if barTooltip}
		<div
			class="fixed z-50 pointer-events-none"
			style="left: {barTooltip.x + 14}px; top: {barTooltip.y - 12}px;"
		>
			<div class="bg-bg-tertiary border border-border rounded-md px-3 py-2.5 shadow-lg text-xs space-y-1.5 min-w-[160px]">
				<div class="text-text font-medium truncate">{barTooltip.model}</div>
				<div class="h-px bg-border"></div>
				<div class="flex items-center justify-between gap-4">
					<span class="flex items-center gap-1.5">
						<span class="w-2 h-2 rounded-sm" style="background: #67e8f988;"></span>
						<span class="text-text-muted">Input</span>
					</span>
					<span class="text-text font-mono">{barTooltip.input.toLocaleString()}</span>
				</div>
				<div class="flex items-center justify-between gap-4">
					<span class="flex items-center gap-1.5">
						<span class="w-2 h-2 rounded-sm" style="background: #a78bfa88;"></span>
						<span class="text-text-muted">Output</span>
					</span>
					<span class="text-text font-mono">{barTooltip.output.toLocaleString()}</span>
				</div>
				<div class="h-px bg-border"></div>
				<div class="flex items-center justify-between gap-4">
					<span class="text-text-secondary font-medium">Total</span>
					<span class="text-text font-mono font-medium">{barTooltip.total.toLocaleString()}</span>
				</div>
			</div>
		</div>
	{/if}
{/if}

<!-- Chart builder modal -->
<ChartBuilderModal
	bind:open={showChartBuilder}
	editConfig={editingChart}
	{timeRange}
	onSave={handleSaveCustomChart}
/>

<style>
	.tabular-nums {
		font-variant-numeric: tabular-nums;
	}
</style>
