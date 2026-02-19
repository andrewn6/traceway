<script lang="ts">
	import { getStats, getHealth, getConfig, updateConfig, shutdownDaemon, clearAll, type Stats, type HealthStatus, type DaemonConfig } from '$lib/api';
	import { onMount } from 'svelte';

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let health: HealthStatus | null = $state(null);
	let config: DaemonConfig | null = $state(null);
	let loading = $state(true);

	// Config editor state
	let configLoading = $state(false);
	let configSaving = $state(false);
	let configSaved = $state(false);
	let configError = $state('');

	// Editable config fields
	let editApiAddr = $state('');
	let editProxyAddr = $state('');
	let editProxyTarget = $state('');
	let editCaptureMode = $state('');
	let editDbPath = $state('');
	let editLogLevel = $state('');

	// Danger zone
	let showClearConfirm = $state(false);
	let clearing = $state(false);
	let showShutdownConfirm = $state(false);
	let shuttingDown = $state(false);

	function populateForm(c: DaemonConfig) {
		editApiAddr = c.api.addr;
		editProxyAddr = c.proxy.addr;
		editProxyTarget = c.proxy.target;
		editCaptureMode = c.proxy.capture_mode;
		editDbPath = c.storage.db_path ?? '';
		editLogLevel = c.logging.level;
	}

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

		// Load config
		configLoading = true;
		try {
			config = await getConfig();
			populateForm(config);
		} catch {
			configError = 'Could not load config. Is the daemon running?';
		}
		configLoading = false;
		loading = false;
	}

	async function handleSaveConfig() {
		configSaving = true;
		configSaved = false;
		configError = '';
		try {
			const updated: DaemonConfig = {
				api: { addr: editApiAddr },
				proxy: { addr: editProxyAddr, target: editProxyTarget, capture_mode: editCaptureMode },
				storage: { db_path: editDbPath.trim() || null },
				logging: { level: editLogLevel }
			};
			config = await updateConfig(updated);
			populateForm(config);
			configSaved = true;
			setTimeout(() => configSaved = false, 3000);
		} catch (e) {
			configError = 'Failed to save config';
		}
		configSaving = false;
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

	async function handleShutdown() {
		shuttingDown = true;
		try {
			await shutdownDaemon();
			health = null;
			showShutdownConfirm = false;
		} catch {
			// may fail because connection drops — that's expected
			health = null;
			showShutdownConfirm = false;
		}
		shuttingDown = false;
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
					<div>
						<span class="text-text-muted text-xs">Uptime</span>
						<div class="text-text mt-0.5">{formatUptime(health.uptime_secs)}</div>
					</div>
					<div>
						<span class="text-text-muted text-xs">Version</span>
						<div class="text-text mt-0.5 font-mono">{health.version}</div>
					</div>
				</div>
			{:else}
				<div class="flex items-center gap-1.5 text-sm">
					<span class="w-2 h-2 rounded-full bg-danger"></span>
					<span class="text-danger">Not connected</span>
				</div>
				<p class="text-text-muted text-xs">Start the daemon with <code class="bg-bg-tertiary px-1 rounded">cargo run -p daemon -- --foreground</code></p>
			{/if}
		</section>

		<!-- Storage -->
		<section class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Storage</h2>
			<div class="grid grid-cols-2 gap-4 text-sm">
				<div>
					<span class="text-text-muted text-xs">Traces</span>
					<div class="text-text text-lg font-bold mt-0.5">{stats.trace_count}</div>
				</div>
				<div>
					<span class="text-text-muted text-xs">Spans</span>
					<div class="text-text text-lg font-bold mt-0.5">{stats.span_count}</div>
				</div>
			</div>
		</section>

		<!-- Configuration -->
		<section class="bg-bg-secondary border border-border rounded p-4 space-y-4">
			<div class="flex items-center justify-between">
				<h2 class="text-sm font-semibold text-text uppercase tracking-wide">Configuration</h2>
				{#if configSaved}
					<span class="text-success text-xs">Saved! Restart daemon to apply changes.</span>
				{/if}
			</div>

			{#if configLoading}
				<div class="text-text-muted text-xs">Loading config...</div>
			{:else if !config}
				<div class="text-text-muted text-xs">{configError || 'Config not available'}</div>
			{:else}
				<form onsubmit={(e) => { e.preventDefault(); handleSaveConfig(); }} class="space-y-4">
					<!-- API -->
					<fieldset class="space-y-2">
						<legend class="text-xs text-text-muted uppercase tracking-wide">API Server</legend>
						<div>
							<label for="cfg-api-addr" class="block text-xs text-text-secondary mb-0.5">Listen Address</label>
							<input
								id="cfg-api-addr"
								type="text"
								bind:value={editApiAddr}
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text font-mono"
							/>
						</div>
					</fieldset>

					<!-- Proxy -->
					<fieldset class="space-y-2">
						<legend class="text-xs text-text-muted uppercase tracking-wide">Proxy</legend>
						<div class="grid grid-cols-2 gap-3">
							<div>
								<label for="cfg-proxy-addr" class="block text-xs text-text-secondary mb-0.5">Listen Address</label>
								<input
									id="cfg-proxy-addr"
									type="text"
									bind:value={editProxyAddr}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text font-mono"
								/>
							</div>
							<div>
								<label for="cfg-proxy-target" class="block text-xs text-text-secondary mb-0.5">Target URL</label>
								<input
									id="cfg-proxy-target"
									type="text"
									bind:value={editProxyTarget}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text font-mono"
								/>
							</div>
						</div>
						<div>
							<label for="cfg-capture-mode" class="block text-xs text-text-secondary mb-0.5">Capture Mode</label>
							<select
								id="cfg-capture-mode"
								bind:value={editCaptureMode}
								class="bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text"
							>
								<option value="full">Full</option>
								<option value="headers">Headers only</option>
								<option value="none">None</option>
							</select>
						</div>
					</fieldset>

					<!-- Storage -->
					<fieldset class="space-y-2">
						<legend class="text-xs text-text-muted uppercase tracking-wide">Storage</legend>
						<div>
							<label for="cfg-db-path" class="block text-xs text-text-secondary mb-0.5">Database Path <span class="text-text-muted">(blank = default ~/.llmtrace/traces.db)</span></label>
							<input
								id="cfg-db-path"
								type="text"
								bind:value={editDbPath}
								placeholder="~/.llmtrace/traces.db"
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text font-mono placeholder:text-text-muted"
							/>
						</div>
					</fieldset>

					<!-- Logging -->
					<fieldset class="space-y-2">
						<legend class="text-xs text-text-muted uppercase tracking-wide">Logging</legend>
						<div>
							<label for="cfg-log-level" class="block text-xs text-text-secondary mb-0.5">Log Level</label>
							<select
								id="cfg-log-level"
								bind:value={editLogLevel}
								class="bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text"
							>
								<option value="trace">Trace</option>
								<option value="debug">Debug</option>
								<option value="info">Info</option>
								<option value="warn">Warn</option>
								<option value="error">Error</option>
							</select>
						</div>
					</fieldset>

					{#if configError}
						<div class="text-danger text-xs">{configError}</div>
					{/if}

					<div class="flex items-center gap-3">
						<button
							type="submit"
							disabled={configSaving}
							class="px-4 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50"
						>
							{configSaving ? 'Saving...' : 'Save Configuration'}
						</button>
						<span class="text-text-muted text-xs">Changes are saved to disk. Restart the daemon to apply.</span>
					</div>
				</form>
			{/if}
		</section>

		<!-- Danger Zone -->
		<section class="border border-danger/30 rounded p-4 space-y-4">
			<h2 class="text-sm font-semibold text-danger uppercase tracking-wide">Danger Zone</h2>

			<!-- Clear all data -->
			<div class="space-y-2">
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
			</div>

			<!-- Shutdown daemon -->
			<div class="space-y-2">
				{#if showShutdownConfirm}
					<div class="bg-danger/10 border border-danger/30 rounded p-3 space-y-2">
						<p class="text-sm text-text">This will gracefully shut down the daemon process. You'll need to restart it manually.</p>
						<div class="flex gap-2">
							<button
								class="bg-danger text-white px-3 py-1.5 rounded text-sm font-semibold hover:opacity-90 transition-opacity"
								onclick={handleShutdown}
								disabled={shuttingDown}
							>
								{shuttingDown ? 'Shutting down...' : 'Yes, stop daemon'}
							</button>
							<button
								class="bg-bg-tertiary text-text px-3 py-1.5 rounded text-sm hover:bg-bg-secondary transition-colors"
								onclick={() => showShutdownConfirm = false}
							>
								Cancel
							</button>
						</div>
					</div>
				{:else}
					<button
						class="border border-danger/50 text-danger px-3 py-1.5 rounded text-sm hover:bg-danger/10 transition-colors"
						onclick={() => showShutdownConfirm = true}
						disabled={!health}
					>
						Stop daemon
					</button>
				{/if}
			</div>
		</section>
	{/if}
</div>
