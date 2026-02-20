<script lang="ts">
	import { getApiKeys, createApiKey, deleteApiKey, type ApiKeyInfo, type ApiKeyCreated } from '$lib/api';
	import { onMount } from 'svelte';

	let keys: ApiKeyInfo[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	// Create form
	let showCreate = $state(false);
	let newKeyName = $state('');
	let creating = $state(false);

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

<div class="max-w-3xl space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold">API Keys</h1>
		<button
			onclick={() => { showCreate = true; createdKey = null; }}
			class="px-3 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors"
		>
			Create key
		</button>
	</div>

	<p class="text-text-muted text-sm">
		API keys allow SDKs and external tools to send traces to your organization.
		Set <code class="bg-bg-tertiary px-1 rounded text-xs">TRACEWAY_API_KEY</code> in your environment.
	</p>

	{#if error}
		<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">
			{error}
		</div>
	{/if}

	<!-- Newly created key banner -->
	{#if createdKey}
		<div class="bg-success/10 border border-success/30 rounded p-4 space-y-2">
			<p class="text-sm font-semibold text-text">Key created: {createdKey.name}</p>
			<p class="text-xs text-text-muted">Copy this key now. You won't be able to see it again.</p>
			<div class="flex items-center gap-2">
				<code class="flex-1 bg-bg-tertiary border border-border rounded px-3 py-2 text-sm font-mono text-text select-all break-all">
					{createdKey.key}
				</code>
				<button
					onclick={copyKey}
					class="shrink-0 px-3 py-2 text-xs bg-bg-tertiary border border-border rounded hover:bg-bg-secondary transition-colors"
				>
					{copied ? 'Copied!' : 'Copy'}
				</button>
			</div>
		</div>
	{/if}

	<!-- Create form -->
	{#if showCreate}
		<form onsubmit={handleCreate} class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text">New API Key</h2>
			<div>
				<label for="key-name" class="block text-xs text-text-secondary mb-1">Key name</label>
				<input
					id="key-name"
					type="text"
					bind:value={newKeyName}
					required
					placeholder="e.g. Production, CI/CD, Development"
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
				/>
			</div>
			<div class="flex gap-2">
				<button
					type="submit"
					disabled={creating}
					class="px-4 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50"
				>
					{creating ? 'Creating...' : 'Create'}
				</button>
				<button
					type="button"
					onclick={() => showCreate = false}
					class="px-4 py-1.5 text-sm bg-bg-tertiary text-text rounded hover:bg-bg-secondary transition-colors"
				>
					Cancel
				</button>
			</div>
		</form>
	{/if}

	<!-- Keys list -->
	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if keys.length === 0}
		<div class="text-text-muted text-sm py-8 text-center bg-bg-secondary border border-border rounded">
			No API keys yet. Create one to get started.
		</div>
	{:else}
		<div class="bg-bg-secondary border border-border rounded divide-y divide-border">
			{#each keys as key}
				<div class="px-4 py-3 flex items-center justify-between">
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
