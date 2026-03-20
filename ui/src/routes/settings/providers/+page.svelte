<script lang="ts">
	import {
		listProviderConnections,
		createProviderConnection,
		updateProviderConnection,
		deleteProviderConnection,
		getAuthConfig,
		type ProviderConnectionInfo,
		type AuthConfig
	} from '$lib/api';
	import FloatingInspector from '$lib/components/FloatingInspector.svelte';
	import { onMount } from 'svelte';

	let authConfig: AuthConfig = $state({ mode: 'local', features: [] });
	const isLocalMode = $derived(authConfig.mode === 'local');

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
			id: 'gemini',
			name: 'Google Gemini',
			icon: `<svg viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path d="M12 0C5.4 0 0 5.4 0 12s5.4 12 12 12c2.8 0 5.4-1 7.4-2.6l-3.7-2.8c-1.1.7-2.4 1.1-3.7 1.1-3.4 0-6.2-2.3-7.2-5.4h-.1c1-3.1 3.8-5.3 7.3-5.3 1.3 0 2.5.3 3.5 1l3.6-2.8C17.2 1 14.7 0 12 0z"/></svg>`,
			color: '#4285f4',
			baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
			defaultModel: 'gemini-2.0-flash',
			keyPlaceholder: 'AIza...',
			needsKey: true,
		},
		{
			id: 'groq',
			name: 'Groq',
			icon: `<svg viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 3a7 7 0 1 1 0 14 7 7 0 0 1 0-14zm0 2.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9z"/></svg>`,
			color: '#f55036',
			baseUrl: 'https://api.groq.com/openai/v1',
			defaultModel: 'llama-3.3-70b-versatile',
			keyPlaceholder: 'gsk_...',
			needsKey: true,
		},
		{
			id: 'mistral',
			name: 'Mistral',
			icon: `<svg viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><rect x="3" y="3" width="5" height="5" rx="0.5"/><rect x="3" y="10" width="5" height="5" rx="0.5"/><rect x="10" y="3" width="5" height="5" rx="0.5"/><rect x="10" y="10" width="5" height="5" rx="0.5"/><rect x="17" y="10" width="5" height="5" rx="0.5"/><rect x="3" y="17" width="5" height="5" rx="0.5"/><rect x="17" y="17" width="5" height="5" rx="0.5"/></svg>`,
			color: '#ff7000',
			baseUrl: 'https://api.mistral.ai/v1',
			defaultModel: 'mistral-large-latest',
			keyPlaceholder: 'api-key...',
			needsKey: true,
		},
		{
			id: 'azure',
			name: 'Azure OpenAI',
			icon: `<svg viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path d="M13.05 4.24L6.56 18.05l2.11-.01 1.05-2.43h6.73l1.08 2.43 2.03.01L13.05 4.24zM10.57 13.75L13.15 7.4l2.51 6.35h-5.09zM5.88 18.34l-2.69 2.95h5.77l-3.08-2.95zM2 3.47l1.74 8.12 4.85-5.5L2 3.47z"/></svg>`,
			color: '#0078d4',
			baseUrl: '',
			defaultModel: 'gpt-4o',
			keyPlaceholder: 'Azure API key...',
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

	const PROVIDER_MODELS: Record<string, string[]> = {
		openai: [
			'gpt-4o',
			'gpt-4o-mini',
			'gpt-4.1',
			'gpt-4.1-mini',
			'gpt-4.1-nano',
			'o3',
			'o3-mini',
			'o4-mini',
		],
		anthropic: [
			'claude-opus-4-20250514',
			'claude-sonnet-4-20250514',
			'claude-haiku-4-20250414',
		],
		gemini: [
			'gemini-2.5-pro',
			'gemini-2.5-flash',
			'gemini-2.0-flash',
			'gemini-2.0-flash-lite',
		],
		groq: [
			'llama-3.3-70b-versatile',
			'llama-3.1-8b-instant',
			'llama-guard-3-8b',
			'gemma2-9b-it',
			'mixtral-8x7b-32768',
		],
		mistral: [
			'mistral-large-latest',
			'mistral-medium-latest',
			'mistral-small-latest',
			'codestral-latest',
			'open-mistral-nemo',
		],
		azure: [
			'gpt-4o',
			'gpt-4o-mini',
			'gpt-4.1',
			'gpt-4.1-mini',
		],
		ollama: [
			'llama3.2',
			'llama3.1',
			'gemma2',
			'mistral',
			'codellama',
			'phi3',
		],
		custom: [],
	};

	function getModelsForProvider(providerId: string): string[] {
		return PROVIDER_MODELS[providerId] ?? [];
	}

	function getProviderDef(id: string): ProviderDef {
		return PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[PROVIDERS.length - 1];
	}

	// ─── State ──────────────────────────────────────────────────────────

	let connections: ProviderConnectionInfo[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let success = $state('');

	// Modal state
	let showModal = $state(false);
	let selectedProviderId = $state('');
	let formName = $state('');
	let formBaseUrl = $state('');
	let formApiKey = $state('');
	let formDefaultModel = $state('');
	let creating = $state(false);
	let modalWidth: 'compact' | 'default' | 'wide' = $state('default');

	// Edit state
	let editingId: string | null = $state(null);
	let editName = $state('');
	let editBaseUrl = $state('');
	let editApiKey = $state('');
	let editDefaultModel = $state('');
	let saving = $state(false);

	// Delete confirm
	let deletingId: string | null = $state(null);

	// Derived
	const selectedProvider = $derived(PROVIDERS.find(p => p.id === selectedProviderId));

	const visibleProviders = $derived(
		isLocalMode ? PROVIDERS : PROVIDERS.filter(p => p.id !== 'ollama')
	);

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

	function openAddModal() {
		showModal = true;
		selectedProviderId = '';
		formName = '';
		formBaseUrl = '';
		formApiKey = '';
		formDefaultModel = '';
		error = '';
	}

	function closeModal() {
		showModal = false;
		selectedProviderId = '';
	}

	function selectProvider(id: string) {
		selectedProviderId = id;
		const prov = PROVIDERS.find(p => p.id === id);
		if (prov) {
			formName = prov.name;
			formBaseUrl = prov.baseUrl;
			formDefaultModel = prov.defaultModel;
		}
	}

	async function handleCreate() {
		if (!selectedProvider || !formName.trim()) return;
		creating = true;
		error = '';

		try {
			await createProviderConnection({
				name: formName.trim(),
				provider: selectedProvider.id,
				base_url: formBaseUrl.trim() || undefined,
				api_key: formApiKey.trim() || undefined,
				default_model: formDefaultModel.trim() || undefined,
			});
			success = `${formName.trim()} added`;
			closeModal();
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
		editBaseUrl = conn.base_url ?? '';
		editApiKey = '';
		editDefaultModel = conn.default_model ?? '';
	}

	async function handleUpdate(e: Event) {
		e.preventDefault();
		if (!editingId || !editName.trim()) return;
		saving = true;
		error = '';

		try {
			const conn = connections.find(c => c.id === editingId);
			await updateProviderConnection(editingId, {
				name: editName.trim(),
				provider: conn?.provider || 'openai',
				base_url: editBaseUrl.trim() || undefined,
				api_key: editApiKey.trim() || undefined,
				default_model: editDefaultModel.trim() || undefined,
			});
			editingId = null;
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

	onMount(() => {
		loadConnections();
		getAuthConfig().then(c => { authConfig = c; }).catch(() => {});
	});
</script>

<div class="w-full space-y-5">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-xl font-semibold tracking-tight">Providers</h1>
			<p class="text-text-muted text-sm mt-1">Connect LLM providers to use in evals and traces.</p>
		</div>
		<button
			onclick={openAddModal}
			class="btn-primary px-4"
		>+ Add Provider</button>
	</div>

	{#if error && !showModal}
		<div class="alert-danger">{error}</div>
	{/if}
	{#if success}
		<div class="alert-success">{success}</div>
	{/if}

	<!-- Active connections -->
	{#if loading}
		<div class="text-text-muted text-sm py-10 text-center">Loading...</div>
	{:else if connections.length === 0 && !showModal}
		<div class="table-float text-center py-12 border-dashed">
			<p class="text-text-muted text-sm mb-1">No providers connected yet.</p>
			<p class="text-text-muted text-xs mb-4">Add a provider to run evals and use LLM features.</p>
			<button
				onclick={openAddModal}
				class="btn-primary px-4"
			>+ Add Provider</button>
		</div>
	{:else}
		<div class="table-float divide-y divide-border/40">
			{#each connections as conn (conn.id)}
				{@const prov = getProviderDef(conn.provider)}
				{#if editingId === conn.id}
					<!-- Edit form -->
					<form onsubmit={handleUpdate} class="bg-bg-secondary border-b border-border p-4 space-y-3">
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
									class="w-full bg-bg-tertiary border border-border rounded-lg px-3 py-1.5 text-sm text-text focus:outline-none focus:border-accent" />
							</div>
							<div>
								<label for="edit-model" class="block text-xs text-text-secondary mb-1">Default Model</label>
								{#if getModelsForProvider(conn.provider).length > 0}
									{@const editModels = getModelsForProvider(conn.provider)}
									<div class="relative">
										<select
											id="edit-model"
											bind:value={editDefaultModel}
											class="w-full bg-bg-tertiary border border-border rounded-lg px-3 py-1.5 text-sm text-text focus:outline-none focus:border-accent font-mono appearance-none cursor-pointer"
										>
											{#each editModels as model}
												<option value={model}>{model}</option>
											{/each}
										</select>
										<div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-text-muted">
											<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
												<path stroke-linecap="round" stroke-linejoin="round" d="M8 9l4-4 4 4m0 6l-4 4-4-4" />
											</svg>
										</div>
									</div>
								{:else}
									<input id="edit-model" type="text" bind:value={editDefaultModel} placeholder={prov.defaultModel}
										class="w-full bg-bg-tertiary border border-border rounded-lg px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono" />
								{/if}
							</div>
						</div>
						<div>
							<label for="edit-url" class="block text-xs text-text-secondary mb-1">Base URL</label>
							<input id="edit-url" type="text" bind:value={editBaseUrl} placeholder={prov.baseUrl}
								class="w-full bg-bg-tertiary border border-border rounded-lg px-3 py-1.5 text-text placeholder:text-text-muted focus:outline-none focus:border-accent font-mono text-xs" />
						</div>
						<div>
							<label for="edit-key" class="block text-xs text-text-secondary mb-1">API Key <span class="text-text-muted">(leave blank to keep existing)</span></label>
							<input id="edit-key" type="password" bind:value={editApiKey} placeholder="Enter new key"
								class="w-full bg-bg-tertiary border border-border rounded-lg px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-accent" />
						</div>
						<div class="flex gap-2">
							<button type="submit" disabled={saving} class="btn-primary h-7 text-xs">
								{saving ? 'Saving...' : 'Save'}
							</button>
							<button type="button" onclick={() => { editingId = null; }} class="btn-ghost h-7 text-xs">Cancel</button>
						</div>
					</form>
				{:else}
					<!-- Connection row -->
					<div class="px-4 py-3 flex items-center gap-3 group hover:bg-bg-tertiary/20 transition-colors">
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
</div>

<!-- Add Provider Modal -->
{#if showModal}
	<FloatingInspector
		open={showModal}
		title="Add API key"
		subtitle={selectedProvider ? selectedProvider.name : 'Provider connection'}
		width={modalWidth}
		onclose={closeModal}
		onwidth={(w) => (modalWidth = w)}
	>
		<div class="space-y-4">
				{#if error}
					<div class="alert-danger">{error}</div>
				{/if}

				<!-- Name -->
				<div>
					<label for="modal-name" class="label-micro block mb-1.5">Name</label>
					<input id="modal-name" type="text" bind:value={formName} placeholder="My OpenAI key"
						class="control-input" />
				</div>

				<!-- Provider dropdown -->
				<div>
					<label for="modal-provider" class="label-micro block mb-1.5">API key provider</label>
					<div class="relative">
						<select
							id="modal-provider"
							bind:value={selectedProviderId}
							onchange={() => {
								const prov = PROVIDERS.find(p => p.id === selectedProviderId);
								if (prov) {
									if (!formName.trim() || PROVIDERS.some(p => p.name === formName)) formName = prov.name;
									formBaseUrl = prov.baseUrl;
									formDefaultModel = prov.defaultModel;
								}
							}}
							class="control-select appearance-none cursor-pointer"
						>
							<option value="" disabled>Select a provider...</option>
							{#each visibleProviders as prov}
								<option value={prov.id}>{prov.name}</option>
							{/each}
						</select>
						<div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-text-muted">
							<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="M8 9l4-4 4 4m0 6l-4 4-4-4" />
							</svg>
						</div>
					</div>

					<!-- Provider icon preview -->
					{#if selectedProvider}
						<div class="flex items-center gap-2 mt-2 px-1">
							<div class="w-5 h-5 flex items-center justify-center" style="color: {selectedProvider.color}">
								{@html selectedProvider.icon}
							</div>
							<span class="text-[12px] text-text-secondary">{selectedProvider.name}</span>
							{#if selectedProvider.defaultModel}
								<span class="text-[11px] font-mono text-text-muted bg-bg-tertiary px-1.5 py-0.5 rounded">{selectedProvider.defaultModel}</span>
							{/if}
						</div>
					{/if}
				</div>

				<!-- API Key -->
				{#if selectedProvider}
					{#if selectedProvider.needsKey}
						<div>
							<label for="modal-key" class="label-micro block mb-1.5">API Key</label>
							<input id="modal-key" type="password" bind:value={formApiKey} placeholder={selectedProvider.keyPlaceholder}
								class="control-input font-mono" />
						</div>
					{/if}

					<!-- Default model -->
					<div>
						<label for="modal-model" class="label-micro block mb-1.5">Default Model</label>
						{#if getModelsForProvider(selectedProvider.id).length > 0}
							{@const models = getModelsForProvider(selectedProvider.id)}
							<div class="relative">
								<select
									id="modal-model"
									bind:value={formDefaultModel}
									class="control-select appearance-none cursor-pointer font-mono"
								>
									{#each models as model}
										<option value={model}>{model}</option>
									{/each}
								</select>
								<div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-text-muted">
									<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
										<path stroke-linecap="round" stroke-linejoin="round" d="M8 9l4-4 4 4m0 6l-4 4-4-4" />
									</svg>
								</div>
							</div>
						{:else}
							<input id="modal-model" type="text" bind:value={formDefaultModel} placeholder="e.g. gpt-4o"
								class="control-input font-mono" />
						{/if}
					</div>

					<!-- Base URL (only for custom/azure or if changed) -->
					{#if selectedProvider.id === 'custom' || selectedProvider.id === 'azure' || !selectedProvider.baseUrl}
						<div>
							<label for="modal-url" class="label-micro block mb-1.5">Base URL</label>
							<input id="modal-url" type="text" bind:value={formBaseUrl} placeholder={selectedProvider.baseUrl || 'https://api.example.com/v1'}
								class="control-input font-mono text-[12px]" />
						</div>
					{/if}
				{/if}
			<!-- Footer -->
			<div class="pt-1 border-t border-border flex items-center justify-end gap-3">
				<button onclick={closeModal} class="btn-ghost">Cancel</button>
				<button
					onclick={handleCreate}
					disabled={creating || !selectedProvider || !formName.trim() || (selectedProvider?.needsKey && !formApiKey.trim())}
					class="btn-primary px-5 disabled:opacity-40"
				>
					{creating ? 'Saving...' : 'Save'}
				</button>
			</div>
		</div>
	</FloatingInspector>
{/if}
