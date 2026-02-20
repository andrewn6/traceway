<script lang="ts">
	import { getAnalyticsSummary, type AnalyticsSummary } from '$lib/api';
	import DashboardCard from '$lib/components/DashboardCard.svelte';
	import HorizontalBarList from '$lib/components/HorizontalBarList.svelte';
	import { onMount } from 'svelte';

	let summary: AnalyticsSummary | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	// Layout configuration stored in localStorage
	type CardId =
		| 'top_model_tokens'
		| 'top_spans'
		| 'top_llm_spans'
		| 'cost_by_model'
		| 'total_cost'
		| 'tokens_by_model'
		| 'overview_traces'
		| 'overview_spans'
		| 'overview_llm_calls'
		| 'overview_errors'
		| 'avg_latency'
		| 'providers';

	interface CardLayout {
		id: CardId;
		col: number; // grid column span
		row: number; // grid row span
		visible: boolean;
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

	// Time range state
	type TimeRange = '1h' | '6h' | '1d' | '7d' | '30d';
	let timeRange: TimeRange = $state('1d');
	let interval = $state('By hour');

	const timeRangeLabels: Record<TimeRange, string> = {
		'1h': '1h',
		'6h': '6h',
		'1d': '1d',
		'7d': '7d',
		'30d': '30d',
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

	const cardLabels: Record<CardId, string> = {
		top_model_tokens: 'Top model tokens',
		top_spans: 'Top spans',
		top_llm_spans: 'Top LLM spans',
		cost_by_model: 'Top model cost',
		total_cost: 'Total cost',
		tokens_by_model: 'Tokens by model (sum)',
		overview_traces: 'Total Traces',
		overview_spans: 'Total Spans',
		overview_llm_calls: 'LLM Calls',
		overview_errors: 'Errors',
		avg_latency: 'Avg Latency',
		providers: 'Providers',
	};

	const visibleCards = $derived(layout.filter((c) => c.visible));

	function formatLargeNumber(v: number): string {
		return v.toLocaleString();
	}

	function generateSparkline(base: number, count: number): number[] {
		return Array.from({ length: count }, (_, i) => {
			const variance = base * 0.3;
			const seed = Math.sin(i * 2.5 + 1) * 0.5 + 0.5;
			return base + (seed - 0.5) * variance * 2;
		});
	}

	function sparklinePath(points: number[], width: number, height: number): { line: string; area: string } {
		const maxP = Math.max(...points);
		const minP = Math.min(...points);
		const range = maxP - minP || 1;
		const stepX = width / (points.length - 1);
		const coords = points.map((v, i) => ({
			x: i * stepX,
			y: height - ((v - minP) / range) * (height * 0.8) - height * 0.1,
		}));
		const line = coords.map((c, i) => `${i === 0 ? 'M' : 'L'}${c.x.toFixed(1)},${c.y.toFixed(1)}`).join(' ');
		const area = `${line} L${coords[coords.length - 1].x.toFixed(1)},${height} L0,${height} Z`;
		return { line, area };
	}

	const latencyPoints = $derived.by(() => generateSparkline(summary ? summary.avg_latency_ms : 100, 24));
	const latencyPath = $derived.by(() => sparklinePath(latencyPoints, 96, 60));
	const costPoints = $derived.by(() => generateSparkline(summary ? summary.total_cost : 10, 24));
	const costPath = $derived.by(() => sparklinePath(costPoints, 96, 60));

	async function loadData() {
		try {
			summary = await getAnalyticsSummary();
			error = '';
		} catch (e) {
			error = 'Could not load analytics. Is the daemon running?';
		}
		loading = false;
	}

	onMount(() => {
		loadData();
		const interval = setInterval(loadData, 10000);
		return () => clearInterval(interval);
	});
</script>

<!-- Dashboard header bar -->
<div class="flex items-center gap-3 mb-5">
	<div class="flex items-center gap-2 bg-bg-secondary border border-border rounded px-3 py-1.5">
		<svg class="w-4 h-4 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6ZM3.75 15.75A2.25 2.25 0 0 1 6 13.5h2.25a2.25 2.25 0 0 1 2.25 2.25V18a2.25 2.25 0 0 1-2.25 2.25H6A2.25 2.25 0 0 1 3.75 18v-2.25ZM13.5 6a2.25 2.25 0 0 1 2.25-2.25H18A2.25 2.25 0 0 1 20.25 6v2.25A2.25 2.25 0 0 1 18 10.5h-2.25a2.25 2.25 0 0 1-2.25-2.25V6ZM13.5 15.75a2.25 2.25 0 0 1 2.25-2.25H18a2.25 2.25 0 0 1 2.25 2.25V18A2.25 2.25 0 0 1 18 20.25h-2.25A2.25 2.25 0 0 1 13.5 18v-2.25Z" />
		</svg>
		<span class="text-text text-sm font-medium">dashboard</span>
	</div>

	<!-- Time range buttons -->
	<div class="flex items-center gap-0.5 bg-bg-secondary border border-border rounded px-1 py-1">
		{#each Object.entries(timeRangeLabels) as [key, label]}
			<button
				class="px-2 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer
					{timeRange === key ? 'bg-bg-tertiary text-text' : 'text-text-muted hover:text-text'}"
				onclick={() => (timeRange = key as TimeRange)}
			>
				{label}
			</button>
		{/each}
	</div>

	<!-- Date display -->
	<div class="flex items-center gap-1.5 bg-bg-secondary border border-border rounded px-3 py-1.5 text-xs text-text-secondary">
		<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
			<path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 0 1 2.25-2.25h13.5A2.25 2.25 0 0 1 21 7.5v11.25m-18 0A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75m-18 0v-7.5A2.25 2.25 0 0 1 5.25 9h13.5A2.25 2.25 0 0 1 21 11.25v7.5" />
		</svg>
		{new Date().toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}, {new Date().toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })}
	</div>

	<!-- Interval selector -->
	<div class="flex items-center gap-1 bg-bg-secondary border border-border rounded px-3 py-1.5 text-xs text-text-secondary">
		{interval}
		<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
			<path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
		</svg>
	</div>

	<div class="flex-1"></div>

	<!-- Layout config button -->
	<button
		class="flex items-center gap-1.5 bg-bg-secondary border border-border rounded px-3 py-1.5 text-xs text-text-secondary hover:text-text transition-colors cursor-pointer"
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
	<div class="bg-bg-secondary border border-border rounded-lg p-4 mb-5 space-y-3">
		<div class="flex items-center justify-between">
			<span class="text-text text-sm font-medium">Configure Layout</span>
			<button
				class="text-xs text-text-muted hover:text-text transition-colors cursor-pointer"
				onclick={resetLayout}
			>
				Reset to default
			</button>
		</div>
		<div class="space-y-2">
			{#each layout as card, i}
				<div class="flex items-center gap-3 bg-bg-tertiary rounded px-3 py-2">
					<!-- Reorder buttons -->
					<div class="flex flex-col gap-0.5">
						<button
							class="text-text-muted hover:text-text transition-colors cursor-pointer disabled:opacity-30"
							onclick={() => moveCard(i, -1)}
							disabled={i === 0}
						>
							<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="m4.5 15.75 7.5-7.5 7.5 7.5" />
							</svg>
						</button>
						<button
							class="text-text-muted hover:text-text transition-colors cursor-pointer disabled:opacity-30"
							onclick={() => moveCard(i, 1)}
							disabled={i === layout.length - 1}
						>
							<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
							</svg>
						</button>
					</div>

					<!-- Toggle visibility -->
					<button
						class="w-4 h-4 rounded border cursor-pointer transition-colors {card.visible
							? 'bg-accent border-accent'
							: 'border-border'}"
						onclick={() => toggleCard(card.id)}
					>
						{#if card.visible}
							<svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
							</svg>
						{/if}
					</button>

					<!-- Card name -->
					<span class="text-text text-xs flex-1 {!card.visible ? 'opacity-40' : ''}">{cardLabels[card.id]}</span>

					<!-- Column size selector -->
					<div class="flex items-center gap-1">
						{#each [2, 3, 6] as size}
							<button
								class="px-2 py-0.5 rounded text-xs cursor-pointer transition-colors
									{card.col === size ? 'bg-accent text-white' : 'bg-bg-secondary text-text-muted hover:text-text'}"
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
	<div class="text-text-muted text-sm py-8 text-center">Loading analytics...</div>
{:else if error}
	<div class="bg-bg-secondary border border-border rounded p-6 text-center">
		<p class="text-text-muted text-sm">{error}</p>
	</div>
{:else if !summary || summary.total_spans === 0}
	<div class="bg-bg-secondary border border-border rounded p-6 text-center">
		<p class="text-text-muted text-sm">No data yet. Create some traces to see analytics.</p>
	</div>
{:else}
	<!-- Dashboard grid -->
	<div class="grid grid-cols-6 gap-4 auto-rows-auto">
		{#each visibleCards as card (card.id)}
			{#if card.id === 'top_model_tokens'}
				<DashboardCard title="Top model tokens" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text">{formatLargeNumber(summary.total_tokens)}</span>
					</div>
					<HorizontalBarList
						items={summary.tokens_by_model.map((t) => ({
							label: t.model,
							value: t.total_tokens,
							color: '#4673d1',
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'top_spans'}
				<DashboardCard title="Top spans" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text">{formatLargeNumber(summary.total_spans)}</span>
					</div>
					<HorizontalBarList
						items={summary.cost_by_model.map((c) => ({
							label: c.model,
							value: c.span_count,
							color: '#4673d1',
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'top_llm_spans'}
				<DashboardCard title="Top LLM spans" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text">{formatLargeNumber(summary.total_llm_calls)}</span>
					</div>
					<HorizontalBarList
						items={summary.tokens_by_model.map((t) => ({
							label: t.model,
							value: t.total_tokens,
							color: '#4673d1',
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'avg_latency'}
				<DashboardCard title="Trace duration (p99)" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text">{Math.round(summary.avg_latency_ms)}ms</span>
					</div>
					<!-- Latency sparkline -->
					<div class="h-32 flex items-end">
						<svg class="w-full h-full" viewBox="0 0 96 60" preserveAspectRatio="none">
							<defs>
								<linearGradient id="latency-gradient" x1="0" y1="0" x2="0" y2="1">
									<stop offset="0%" stop-color="#58a6ff" stop-opacity="0.3" />
									<stop offset="100%" stop-color="#58a6ff" stop-opacity="0.05" />
								</linearGradient>
							</defs>
							<path d={latencyPath.area} fill="url(#latency-gradient)" />
							<path d={latencyPath.line} fill="none" stroke="#58a6ff" stroke-width="1.5" vector-effect="non-scaling-stroke" />
						</svg>
					</div>
				</DashboardCard>
			{:else if card.id === 'tokens_by_model'}
				<DashboardCard title="Tokens by model (sum)" colSpan={card.col} rowSpan={card.row}>
					<div class="h-40 flex items-end justify-center gap-2 pt-4">
						{#each summary.tokens_by_model as item}
							{@const maxTokens = Math.max(...summary.tokens_by_model.map((t) => t.total_tokens))}
							{@const height = maxTokens > 0 ? (item.total_tokens / maxTokens) * 100 : 0}
							<div class="flex flex-col items-center gap-1 flex-1 max-w-16">
								<div class="w-full flex flex-col items-center" style="height: 120px;">
									<div class="w-full mt-auto flex gap-0.5">
										<div
											class="flex-1 bg-blue-400/40 rounded-t"
											style="height: {(item.input_tokens / maxTokens) * 120}px"
											title="Input: {item.input_tokens.toLocaleString()}"
										></div>
										<div
											class="flex-1 bg-purple-400/40 rounded-t"
											style="height: {(item.output_tokens / maxTokens) * 120}px"
											title="Output: {item.output_tokens.toLocaleString()}"
										></div>
									</div>
								</div>
								<span class="text-text-muted text-[9px] truncate w-full text-center">{item.model.split('/').pop()}</span>
							</div>
						{/each}
					</div>
				</DashboardCard>
			{:else if card.id === 'cost_by_model'}
				<DashboardCard title="Top model cost" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text">{summary.total_cost.toFixed(2)}</span>
					</div>
					<HorizontalBarList
						items={summary.cost_by_model.map((c) => ({
							label: c.model,
							value: c.cost,
							color: '#4673d1',
						}))}
					/>
				</DashboardCard>
			{:else if card.id === 'total_cost'}
				<DashboardCard title="Total cost" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-1 mb-3">
						<span class="text-2xl font-bold text-text">{summary.total_cost.toFixed(2)}</span>
					</div>
					<!-- Mini cost sparkline -->
					<div class="h-24 flex items-end">
						<svg class="w-full h-full" viewBox="0 0 96 60" preserveAspectRatio="none">
							<defs>
								<linearGradient id="cost-gradient" x1="0" y1="0" x2="0" y2="1">
									<stop offset="0%" stop-color="#58a6ff" stop-opacity="0.2" />
									<stop offset="100%" stop-color="#58a6ff" stop-opacity="0.02" />
								</linearGradient>
							</defs>
							<path d={costPath.area} fill="url(#cost-gradient)" />
							<path d={costPath.line} fill="none" stroke="#58a6ff" stroke-width="1.5" vector-effect="non-scaling-stroke" />
						</svg>
					</div>
				</DashboardCard>
			{:else if card.id === 'overview_traces'}
				<DashboardCard title="Total Traces" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-2">
						<span class="text-3xl font-bold text-text">{formatLargeNumber(summary.total_traces)}</span>
					</div>
				</DashboardCard>
			{:else if card.id === 'overview_spans'}
				<DashboardCard title="Total Spans" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-2">
						<span class="text-3xl font-bold text-text">{formatLargeNumber(summary.total_spans)}</span>
					</div>
				</DashboardCard>
			{:else if card.id === 'overview_llm_calls'}
				<DashboardCard title="LLM Calls" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-2">
						<span class="text-3xl font-bold text-text">{formatLargeNumber(summary.total_llm_calls)}</span>
					</div>
				</DashboardCard>
			{:else if card.id === 'overview_errors'}
				<DashboardCard title="Errors" colSpan={card.col} rowSpan={card.row}>
					<div class="mt-2">
						<span class="text-3xl font-bold {summary.error_count > 0 ? 'text-danger' : 'text-success'}">
							{formatLargeNumber(summary.error_count)}
						</span>
					</div>
				</DashboardCard>
			{:else if card.id === 'providers'}
				<DashboardCard title="Providers" colSpan={card.col} rowSpan={card.row}>
					<div class="flex flex-wrap gap-2 mt-2">
						{#each summary.providers_used as provider}
							<span class="bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text">
								{provider}
							</span>
						{/each}
					</div>
				</DashboardCard>
			{/if}
		{/each}
	</div>
{/if}
