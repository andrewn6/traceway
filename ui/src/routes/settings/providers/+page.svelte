<script lang="ts">
	import {
		listProviderConnections,
		createProviderConnection,
		updateProviderConnection,
		deleteProviderConnection,
		testProviderConnection,
		listProviderModels,
		type ProviderConnectionInfo,
		type ProviderModelInfo
	} from '$lib/api';
	import { onMount } from 'svelte';

	// ─── Provider catalog ───────────────────────────────────────────────

	interface ProviderDef {
		id: string;
		name: string;
		icon: string;
		color: string;
		baseUrl: string;
		defaultModel: string;
		keyPlaceholder: string;
		needsKey: boolean;
	}

	const PROVIDERS: ProviderDef[] = [
		{
			id: 'openai',
			name: 'OpenAI',
			icon: `<svg viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path d="M22.282 9.821a5.985 5.985 0 0 0-.516-4.91 6.046 6.046 0 0 0-6.51-2.9A6.065 6.065 0 0 0 4.981 4.18a5.985 5.985 0 0 0-3.998 2.9 6.046 6.046 0 0 0 .743 7.097 5.98 5.98 0 0 0 .51 4.911 6.051 6.051 0 0 0 6.515 2.9A5.985 5.985 0 0 0 13.26 24a6.056 6.056 0 0 0 5.772-4.206 5.99 5.99 0 0 0 3.997-2.9 6.056 6.056 0 0 0-.747-7.073zM13.26 22.43a4.476 4.476 0 0 1-2.876-1.04l.141-.081 4.779-2.758a.795.795 0 0 0 .392-.681v-6.737l2.02 1.168a.071.071 0 0 1 .038.052v5.583a4.504 4.504 0 0 1-4.494 4.494zM3.6 18.304a4.47 4.47 0 0 1-.535-3.014l.142.085 4.783 2.759a.771.771 0 0 0 .78 0l5.843-3.369v2.332a.08.08 0 0 1-.033.062L9.74 19.95a4.5 4.5 0 0 1-6.14-1.646zM2.34 7.896a4.485 4.485 0 0 1 2.366-1.973V11.6a.766.766 0 0 0 .388.676l5.815 3.355-2.02 1.168a.076.076 0 0 1-.071 0l-4.83-2.786A4.504 4.504 0 0 1 2.34 7.872zm16.597 3.855l-5.833-3.387L15.119 7.2a.076.076 0 0 1 .071 0l4.83 2.791a4.494 4.494 0 0 1-.676 8.105v-5.678a.79.79 0 0 0-.407-.667zm2.01-3.023l-.141-.085-4.774-2.782a.776.776 0 0 0-.785 0L9.409 9.23V6.897a.066.066 0 0 1 .028-.061l4.83-2.787a4.5 4.5 0 0 1 6.68 4.66zm-12.64 4.135l-2.02-1.164a.08.08 0 0 1-.038-.057V6.075a4.5 4.5 0 0 1 7.375-3.453l-.142.08L8.704 5.46a.795.795 0 0 0-.393.681zm1.097-2.365l2.602-1.5 2.607 1.5v2.999l-2.597 1.5-2.607-1.5z"/></svg>`,
			color: '#10a37f',
			baseUrl: 'https://api.openai.com/v1',
			defaultModel: 'gpt-4o',
			keyPlaceholder: 'sk-...',
			needsKey: true,
		},
		{
			id: 'anthropic',
			name: 'Anthropic',
			icon: `<svg viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path d="M17.304 3.541h-3.677l6.696 16.918h3.677zm-10.608 0L0 20.459h3.744l1.37-3.553h7.005l1.369 3.553h3.744L10.536 3.541zm-.443 10.34 2.378-6.161 2.377 6.161z"/></svg>`,
			color: '#d4a27f',
			baseUrl: 'https://api.anthropic.com/v1',
			defaultModel: 'claude-sonnet-4-20250514',
			keyPlaceholder: 'sk-ant-...',
			needsKey: true,
		},
		{
			id: 'ollama',
			name: 'Ollama',
			icon: `<svg viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2c-2.5 0-4.71-1.28-6-3.22.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08-1.29 1.94-3.5 3.22-6 3.22z"/></svg>`,
			color: '#ffffff',
			baseUrl: 'http://localhost:11434/v1',
			defaultModel: 'llama3.2',
			keyPlaceholder: 'Not required',
			needsKey: false,
		},
		{
			id: 'custom',
			name: 'Custom / OpenAI-compatible',
			icon: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M10.343 3.94c.09-.542.56-.94 1.11-.94h1.093c.55 0 1.02.398 1.11.94l.149.894c.07.424.384.764.78.93.398.164.855.142 1.205-.108l.737-.527a1.125 1.125 0 0 1 1.45.12l.773.774c.39.389.44 1.002.12 1.45l-.527.737c-.25.35-.272.806-.107 1.204.165.397.505.71.93.78l.893.15c.543.09.94.559.94 1.109v1.094c0 .55-.397 1.02-.94 1.11l-.894.149c-.424.07-.764.383-.929.78-.165.398-.143.854.107 1.204l.527.738c.32.447.269 1.06-.12 1.45l-.774.773a1.125 1.125 0 0 1-1.449.12l-.738-.527c-.35-.25-.806-.272-1.203-.107-.398.165-.71.505-.781.929l-.149.894c-.09.542-.56.94-1.11.94h-1.094c-.55 0-1.019-.398-1.11-.94l-.148-.894c-.071-.424-.384-.764-.781-.93-.398-.164-.854-.142-1.204.109l-.738.527c-.447.32-1.06.269-1.45-.12l-.773-.774a1.125 1.125 0 0 1-.12-1.45l.527-.737c.25-.35.272-.806.108-1.204-.165-.397-.506-.71-.93-.78l-.894-.15c-.542-.09-.94-.56-.94-1.109v-1.094c0-.55.398-1.02.94-1.11l.894-.149c.424-.07.765-.383.93-.78.165-.398.143-.854-.108-1.204l-.526-.738a1.125 1.125 0 0 1 .12-1.45l.773-.773a1.125 1.125 0 0 1 1.45-.12l.737.527c.35.25.807.272 1.204.107.397-.165.71-.505.78-.929z" /><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z" /></svg>`,
			color: '#6ee7b7',
			baseUrl: '',
			defaultModel: '',
			keyPlaceholder: 'API key',
			needsKey: true,
		},
	];

	function getProviderDef(id: string): ProviderDef {
		return PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[PROVIDERS.length - 1];
	}

	// ─── State ──────────────────────────────────────────────────────────

	let connections: ProviderConnectionInfo[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let success = $state('');

	// Add flow
	let addingProvider: ProviderDef | null = $state(null);
	let formName = $state('');
	let formBaseUrl = $state('');
	let formApiKey = $state('');
	let formDefaultModel = $state('');
	let creating = $state(false);

	// Test / models
	let testing = $state(false);
	let testOk = $state(false);
	let testError = $state('');
	let availableModels: ProviderModelInfo[] = $state([]);
	let modelSearch = $state('');

	// Edit state
	let editingId: string | null = $state(null);
	let editName = $state('');
	let editProvider = $state('openai');
	let editBaseUrl = $state('');
	let editApiKey = $state('');
	let editDefaultModel = $state('');
	let editModels: ProviderModelInfo[] = $state([]);
	let editModelSearch = $state('');
	let editLoadingModels = $state(false);
	let saving = $state(false);

	// Delete confirm
	let deletingId: string | null = $state(null);

	// Derived: filtered model list for search
	let filteredModels = $derived.by(() => {
		const q = modelSearch.toLowerCase();
		if (!q) return availableModels;
		return availableModels.filter((m) => m.id.toLowerCase().includes(q));
	});

	let filteredEditModels = $derived.by(() => {
		const q = editModelSearch.toLowerCase();
		if (!q) return editModels;
		return editModels.filter((m) => m.id.toLowerCase().includes(q));
	});

	// ─── Actions ────────────────────────────────────────────────────────

	async function loadConnections() {
		try {
			const resp = await listProviderConnections();
			connections = resp.connections;
			error = '';
		} catch (e: any) {
			error = e?.message || 'Failed to load provider connections';
		}
		loading = false;
	}

	function startAdd(provider: ProviderDef) {
		addingProvider = provider;
		formName = provider.name;
		formBaseUrl = provider.baseUrl;
		formApiKey = '';
		formDefaultModel = provider.defaultModel;
		testOk = false;
		testError = '';
		availableModels = [];
		modelSearch = '';
		error = '';
	}

	function cancelAdd() {
		addingProvider = null;
		availableModels = [];
		testOk = false;
		testError = '';
	}

	async function handleTest() {
		if (!addingProvider) return;
		testing = true;
		testOk = false;
		testError = '';
		availableModels = [];

		try {
			const resp = await testProviderConnection({
				provider: addingProvider.id,
				base_url: formBaseUrl.trim() || undefined,
				api_key: formApiKey.trim() || undefined,
			});

			if (resp.ok) {
				testOk = true;
				availableModels = resp.models;
				// If the current default model isn't in the list and there are models, keep it
				// (provider may have returned a subset or the model name may differ)
			} else {
				testError = resp.error || 'Connection failed';
			}
		} catch (e: any) {
			testError = e?.message || 'Test request failed';
		}
		testing = false;
	}

	async function handleCreate() {
		if (!addingProvider || !formName.trim()) return;
		creating = true;
		error = '';

		try {
			await createProviderConnection({
				name: formName.trim(),
				provider: addingProvider.id,
				base_url: formBaseUrl.trim() || undefined,
				api_key: formApiKey.trim() || undefined,
				default_model: formDefaultModel.trim() || undefined,
			});
			success = `${formName.trim()} connected`;
			addingProvider = null;
			availableModels = [];
			await loadConnections();
			setTimeout(() => (success = ''), 3000);
		} catch (e: any) {
			error = e?.message || 'Failed to create connection';
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
		editModels = [];
		editModelSearch = '';
		// Fetch models for this saved connection
		loadModelsForConnection(conn.id);
	}

	async function loadModelsForConnection(connId: string) {
		editLoadingModels = true;
		try {
			const resp = await listProviderModels(connId);
			if (resp.ok) {
				editModels = resp.models;
			}
		} catch {
			// Silently fail — they can still type a model name
		}
		editLoadingModels = false;
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
				default_model: editDefaultModel.trim() || undefined,
			});
			editingId = null;
			editModels = [];
			await loadConnections();
		} catch (e: any) {
			error = e?.message || 'Failed to update connection';
		}
		saving = false;
	}

	async function handleDelete(id: string) {
		try {
			await deleteProviderConnection(id);
			connections = connections.filter((c) => c.id !== id);
			deletingId = null;
		} catch (e: any) {
			error = e?.message || 'Failed to delete connection';
		}
	}

	onMount(loadConnections);
</script>

<div class="max-w-3xl space-y-6">
	<div>
		<h1 class="text-xl font-bold">Providers</h1>
		<p class="text-text-muted text-sm mt-1">Connect LLM providers to use in evals and traces.</p>
	</div>

	{#if error}
		<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">{error}</div>
	{/if}
	{#if success}
		<div class="bg-success/10 border border-success/30 rounded px-3 py-2 text-success text-sm">{success}</div>
	{/if}

	<!-- Active connections -->
	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if connections.length > 0}
		<div class="space-y-2">
			{#each connections as conn (conn.id)}
				{@const prov = getProviderDef(conn.provider)}
				{#if editingId === conn.id}
					<!-- Edit form -->
					<form onsubmit={handleUpdate} class="bg-bg-secondary border border-border rounded-lg p-4 space-y-3">
						<div class="flex items-center gap-2 mb-1">
							<div class="w-6 h-6 rounded flex items-center justify-center" style="color: {prov.color}">
								{@html prov.icon}
							</div>
							<span class="text-sm font-semibold text-text">Edit {conn.name}</span>
						</div>

						<div class="grid grid-cols-2 gap-3">
							<div>
								<label for="edit-name" class="block text-xs text-text-secondary mb-1">Name</label>
								<input id="edit-name" type="text" bind:value={editName} required
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text focus:outline-none focus:border-accent" />
							</div>
							<div>
								<label for="edit-model" class="block text-xs text-text-secondary mb-1">Default Model</label>
								{#if editModels.length > 0}
									<div class="relative">
										<input id="edit-model" type="text" bind:value={editDefaultModel} placeholder="Search models..."
											onfocus={() => (editModelSearch = editDefaultModel)}
											oninput={(e: Event) => { editDefaultModel = (e.target as HTMLInputElement).value; editModelSearch = editDefaultModel; }}
											class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono" />
										{#if editModelSearch && filteredEditModels.length > 0}
											<div class="absolute z-10 top-full left-0 right-0 mt-1 bg-bg-tertiary border border-border rounded shadow-lg max-h-48 overflow-y-auto">
												{#each filteredEditModels.slice(0, 30) as model}
													<button type="button"
														onclick={() => { editDefaultModel = model.id; editModelSearch = ''; }}
														class="w-full text-left px-3 py-1.5 text-xs font-mono text-text hover:bg-bg-secondary transition-colors {model.id === editDefaultModel ? 'bg-accent/10 text-accent' : ''}">
														{model.id}
													</button>
												{/each}
											</div>
										{/if}
									</div>
								{:else if editLoadingModels}
									<div class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text-muted">
										Loading models...
									</div>
								{:else}
									<input id="edit-model" type="text" bind:value={editDefaultModel} placeholder={prov.defaultModel}
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono" />
								{/if}
							</div>
						</div>
						<div>
							<label for="edit-url" class="block text-xs text-text-secondary mb-1">Base URL</label>
							<input id="edit-url" type="text" bind:value={editBaseUrl} placeholder={prov.baseUrl}
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono text-xs" />
						</div>
						<div>
							<label for="edit-key" class="block text-xs text-text-secondary mb-1">API Key <span class="text-text-muted">(leave blank to keep existing)</span></label>
							<input id="edit-key" type="password" bind:value={editApiKey} placeholder="Enter new key"
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent" />
						</div>
						<div class="flex gap-2">
							<button type="submit" disabled={saving}
								class="px-3 py-1.5 text-xs bg-accent text-bg font-semibold rounded hover:bg-accent/80 disabled:opacity-50">
								{saving ? 'Saving...' : 'Save'}
							</button>
							<button type="button" onclick={() => { editingId = null; editModels = []; }}
								class="px-3 py-1.5 text-xs text-text-muted hover:text-text">Cancel</button>
						</div>
					</form>
				{:else}
					<!-- Connection row -->
					<div class="bg-bg-secondary border border-border rounded-lg px-4 py-3 flex items-center gap-3 group">
						<div class="shrink-0 w-8 h-8 rounded-lg flex items-center justify-center" style="background: {prov.color}15; color: {prov.color}">
							{@html prov.icon}
						</div>
						<div class="flex-1 min-w-0">
							<div class="flex items-center gap-2">
								<span class="text-sm font-medium text-text">{conn.name}</span>
								{#if conn.default_model}
									<span class="text-[11px] font-mono text-text-muted bg-bg-tertiary px-1.5 py-0.5 rounded">{conn.default_model}</span>
								{/if}
							</div>
							<div class="text-[11px] text-text-muted font-mono mt-0.5">
								{#if conn.api_key_preview}
									{conn.api_key_preview}
								{:else if !prov.needsKey}
									<span class="text-text-muted">No key required</span>
								{:else}
									<span class="text-warning">No key set</span>
								{/if}
								{#if conn.base_url}
									<span class="mx-1 text-border">|</span>
									{conn.base_url}
								{/if}
							</div>
						</div>
						<div class="shrink-0 flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
							<button onclick={() => startEdit(conn)}
								class="text-xs text-text-muted hover:text-text transition-colors">Edit</button>
							{#if deletingId === conn.id}
								<button onclick={() => handleDelete(conn.id)} class="text-xs text-danger font-medium">Confirm</button>
								<button onclick={() => (deletingId = null)} class="text-xs text-text-muted">Cancel</button>
							{:else}
								<button onclick={() => (deletingId = conn.id)}
									class="text-xs text-text-muted hover:text-danger transition-colors">Delete</button>
							{/if}
						</div>
					</div>
				{/if}
			{/each}
		</div>
	{/if}

	<!-- Add provider flow -->
	{#if addingProvider}
		{@const prov = addingProvider}
		<div class="bg-bg-secondary border rounded-lg overflow-hidden" style="border-color: {prov.color}40">
			<!-- Header -->
			<div class="px-4 py-3 flex items-center justify-between border-b" style="border-color: {prov.color}20; background: {prov.color}08">
				<div class="flex items-center gap-3">
					<div class="w-7 h-7 rounded flex items-center justify-center" style="color: {prov.color}">
						{@html prov.icon}
					</div>
					<span class="text-sm font-semibold text-text">Add {prov.name}</span>
				</div>
				<button onclick={cancelAdd} class="text-xs text-text-muted hover:text-text transition-colors">Cancel</button>
			</div>

			<div class="p-4 space-y-4">
				<!-- Step 1: Credentials -->
				<div class="space-y-3">
					{#if prov.needsKey}
						<div>
							<label for="add-key" class="block text-xs text-text-secondary mb-1">API Key</label>
							<input id="add-key" type="password" bind:value={formApiKey} placeholder={prov.keyPlaceholder}
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono" />
						</div>
					{/if}

					<div class="flex items-center gap-3">
						<button onclick={handleTest} disabled={testing || (prov.needsKey && !formApiKey.trim())}
							class="px-4 py-1.5 text-sm border rounded transition-colors disabled:opacity-40
								{testOk ? 'border-success/50 text-success bg-success/5' : 'border-border text-text-secondary hover:border-accent hover:text-accent'}">
							{#if testing}
								Testing...
							{:else if testOk}
								Connected ({availableModels.length} models)
							{:else}
								Test Connection
							{/if}
						</button>

						{#if testError}
							<span class="text-xs text-danger">{testError}</span>
						{/if}
					</div>
				</div>

				<!-- Step 2: Configuration (shown after test or immediately for no-key providers) -->
				{#if testOk || !prov.needsKey}
					<div class="border-t border-border/50 pt-4 space-y-3">
						<div class="grid grid-cols-2 gap-3">
							<div>
								<label for="add-name" class="block text-xs text-text-secondary mb-1">Connection Name</label>
								<input id="add-name" type="text" bind:value={formName}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text focus:outline-none focus:border-accent" />
							</div>
							<div>
								<label for="add-model" class="block text-xs text-text-secondary mb-1">Default Model</label>
								{#if availableModels.length > 0}
									<!-- Searchable model dropdown -->
									<div class="relative">
										<input id="add-model" type="text" bind:value={formDefaultModel}
											placeholder="Search {availableModels.length} models..."
											onfocus={() => (modelSearch = formDefaultModel)}
											oninput={(e: Event) => { formDefaultModel = (e.target as HTMLInputElement).value; modelSearch = formDefaultModel; }}
											class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono" />
										{#if modelSearch && filteredModels.length > 0 && modelSearch !== formDefaultModel}
											<div class="absolute z-10 top-full left-0 right-0 mt-1 bg-bg-tertiary border border-border rounded shadow-lg max-h-48 overflow-y-auto">
												{#each filteredModels.slice(0, 30) as model}
													<button type="button"
														onclick={() => { formDefaultModel = model.id; modelSearch = ''; }}
														class="w-full text-left px-3 py-1.5 text-xs font-mono text-text hover:bg-bg-secondary transition-colors {model.id === formDefaultModel ? 'bg-accent/10 text-accent' : ''}">
														{model.id}
													</button>
												{/each}
											</div>
										{/if}
									</div>
								{:else}
									<input id="add-model" type="text" bind:value={formDefaultModel} placeholder={prov.defaultModel || 'e.g. gpt-4o'}
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono" />
								{/if}
							</div>
						</div>

						<!-- Advanced: base URL (collapsed by default for known providers) -->
						{#if prov.id === 'custom' || formBaseUrl !== prov.baseUrl}
							<div>
								<label for="add-url" class="block text-xs text-text-secondary mb-1">Base URL</label>
								<input id="add-url" type="text" bind:value={formBaseUrl} placeholder={prov.baseUrl || 'https://api.example.com/v1'}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono" />
							</div>
						{/if}

						<div class="flex items-center gap-2 pt-1">
							<button onclick={handleCreate} disabled={creating || !formName.trim()}
								class="px-4 py-1.5 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 disabled:opacity-50 transition-colors">
								{creating ? 'Saving...' : 'Save Connection'}
							</button>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{:else}
		<!-- Provider catalog cards -->
		<div>
			<h2 class="text-xs font-semibold text-text-muted uppercase tracking-wider mb-3">Add a Provider</h2>
			<div class="grid grid-cols-2 gap-3">
				{#each PROVIDERS as prov}
					<button
						onclick={() => startAdd(prov)}
						class="bg-bg-secondary border border-border rounded-lg p-4 text-left hover:border-opacity-60 transition-all group"
					>
						<div class="flex items-center gap-3">
							<div class="w-9 h-9 rounded-lg flex items-center justify-center transition-colors" style="background: {prov.color}12; color: {prov.color}">
								{@html prov.icon}
							</div>
							<div>
								<div class="text-sm font-medium text-text">{prov.name}</div>
								<div class="text-[11px] text-text-muted">{prov.needsKey ? 'API key required' : 'No key required'}</div>
							</div>
						</div>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
