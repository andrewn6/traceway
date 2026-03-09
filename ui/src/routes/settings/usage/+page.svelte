<script lang="ts">
	import { getOrg, getOrgMembers, getStats, queryAnalytics, type OrgInfo, type Stats } from '$lib/api';
	import SparklineChart from '$lib/components/SparklineChart.svelte';
	import { onMount } from 'svelte';

	let org: OrgInfo | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let memberCount = $state(0);
	let monthlySpans = $state(0);

	type UsageRange = '24h' | '7d' | '30d';
	type UsageGroup = 'hour' | 'day';

	let usageRange: UsageRange = $state('24h');
	let usageLoading = $state(false);
	let usageError = $state('');
	let usageSpansTotal = $state(0);
	let usageTokensTotal = $state(0);
	let usageSpanSeries: number[] = $state([]);
	let usageTokenSeries: number[] = $state([]);
	let usageSeriesLabels: string[] = $state([]);

	function formatNumber(n: number): string {
		if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(0)}B`;
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
		return n.toString();
	}

	function formatPercent(value: number): string {
		if (value < 0.1) return '<0.1%';
		if (value < 1) return `${value.toFixed(1)}%`;
		return `${Math.round(value)}%`;
	}

	function formatSeriesLabel(iso: string, groupBy: UsageGroup): string {
		const d = new Date(iso);
		if (groupBy === 'hour') {
			return d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
		}
		return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
	}

	function getMonthStart(): string {
		const now = new Date();
		return new Date(now.getFullYear(), now.getMonth(), 1).toISOString();
	}

	function getUsageWindow(range: UsageRange): { since: string; until: string; groupBy: UsageGroup } {
		const now = new Date();
		const until = now.toISOString();
		if (range === '24h') {
			return { since: new Date(now.getTime() - 24 * 60 * 60 * 1000).toISOString(), until, groupBy: 'hour' };
		}
		if (range === '7d') {
			return { since: new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000).toISOString(), until, groupBy: 'day' };
		}
		return { since: new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000).toISOString(), until, groupBy: 'day' };
	}

	function ensureSeriesValues<T>(values: T[], fallback: T): T[] {
		if (values.length >= 2) return values;
		if (values.length === 1) return [values[0], values[0]];
		return [fallback, fallback];
	}

	async function loadUsage() {
		usageLoading = true;
		usageError = '';

		try {
			const { since, until, groupBy } = getUsageWindow(usageRange);
			const [rangeSeries, rangeTotals] = await Promise.all([
				queryAnalytics({ metrics: ['span_count', 'total_tokens'], filter: { since, until }, group_by: [groupBy] }),
				queryAnalytics({ metrics: ['span_count', 'total_tokens'], filter: { since, until } })
			]);

			usageSpansTotal = rangeTotals.totals.span_count ?? 0;
			usageTokensTotal = rangeTotals.totals.total_tokens ?? 0;

			const sorted = [...rangeSeries.groups].sort((a, b) => {
				const aKey = a.key[groupBy] ?? '';
				const bKey = b.key[groupBy] ?? '';
				return aKey.localeCompare(bKey);
			});

			usageSpanSeries = ensureSeriesValues(sorted.map((g) => g.metrics.span_count ?? 0), 0);
			usageTokenSeries = ensureSeriesValues(sorted.map((g) => g.metrics.total_tokens ?? 0), 0);
			usageSeriesLabels = ensureSeriesValues(sorted.map((g) => formatSeriesLabel(g.key[groupBy] ?? '', groupBy)), '-');
		} catch {
			usageError = 'Could not load usage breakdown right now.';
			usageSpanSeries = [0, 0];
			usageTokenSeries = [0, 0];
			usageSeriesLabels = ['-', '-'];
		}

		usageLoading = false;
	}

	const spanQuotaPct = $derived.by(() => {
		const limit = org?.plan_limits.spans_per_month ?? 0;
		if (limit <= 0) return 0;
		return Math.min(100, (monthlySpans / limit) * 100);
	});

	const teamQuotaPct = $derived.by(() => {
		const limit = org?.plan_limits.max_team_members ?? 0;
		if (limit <= 0) return 0;
		return Math.min(100, (memberCount / limit) * 100);
	});

	const spansLeftInQuota = $derived.by(() => {
		const limit = org?.plan_limits.spans_per_month ?? 0;
		return Math.max(0, limit - monthlySpans);
	});

	const teamSeatsLeft = $derived.by(() => {
		const limit = org?.plan_limits.max_team_members ?? 0;
		return Math.max(0, limit - memberCount);
	});

	onMount(async () => {
		try {
			const [o, members, s, monthAnalytics] = await Promise.all([
				getOrg(),
				getOrgMembers().catch(() => []),
				getStats().catch(() => ({ trace_count: 0, span_count: 0 })),
				queryAnalytics({ metrics: ['span_count'], filter: { since: getMonthStart() } }).catch(() => null)
			]);

			org = o;
			memberCount = members.length;
			stats = s;
			monthlySpans = monthAnalytics?.totals?.span_count ?? 0;
			await loadUsage();
		} catch {
			error = 'Failed to load usage.';
		}
		loading = false;
	});

	$effect(() => {
		if (!org) return;
		usageRange;
		loadUsage();
	});
</script>

<div class="w-full space-y-3.5">
	<div>
		<h1 class="text-xl font-semibold tracking-tight">Usage</h1>
		<p class="text-text-muted text-[13px] mt-0.5">Quota and activity for your current workspace.</p>
	</div>

	{#if error}
		<div class="alert-danger">{error}</div>
	{/if}

	{#if loading || !org}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else}
		<section class="space-y-2.5">
			<div class="flex items-center gap-1.5 px-0.5">
				<svg class="w-3.5 h-3.5 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8">
					<path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m-9.75 7.5h10.5a2.25 2.25 0 0 0 2.25-2.25V6.75A2.25 2.25 0 0 0 17.25 4.5H6.75A2.25 2.25 0 0 0 4.5 6.75v10.5A2.25 2.25 0 0 0 6.75 19.5Z" />
				</svg>
				<h2 class="text-[13.5px] font-semibold text-text">Quota</h2>
			</div>
			<p class="text-[12.5px] text-text-muted px-0.5">How much of your monthly allowance has been consumed.</p>
			<div class="table-float p-0 overflow-hidden">
				<div class="grid grid-cols-1 md:grid-cols-2 divide-y md:divide-y-0 md:divide-x divide-border/55">
					<div class="p-4 space-y-2">
						<div class="flex items-center justify-between">
							<div class="text-[13px] font-medium text-text-secondary">Spans</div>
							<div class="text-[12px] text-text-muted">Monthly</div>
						</div>
						<div class="text-[1.7rem] leading-none text-text">
							<span class="font-semibold">{formatNumber(monthlySpans)}</span>
							<span class="text-[13px] text-text-muted"> / {formatNumber(org.plan_limits.spans_per_month)}</span>
						</div>
						<div class="text-[12px] text-text-muted">{formatPercent(spanQuotaPct)} used</div>
						<div class="h-2 rounded-full bg-bg-tertiary overflow-hidden">
							<div class="h-full rounded-full bg-accent transition-all duration-400" style="width: {spanQuotaPct}%"></div>
						</div>
						<div class="text-[12px] text-text-secondary text-right">{formatNumber(spansLeftInQuota)} spans left this month</div>
					</div>

					<div class="p-4 space-y-2">
						<div class="flex items-center justify-between">
							<div class="text-[13px] font-medium text-text-secondary">Team seats</div>
							<div class="text-[12px] text-text-muted">Organization</div>
						</div>
						<div class="text-[1.7rem] leading-none text-text">
							<span class="font-semibold">{memberCount}</span>
							<span class="text-[13px] text-text-muted"> / {org.plan_limits.max_team_members}</span>
						</div>
						<div class="text-[12px] text-text-muted">{formatPercent(teamQuotaPct)} occupied</div>
						<div class="h-2 rounded-full bg-bg-tertiary overflow-hidden">
							<div class="h-full rounded-full bg-accent transition-all duration-400" style="width: {teamQuotaPct}%"></div>
						</div>
						<div class="text-[12px] text-text-secondary text-right">{teamSeatsLeft} seats available</div>
					</div>
				</div>
			</div>
		</section>

		<section class="space-y-2.5">
			<div class="flex items-center justify-between gap-3 px-0.5">
				<div>
					<h2 class="text-[13.5px] font-semibold text-text">Usage</h2>
					<p class="text-[12.5px] text-text-muted">Actual ingestion and token load for your selected period.</p>
				</div>
				<div class="flex items-center gap-0.5 rounded-lg border border-border/60 bg-bg-tertiary/35 p-0.5">
					{#each ['24h', '7d', '30d'] as range}
						<button
							onclick={() => (usageRange = range as UsageRange)}
							class="px-2.5 py-0.5 rounded-md text-[11.5px] transition-colors {usageRange === range ? 'bg-bg-tertiary text-text border border-border/65' : 'text-text-muted hover:text-text'}"
						>
							{range}
						</button>
					{/each}
				</div>
			</div>

			{#if usageError}
				<div class="alert-warning text-[12px]">{usageError}</div>
			{/if}

			<div class="table-float p-0 overflow-hidden">
				<div class="grid grid-cols-1 xl:grid-cols-2 divide-y xl:divide-y-0 xl:divide-x divide-border/55">
					<div class="p-4 space-y-2">
						<div class="flex items-baseline gap-2">
							<span class="text-[13px] text-text-secondary">Spans in {usageRange}</span>
							{#if usageLoading}
								<span class="text-[10px] text-text-muted">Updating...</span>
							{/if}
						</div>
						<div class="text-[1.8rem] leading-none font-semibold text-text">{formatNumber(usageSpansTotal)}</div>
						<SparklineChart points={usageSpanSeries} labels={usageSeriesLabels} color="#2f9c88" height={112} />
					</div>

					<div class="p-4 space-y-2">
						<div class="flex items-baseline gap-2">
							<span class="text-[13px] text-text-secondary">Tokens in {usageRange}</span>
							{#if usageLoading}
								<span class="text-[10px] text-text-muted">Updating...</span>
							{/if}
						</div>
						<div class="text-[1.8rem] leading-none font-semibold text-text">{formatNumber(usageTokensTotal)}</div>
						<SparklineChart points={usageTokenSeries} labels={usageSeriesLabels} color="#67e8f9" height={112} />
					</div>
				</div>
			</div>

			<div class="grid grid-cols-2 lg:grid-cols-4 gap-2 text-[12px] text-text-muted px-0.5">
				<div class="rounded-lg border border-border/60 bg-bg-secondary/30 px-2.5 py-2">
					<div>Total traces</div>
					<div class="text-sm text-text mt-0.5">{formatNumber(stats.trace_count)}</div>
				</div>
				<div class="rounded-lg border border-border/60 bg-bg-secondary/30 px-2.5 py-2">
					<div>Total spans</div>
					<div class="text-sm text-text mt-0.5">{formatNumber(stats.span_count)}</div>
				</div>
				<div class="rounded-lg border border-border/60 bg-bg-secondary/30 px-2.5 py-2">
					<div>Retention</div>
					<div class="text-sm text-text mt-0.5">{org.plan_limits.retention_days} days</div>
				</div>
				<div class="rounded-lg border border-border/60 bg-bg-secondary/30 px-2.5 py-2">
					<div>Team limit</div>
					<div class="text-sm text-text mt-0.5">{org.plan_limits.max_team_members} seats</div>
				</div>
			</div>
		</section>
	{/if}
</div>
