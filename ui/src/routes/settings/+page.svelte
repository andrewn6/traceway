<script lang="ts">
	import { getStats, getHealth, clearAll, getAuthConfig, getAuthMe, getOrg, getOrgMembers, queryAnalytics, type Stats, type HealthResponse, type AuthConfig, type AuthMe, type OrgInfo, type OrgMember } from '$lib/api';
	import { onMount } from 'svelte';

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let health: HealthResponse | null = $state(null);
	let authConfig: AuthConfig | null = $state(null);
	let authMe: AuthMe | null = $state(null);
	let org: OrgInfo | null = $state(null);
	let members: OrgMember[] = $state([]);
	let monthlySpans = $state(0);
	let loading = $state(true);

	// Danger zone
	let showClearConfirm = $state(false);
	let clearing = $state(false);

	const isCloudMode = $derived.by((): boolean => authConfig?.mode === 'cloud');

	// Find current user from members list
	const currentUser = $derived.by(() => {
		if (!authMe?.user_id || members.length === 0) return null;
		return members.find(m => m.id === authMe!.user_id) ?? null;
	});

	// Usage calculations — monthly spans vs plan limit
	const spanLimit = $derived.by((): number => org?.plan_limits?.spans_per_month ?? 0);
	const spanUsage = $derived(isCloudMode ? monthlySpans : stats.span_count);
	const usagePct = $derived(spanLimit > 0 ? Math.min(100, (spanUsage / spanLimit) * 100) : 0);
	const usageColor = $derived(usagePct > 90 ? 'bg-danger' : usagePct > 70 ? 'bg-warning' : 'bg-accent');

	function formatNumber(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
		return n.toLocaleString();
	}

	function getMonthStart(): string {
		const now = new Date();
		return new Date(now.getFullYear(), now.getMonth(), 1).toISOString();
	}

	async function loadData() {
		try {
			const [s, h, auth] = await Promise.all([
				getStats(),
				getHealth().catch(() => null),
				getAuthConfig().catch(() => null)
			]);
			stats = s;
			health = h;
			authConfig = auth;

			if (auth?.mode === 'cloud') {
				const [me, o, m, analytics] = await Promise.all([
					getAuthMe().catch(() => null),
					getOrg().catch(() => null),
					getOrgMembers().catch(() => []),
					queryAnalytics({
						metrics: ['span_count'],
						filter: { since: getMonthStart() }
					}).catch(() => null)
				]);
				authMe = me;
				org = o;
				members = m;
				if (analytics?.totals?.span_count != null) {
					monthlySpans = analytics.totals.span_count;
				}
			}
		} catch {
			// daemon not running
		}
		loading = false;
	}

	async function handleClear() {
		clearing = true;
		try {
			await clearAll();
			stats = { trace_count: 0, span_count: 0 };
			showClearConfirm = false;
		} catch {
			// failed
		}
		clearing = false;
	}

	onMount(loadData);

	function formatUptime(seconds: number): string {
		if (seconds < 60) return `${seconds}s`;
		const mins = Math.floor(seconds / 60);
		if (mins < 60) return `${mins}m`;
		const hours = Math.floor(mins / 60);
		const remainMins = mins % 60;
		if (hours < 24) return `${hours}h ${remainMins}m`;
		const days = Math.floor(hours / 24);
		return `${days}d ${hours % 24}h`;
	}

	function planLabel(plan: string): string {
		return plan.charAt(0).toUpperCase() + plan.slice(1);
	}
</script>

