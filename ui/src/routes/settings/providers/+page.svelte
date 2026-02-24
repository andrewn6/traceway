<script lang="ts">
	import {
		listProviderConnections,
		createProviderConnection,
		updateProviderConnection,
		deleteProviderConnection,
		type ProviderConnectionInfo
	} from '$lib/api';
	import { onMount } from 'svelte';

	let connections: ProviderConnectionInfo[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	// Create form
	let showCreate = $state(false);
	let creating = $state(false);
	let formName = $state('');
	let formProvider = $state('openai');
	let formBaseUrl = $state('');
	let formApiKey = $state('');
	let formDefaultModel = $state('');

	// Edit state
	let editingId: string | null = $state(null);
	let editName = $state('');
	let editProvider = $state('openai');
	let editBaseUrl = $state('');
	let editApiKey = $state('');
	let editDefaultModel = $state('');
	let saving = $state(false);

	// Delete confirm
	let deletingId: string | null = $state(null);

	async function loadConnections() {
		try {
			const resp = await listProviderConnections();
			connections = resp.connections;
		} catch {
			error = 'Failed to load provider connections';
		}
		loading = false;
	}

	async function handleCreate(e: Event) {
		e.preventDefault();
		if (!formName.trim()) return;
		creating = true;
		error = '';

		try {
			await createProviderConnection({
				name: formName.trim(),
				provider: formProvider,
				base_url: formBaseUrl.trim() || undefined,
				api_key: formApiKey.trim() || undefined,
				default_model: formDefaultModel.trim() || undefined
			});
			showCreate = false;
			formName = '';
			formProvider = 'openai';
			formBaseUrl = '';
			formApiKey = '';
			formDefaultModel = '';
			await loadConnections();
		} catch {
			error = 'Failed to create provider connection';
		}
		creating = false;
	}

	function startEdit(conn: ProviderConnectionInfo) {
		editingId = conn.id;
		editName = conn.name;
		editProvider = conn.provider;
		editBaseUrl = conn.base_url ?? '';
		editApiKey = '';
		editDefaultModel = conn.default_model ?? '';
	}

	function cancelEdit() {
		editingId = null;
	}

	async function handleUpdate(e: Event) {
		e.preventDefault();
		if (!editingId || !editName.trim()) return;
		saving = true;
		error = '';

		try {
			await updateProviderConnection(editingId, {
				name: editName.trim(),
				provider: editProvider,
				base_url: editBaseUrl.trim() || undefined,
				api_key: editApiKey.trim() || undefined,
				default_model: editDefaultModel.trim() || undefined
			});
			editingId = null;
			await loadConnections();
		} catch {
			error = 'Failed to update provider connection';
		}
		saving = false;
	}

	async function handleDelete(id: string) {
		try {
			await deleteProviderConnection(id);
			connections = connections.filter((c) => c.id !== id);
			deletingId = null;
		} catch {
			error = 'Failed to delete provider connection';
		}
	}

	onMount(loadConnections);
</script>

<div class="max-w-3xl space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold">Provider Connections</h1>
		<button
			onclick={() => { showCreate = true; }}
			class="px-3 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors"
		>
			Add connection
		</button>
	</div>

	<p class="text-text-muted text-sm">
		Save API credentials for LLM providers. Reference them when creating eval runs instead of manually entering keys each time.
	</p>

	{#if error}
		<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">
			{error}
		</div>
	{/if}

	<!-- Create form -->
	{#if showCreate}
		<form onsubmit={handleCreate} class="bg-bg-secondary border border-border rounded p-4 space-y-3">
			<h2 class="text-sm font-semibold text-text">New Provider Connection</h2>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label for="conn-name" class="block text-xs text-text-secondary mb-1">Name *</label>
					<input
						id="conn-name"
						type="text"
						bind:value={formName}
						required
						placeholder="e.g. OpenAI Production"
						class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
					/>
				</div>
				<div>
					<label for="conn-provider" class="block text-xs text-text-secondary mb-1">Provider *</label>
					<select
						id="conn-provider"
						bind:value={formProvider}
						class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text focus:outline-none focus:border-accent"
					>
						<option value="openai">openai</option>
						<option value="anthropic">anthropic</option>
						<option value="ollama">ollama</option>
						<option value="custom">custom</option>
					</select>
				</div>
			</div>
			<div>
				<label for="conn-url" class="block text-xs text-text-secondary mb-1">Base URL</label>
				<input
					id="conn-url"
					type="text"
					bind:value={formBaseUrl}
					placeholder="https://api.openai.com/v1 (leave blank for default)"
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
				/>
			</div>
			<div>
				<label for="conn-key" class="block text-xs text-text-secondary mb-1">API Key</label>
				<input
					id="conn-key"
					type="password"
					bind:value={formApiKey}
					placeholder="sk-..."
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
				/>
			</div>
			<div>
				<label for="conn-model" class="block text-xs text-text-secondary mb-1">Default Model</label>
				<input
					id="conn-model"
					type="text"
					bind:value={formDefaultModel}
					placeholder="gpt-4o"
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

	<!-- Connections list -->
	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if connections.length === 0}
		<div class="text-text-muted text-sm py-8 text-center bg-bg-secondary border border-border rounded">
			No provider connections yet. Add one to get started.
		</div>
	{:else}
		<div class="bg-bg-secondary border border-border rounded divide-y divide-border">
			{#each connections as conn}
				{#if editingId === conn.id}
					<!-- Edit form inline -->
					<form onsubmit={handleUpdate} class="p-4 space-y-3">
						<div class="grid grid-cols-2 gap-3">
							<div>
								<label for="edit-name" class="block text-xs text-text-secondary mb-1">Name *</label>
								<input
									id="edit-name"
									type="text"
									bind:value={editName}
									required
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text focus:outline-none focus:border-accent"
								/>
							</div>
							<div>
								<label for="edit-provider" class="block text-xs text-text-secondary mb-1">Provider</label>
								<select
									id="edit-provider"
									bind:value={editProvider}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text focus:outline-none focus:border-accent"
								>
									<option value="openai">openai</option>
									<option value="anthropic">anthropic</option>
									<option value="ollama">ollama</option>
									<option value="custom">custom</option>
								</select>
							</div>
						</div>
						<div>
							<label for="edit-url" class="block text-xs text-text-secondary mb-1">Base URL</label>
							<input
								id="edit-url"
								type="text"
								bind:value={editBaseUrl}
								placeholder="https://api.openai.com/v1"
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
							/>
						</div>
						<div>
							<label for="edit-key" class="block text-xs text-text-secondary mb-1">API Key (leave blank to keep existing)</label>
							<input
								id="edit-key"
								type="password"
								bind:value={editApiKey}
								placeholder="Enter new key or leave blank"
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
							/>
						</div>
						<div>
							<label for="edit-model" class="block text-xs text-text-secondary mb-1">Default Model</label>
							<input
								id="edit-model"
								type="text"
								bind:value={editDefaultModel}
								placeholder="gpt-4o"
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent"
							/>
						</div>
						<div class="flex gap-2">
							<button
								type="submit"
								disabled={saving}
								class="px-4 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50"
							>
								{saving ? 'Saving...' : 'Save'}
							</button>
							<button
								type="button"
								onclick={cancelEdit}
								class="px-4 py-1.5 text-sm bg-bg-tertiary text-text rounded hover:bg-bg-secondary transition-colors"
							>
								Cancel
							</button>
						</div>
					</form>
				{:else}
					<!-- Display row -->
					<div class="px-4 py-3 flex items-center justify-between">
						<div class="space-y-0.5">
							<div class="text-sm font-medium text-text">{conn.name}</div>
							<div class="text-xs text-text-muted font-mono space-x-2">
								<span class="px-1.5 py-0.5 bg-bg-tertiary rounded">{conn.provider}</span>
								{#if conn.default_model}
									<span>model: {conn.default_model}</span>
								{/if}
								{#if conn.api_key_preview}
									<span>key: {conn.api_key_preview}</span>
								{/if}
								{#if conn.base_url}
									<span>url: {conn.base_url}</span>
								{/if}
								<span class="ml-2 text-text-muted">
									Created {new Date(conn.created_at).toLocaleDateString()}
								</span>
							</div>
						</div>
						<div class="flex items-center gap-3">
							<button
								onclick={() => startEdit(conn)}
								class="text-xs text-text-muted hover:text-text transition-colors"
							>
								Edit
							</button>
							{#if deletingId === conn.id}
								<div class="flex items-center gap-2">
									<span class="text-xs text-text-muted">Delete?</span>
									<button
										onclick={() => handleDelete(conn.id)}
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
									onclick={() => (deletingId = conn.id)}
									class="text-xs text-text-muted hover:text-danger transition-colors"
								>
									Delete
								</button>
							{/if}
						</div>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
