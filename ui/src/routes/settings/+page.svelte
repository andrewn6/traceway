<script lang="ts">
	import { getStats, getHealth, clearAll, type Stats, type HealthStatus } from '$lib/api';
	import { onMount } from 'svelte';

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let health: HealthStatus | null = $state(null);
	let loading = $state(true);
	let showClearConfirm = $state(false);
	let clearing = $state(false);

	async function loadData() {
		try {
			const [s, h] = await Promise.all([
				getStats(),
				getHealth().catch(() => null)
			]);
			stats = s;
			health = h;
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

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<div class="max-w-3xl space-y-6">
	<h1 class="text-xl font-bold">Settings</h1>

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else}

		<!-- Daemon Status -->
		<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Daemon Status</h2>
			{#if health}
				<div class="grid grid-cols-2 gap-4 text-sm">
					<div>
						<span class="text-text-muted text-xs">Status</span>
						<div class="flex items-center gap-1.5 mt-0.5">
							<span class="w-2 h-2 rounded-full bg-success"></span>
							<span class="text-success">Running</span>
						</div>
					</div>
					{#if health.uptime_seconds !== undefined}
						<div>
							<span class="text-text-muted text-xs">Uptime</span>
							<div class="text-text mt-0.5">{formatUptime(health.uptime_seconds)}</div>
						</div>
					{/if}
					{#if health.version}
						<div>
							<span class="text-text-muted text-xs">Version</span>
							<div class="text-text mt-0.5 font-mono">{health.version}</div>
						</div>
					{/if}
				</div>
				{#if health.components}
					<div class="border-t border-border pt-3">
						<span class="text-text-muted text-xs uppercase">Components</span>
						<div class="grid grid-cols-2 gap-2 mt-2">
							{#each Object.entries(health.components) as [name, status]}
								<div class="flex items-center gap-2 text-sm">
									<span class="w-1.5 h-1.5 rounded-full {status === 'running' || status === 'mounted' || status === 'connected' ? 'bg-success' : 'bg-warning'}"></span>
									<span class="text-text-secondary">{name}</span>
									<span class="text-text-muted text-xs">{status}</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}
			{:else}
				<div class="flex items-center gap-1.5 text-sm">
					<span class="w-2 h-2 rounded-full bg-danger"></span>
					<span class="text-danger">Not connected</span>
				</div>
				<p class="text-text-muted text-xs">Start the daemon with <code class="bg-bg-tertiary px-1 rounded">llmtrace start</code></p>
			{/if}
		</section>

		<!-- Storage -->
		<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Storage</h2>
			<div class="grid grid-cols-3 gap-4 text-sm">
				<div>
					<span class="text-text-muted text-xs">Traces</span>
					<div class="text-text text-lg font-bold mt-0.5">{stats.trace_count}</div>
				</div>
				<div>
					<span class="text-text-muted text-xs">Spans</span>
					<div class="text-text text-lg font-bold mt-0.5">{stats.span_count}</div>
				</div>
				{#if health?.storage?.db_size_bytes}
					<div>
						<span class="text-text-muted text-xs">Database Size</span>
						<div class="text-text text-lg font-bold mt-0.5">{formatBytes(health.storage.db_size_bytes)}</div>
					</div>
				{/if}
			</div>
		</section>

		<!-- Configuration -->
		<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Configuration</h2>
			<p class="text-text-muted text-xs">
				Configuration is loaded from <code class="bg-bg-tertiary px-1 rounded">~/.llmtrace/config.toml</code>
			</p>
			<div class="grid grid-cols-2 gap-4 text-sm">
				<div>
					<span class="text-text-muted text-xs">API Address</span>
					<div class="text-text mt-0.5 font-mono text-xs">127.0.0.1:3000</div>
				</div>
				<div>
					<span class="text-text-muted text-xs">Proxy Address</span>
					<div class="text-text mt-0.5 font-mono text-xs">127.0.0.1:3001</div>
				</div>
				<div>
					<span class="text-text-muted text-xs">Database Path</span>
					<div class="text-text mt-0.5 font-mono text-xs">~/.llmtrace/traces.db</div>
				</div>
				<div>
					<span class="text-text-muted text-xs">Mount Path</span>
					<div class="text-text mt-0.5 font-mono text-xs">~/.llmtrace/mem</div>
				</div>
			</div>
		</section>

		<!-- Danger Zone -->
		<section class="border border-danger/30 rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-danger uppercase tracking-wide">Danger Zone</h2>
			{#if showClearConfirm}
				<div class="bg-danger/10 border border-danger/30 rounded p-3 space-y-2">
					<p class="text-sm text-text">This will permanently delete all traces, spans, and file history.</p>
					<div class="flex gap-2">
						<button
							class="bg-danger text-white px-3 py-1.5 rounded text-sm font-semibold hover:opacity-90 transition-opacity"
							onclick={handleClear}
							disabled={clearing}
						>
							{clearing ? 'Clearing...' : 'Yes, clear everything'}
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
		</section>
	{/if}
</div>