<div class="w-full space-y-5">
	<h1 class="text-xl font-semibold tracking-tight">Settings</h1>

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else}

		<!-- Account (cloud mode) -->
		{#if isCloudMode && (currentUser || org)}
			<section class="bg-bg-secondary border border-border rounded-md p-5 space-y-3">
				<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Account</h2>
				<div class="grid grid-cols-2 md:grid-cols-3 gap-4 text-sm">
					{#if currentUser}
						<div>
							<span class="text-text-muted text-xs">Email</span>
							<div class="text-text mt-0.5 text-sm truncate">{currentUser.email}</div>
						</div>
						{#if currentUser.name}
							<div>
								<span class="text-text-muted text-xs">Name</span>
								<div class="text-text mt-0.5 text-sm">{currentUser.name}</div>
							</div>
						{/if}
						<div>
							<span class="text-text-muted text-xs">Role</span>
							<div class="text-text mt-0.5 text-sm capitalize">{currentUser.role}</div>
						</div>
					{/if}
					{#if org}
						<div>
							<span class="text-text-muted text-xs">Organization</span>
							<div class="text-text mt-0.5 text-sm">{org.name}</div>
						</div>
						<div>
							<span class="text-text-muted text-xs">Plan</span>
							<div class="mt-0.5">
								<span class="px-1.5 py-0.5 bg-accent/10 text-accent rounded text-xs font-medium">{planLabel(org.plan)}</span>
							</div>
						</div>
					{/if}
				</div>
			</section>
		{/if}

		<!-- Usage -->
		<section class="bg-bg-secondary border border-border rounded-md p-5 space-y-4">
			<div class="flex items-center justify-between">
				<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Usage</h2>
				{#if isCloudMode}
					<span class="text-[11px] text-text-muted">This month</span>
				{/if}
			</div>

			<!-- Spans usage bar -->
			<div class="space-y-2">
				<div class="flex items-center justify-between text-sm">
					<span class="text-text-secondary">Spans{#if isCloudMode} this month{/if}</span>
					<span class="text-text-muted text-xs font-mono">
						{formatNumber(spanUsage)}{#if spanLimit > 0} / {formatNumber(spanLimit)}{/if}
					</span>
				</div>
				{#if spanLimit > 0}
					<div class="w-full bg-bg-tertiary rounded-full h-2.5 overflow-hidden">
						<div
							class="h-full rounded-full transition-all duration-500 {usageColor}"
							style="width: {usagePct}%"
						></div>
					</div>
					{#if usagePct > 90}
						<p class="text-[11px] text-danger">Approaching span limit. <a href="/settings/billing" class="underline">Upgrade plan</a></p>
					{/if}
				{:else}
					<div class="w-full bg-bg-tertiary rounded-full h-2.5 overflow-hidden">
						<div class="h-full rounded-full bg-accent/40" style="width: {spanUsage > 0 ? '5' : '0'}%"></div>
					</div>
				{/if}
			</div>

			<!-- Traces & other stats -->
			<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm pt-2 border-t border-border/60">
				<div>
					<span class="text-text-muted text-xs">Traces</span>
					<div class="text-text font-bold mt-0.5">{stats.trace_count.toLocaleString()}</div>
				</div>
				<div>
					<span class="text-text-muted text-xs">Spans</span>
					<div class="text-text font-bold mt-0.5">{stats.span_count.toLocaleString()}</div>
				</div>
				{#if org?.plan_limits}
					<div>
						<span class="text-text-muted text-xs">Retention</span>
						<div class="text-text font-bold mt-0.5">{org.plan_limits.retention_days}d</div>
					</div>
					<div>
						<span class="text-text-muted text-xs">Team limit</span>
						<div class="text-text font-bold mt-0.5">{org.plan_limits.max_team_members}</div>
					</div>
				{/if}
			</div>
		</section>

		<!-- Server -->
		<section class="bg-bg-secondary border border-border rounded-md p-5 space-y-3">
			<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Server</h2>
			{#if health}
				<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
					<div>
						<span class="text-text-muted text-xs">Status</span>
						<div class="flex items-center gap-1.5 mt-0.5">
							<span class="w-2 h-2 rounded-full bg-success"></span>
							<span class="text-success text-sm">Connected</span>
						</div>
					</div>
					<div>
						<span class="text-text-muted text-xs">Uptime</span>
						<div class="text-text mt-0.5">{formatUptime(health.uptime_secs)}</div>
					</div>
					<div>
						<span class="text-text-muted text-xs">Version</span>
						<div class="text-text mt-0.5 font-mono text-xs">{health.version}</div>
					</div>
					{#if health.region}
						<div>
							<span class="text-text-muted text-xs">Region</span>
							<div class="text-text mt-0.5 font-mono text-xs">{health.region}</div>
						</div>
					{/if}
				</div>
			{:else}
				<div class="flex items-center gap-1.5 text-sm">
					<span class="w-2 h-2 rounded-full bg-danger"></span>
					<span class="text-danger">Not connected</span>
				</div>
			{/if}
		</section>

		<!-- Danger Zone -->
		<section class="bg-bg-secondary border border-danger/30 rounded-md p-5 space-y-4">
			<h2 class="text-sm font-semibold text-danger uppercase tracking-wide">Danger Zone</h2>
			<div class="space-y-2">
				<p class="text-text-secondary text-sm">Clear all traces, spans, and file history.</p>
				{#if showClearConfirm}
					<div class="bg-danger/10 border border-danger/30 rounded p-3 space-y-2">
						<p class="text-sm text-text">This action cannot be undone. All data will be permanently deleted.</p>
						<div class="flex gap-2">
							<button
								class="bg-danger text-white px-3 py-1.5 rounded text-sm font-semibold hover:opacity-90 transition-opacity"
								onclick={handleClear}
								disabled={clearing}
							>
								{clearing ? 'Clearing...' : 'Yes, delete everything'}
							</button>
							<button
								class="bg-bg-tertiary text-text px-3 py-1.5 rounded text-sm hover:bg-bg-secondary transition-colors"
								onclick={() => showClearConfirm = false}
							>
								Cancel
							</button>
						</div>
					</div>
				{:else}
					<button
						class="border border-danger/50 text-danger px-3 py-1.5 rounded text-sm hover:bg-danger/10 transition-colors"
						onclick={() => showClearConfirm = true}
					>
						Clear all data
					</button>
				{/if}
			</div>
		</section>
	{/if}
</div>
