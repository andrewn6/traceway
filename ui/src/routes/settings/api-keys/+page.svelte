<script lang="ts">
	import { getApiKeys, createApiKey, deleteApiKey, type ApiKeyInfo, type ApiKeyCreated } from '$lib/api';
	import FloatingInspector from '$lib/components/FloatingInspector.svelte';
	import { onMount } from 'svelte';

	let keys: ApiKeyInfo[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	// Create form
	let showCreate = $state(false);
	let newKeyName = $state('');
	let creating = $state(false);
	let createPanelWidth: 'compact' | 'default' | 'wide' = $state('default');

	// Newly created key (shown once)
	let createdKey: ApiKeyCreated | null = $state(null);
	let copied = $state(false);

	// Delete confirm
	let deletingId: string | null = $state(null);

	async function loadKeys() {
		try {
			keys = await getApiKeys();
		} catch (e) {
			error = 'Failed to load API keys';
		}
		loading = false;
	}

	async function handleCreate(e: Event) {
		e.preventDefault();
		if (!newKeyName.trim()) return;
		creating = true;
		error = '';

		try {
			createdKey = await createApiKey(newKeyName.trim());
			showCreate = false;
			newKeyName = '';
			await loadKeys();
		} catch (e) {
			error = 'Failed to create API key';
		}
		creating = false;
	}

	async function handleDelete(id: string) {
		try {
			await deleteApiKey(id);
			keys = keys.filter((k) => k.id !== id);
			deletingId = null;
		} catch {
			error = 'Failed to delete API key';
		}
	}

	function copyKey() {
		if (createdKey) {
			navigator.clipboard.writeText(createdKey.key);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		}
	}

	onMount(loadKeys);
</script>

<div class="w-full space-y-5">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold tracking-tight">API Keys</h1>
			<p class="text-xs text-text-muted mt-1">Create and rotate ingestion keys for SDKs and external tools.</p>
		</div>
		<button
			onclick={() => { showCreate = true; createdKey = null; }}
			class="btn-primary"
		>
			Create key
		</button>
	</div>

	<p class="text-text-muted text-sm">
		API keys allow SDKs and external tools to send traces to your organization.
		Set <code class="bg-bg-tertiary px-1 rounded text-xs">TRACEWAY_API_KEY</code> in your environment.
	</p>

	{#if error}
		<div class="alert-danger">
			{error}
		</div>
	{/if}

	<!-- Newly created key banner -->
	{#if createdKey}
		<div class="glass-surface border-success/30 rounded-2xl p-4 space-y-2">
			<p class="text-sm font-semibold text-text">Key created: {createdKey.name}</p>
			<p class="text-xs text-text-muted">Copy this key now. You won't be able to see it again.</p>
			<div class="flex items-center gap-2">
				<code class="flex-1 bg-bg-tertiary border border-border rounded-lg px-3 py-2 text-sm font-mono text-text select-all break-all">
					{createdKey.key}
				</code>
				<button
					onclick={copyKey}
					class="shrink-0 px-3 py-2 text-xs bg-bg-tertiary border border-border rounded-lg hover:bg-bg-secondary transition-colors"
				>
					{copied ? 'Copied!' : 'Copy'}
				</button>
			</div>
		</div>
	{/if}

	<!-- Create form -->
	<FloatingInspector
		open={showCreate}
		title="New API key"
		subtitle="Create an ingestion key"
		width={createPanelWidth}
		on:close={() => (showCreate = false)}
		on:width={(e) => (createPanelWidth = e.detail.width)}
	>
		<form onsubmit={handleCreate} class="space-y-3">
			<div>
				<label for="key-name" class="label-micro block mb-1">Key name</label>
				<input
					id="key-name"
					type="text"
					bind:value={newKeyName}
					required
					placeholder="e.g. Production, CI/CD, Development"
					class="control-input"
				/>
			</div>
			<div class="flex gap-2">
				<button type="submit" disabled={creating} class="btn-primary">{creating ? 'Creating...' : 'Create'}</button>
				<button type="button" onclick={() => (showCreate = false)} class="btn-secondary">Cancel</button>
			</div>
		</form>
	</FloatingInspector>

	<!-- Keys list -->
	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if keys.length === 0}
		<div class="table-float text-text-muted text-sm py-8 text-center">
			No API keys yet. Create one to get started.
		</div>
	{:else}
		<div class="table-float divide-y divide-border/40">
			{#each keys as key}
				<div class="px-4 py-3 flex items-center justify-between hover:bg-bg-tertiary/20 transition-colors">
					<div class="space-y-0.5">
						<div class="text-sm font-medium text-text">{key.name}</div>
						<div class="text-xs text-text-muted font-mono">
							{key.key_prefix}...
							<span class="ml-2 text-text-muted">
								Created {new Date(key.created_at).toLocaleDateString()}
							</span>
							{#if key.last_used_at}
								<span class="ml-2">
									Last used {new Date(key.last_used_at).toLocaleDateString()}
								</span>
							{/if}
						</div>
					</div>
					<div>
						{#if deletingId === key.id}
							<div class="flex items-center gap-2">
								<span class="text-xs text-text-muted">Delete?</span>
								<button
									onclick={() => handleDelete(key.id)}
									class="text-xs text-danger hover:underline"
								>
									Yes
								</button>
								<button
									onclick={() => (deletingId = null)}
									class="text-xs text-text-muted hover:text-text"
								>
									No
								</button>
							</div>
						{:else}
							<button
								onclick={() => (deletingId = key.id)}
								class="text-xs text-text-muted hover:text-danger transition-colors"
							>
								Delete
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
