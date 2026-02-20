<script lang="ts">
	import { getStats, getHealth, clearAll, getAuthConfig, type Stats, type HealthResponse, type AuthConfig } from '$lib/api';
	import { onMount } from 'svelte';

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let health: HealthResponse | null = $state(null);
	let authConfig: AuthConfig | null = $state<AuthConfig | null>(null);
	let loading = $state(true);

	// Danger zone
	let showClearConfirm = $state(false);
	let clearing = $state(false);

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

	const isCloudMode = $derived(authConfig !== null && authConfig.mode === 'cloud');
</script>

<div class="max-w-3xl space-y-6">
	<h1 class="text-xl font-bold">Settings</h1>

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else}

		<!-- Server Status -->
		<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Server Status</h2>
			{#if health}
				<div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
					<div>
						<span class="text-text-muted text-xs">Status</span>
						<div class="flex items-center gap-1.5 mt-0.5">
							<span class="w-2 h-2 rounded-full bg-success"></span>
							<span class="text-success">Connected</span>
						</div>
					</div>
					<div>
						<span class="text-text-muted text-xs">Mode</span>
						<div class="text-text mt-0.5">
							{#if isCloudMode}
								<span class="px-1.5 py-0.5 bg-accent/10 text-accent rounded text-xs">Cloud</span>
							{:else}
								<span class="px-1.5 py-0.5 bg-bg-tertiary text-text-secondary rounded text-xs">Local</span>
							{/if}
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
				</div>
				{#if health.region || health.instance}
					<div class="grid grid-cols-2 gap-4 text-sm border-t border-border pt-3 mt-3">
						{#if health.region}
							<div>
								<span class="text-text-muted text-xs">Region</span>
								<div class="text-text mt-0.5 font-mono text-xs">{health.region}</div>
							</div>
						{/if}
						{#if health.instance}
							<div>
								<span class="text-text-muted text-xs">Instance</span>
								<div class="text-text mt-0.5 font-mono text-xs">{health.instance}</div>
							</div>
						{/if}
					</div>
				{/if}
			{:else}
				<div class="flex items-center gap-1.5 text-sm">
					<span class="w-2 h-2 rounded-full bg-danger"></span>
					<span class="text-danger">Not connected</span>
				</div>
				<p class="text-text-muted text-xs">Make sure the server is running.</p>
			{/if}
		</section>

		<!-- Storage -->
		<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Storage</h2>
			<div class="grid grid-cols-2 gap-4 text-sm">
				<div>
					<span class="text-text-muted text-xs">Traces</span>
					<div class="text-text text-lg font-bold mt-0.5">{stats.trace_count.toLocaleString()}</div>
				</div>
				<div>
					<span class="text-text-muted text-xs">Spans</span>
					<div class="text-text text-lg font-bold mt-0.5">{stats.span_count.toLocaleString()}</div>
				</div>
			</div>
			{#if health?.storage}
				<div class="text-text-muted text-xs border-t border-border pt-2 mt-2">
					Backend: <span class="font-mono">{health.storage.backend}</span>
				</div>
			{/if}
		</section>

		<!-- Cloud Features (only show in cloud mode) -->
		{#if isCloudMode}
			<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
				<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Organization</h2>
				<p class="text-text-muted text-sm">
					Manage your organization, team members, and API keys from the 
					<a href="/org" class="text-accent hover:underline">Organization</a> page.
				</p>
			</section>
		{/if}

		<!-- Danger Zone -->
		<section class="border border-danger/30 rounded p-4 space-y-4">
			<h2 class="text-sm font-semibold text-danger uppercase tracking-wide">Danger Zone</h2>

			<!-- Clear all data -->
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
