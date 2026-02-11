<script lang="ts">
	import {
		getDatasets,
		createDataset,
		deleteDataset,
		subscribeEvents,
		shortId,
		type DatasetWithCount
	} from '$lib/api';
	import { onMount } from 'svelte';

	let datasets: DatasetWithCount[] = $state([]);
	let loading = $state(true);

	// New dataset form
	let showForm = $state(false);
	let newName = $state('');
	let newDescription = $state('');
	let creating = $state(false);

	async function load() {
		try {
			const result = await getDatasets();
			datasets = result.datasets;
		} catch {
			// API not available
		}
		loading = false;
	}

	onMount(() => {
		load();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'dataset_created') {
				datasets = [event.dataset, ...datasets];
			} else if (event.type === 'dataset_deleted') {
				datasets = datasets.filter((d) => d.id !== event.dataset_id);
			} else if (event.type === 'cleared') {
				datasets = [];
			}
		});

		return unsub;
	});

	async function handleCreate() {
		if (!newName.trim()) return;
		creating = true;
		try {
			const ds = await createDataset(newName.trim(), newDescription.trim() || undefined);
			datasets = [ds, ...datasets];
			newName = '';
			newDescription = '';
			showForm = false;
		} catch {
			// error
		}
		creating = false;
	}

	async function handleDelete(e: Event, id: string) {
		e.stopPropagation();
		try {
			await deleteDataset(id);
			datasets = datasets.filter((d) => d.id !== id);
		} catch {
			// error
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="max-w-6xl mx-auto space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold">Datasets</h1>
		<button
			class="px-3 py-1.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors"
			onclick={() => (showForm = !showForm)}
		>
			{showForm ? 'Cancel' : '+ New Dataset'}
		</button>
	</div>

	<!-- Inline create form -->
	{#if showForm}
		<form
			class="bg-bg-secondary border border-border rounded p-4 space-y-3"
			onsubmit={(e) => { e.preventDefault(); handleCreate(); }}
		>
			<div>
				<label for="ds-name" class="block text-xs text-text-muted uppercase mb-1">Name</label>
				<input
					id="ds-name"
					type="text"
					bind:value={newName}
					placeholder="e.g. eval-gpt4-coding"
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted"
				/>
			</div>
			<div>
				<label for="ds-desc" class="block text-xs text-text-muted uppercase mb-1">Description (optional)</label>
				<input
					id="ds-desc"
					type="text"
					bind:value={newDescription}
					placeholder="What is this dataset for?"
					class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted"
				/>
			</div>
			<button
				type="submit"
				disabled={creating || !newName.trim()}
				class="px-4 py-1.5 text-xs bg-amber-400 text-bg font-semibold rounded hover:bg-amber-300 transition-colors disabled:opacity-50"
			>
				{creating ? 'Creating...' : 'Create Dataset'}
			</button>
		</form>
	{/if}

	<!-- Table header -->
	<div class="grid grid-cols-[1fr_100px_140px_80px] gap-4 px-3 text-xs text-text-muted uppercase">
		<span>Name</span>
		<span class="text-center">Datapoints</span>
		<span>Created</span>
		<span class="text-right">Actions</span>
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading...</div>
	{:else if datasets.length === 0}
		<div class="text-text-muted text-sm text-center py-8">No datasets yet</div>
	{:else}
		<div class="space-y-0">
			{#each datasets as ds (ds.id)}
				<a
					href="/datasets/{ds.id}"
					class="grid grid-cols-[1fr_100px_140px_80px] gap-4 items-center px-3 py-2.5 text-sm hover:bg-bg-secondary rounded transition-colors border-b border-border/50"
				>
					<div>
						<div class="text-text font-medium">{ds.name}</div>
						{#if ds.description}
							<div class="text-text-muted text-xs truncate">{ds.description}</div>
						{/if}
						<div class="text-text-muted text-xs font-mono">{shortId(ds.id)}</div>
					</div>
					<div class="text-center">
						<span class="px-2 py-0.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded">
							{ds.datapoint_count}
						</span>
					</div>
					<div class="text-text-secondary text-xs">{formatDate(ds.created_at)}</div>
					<div class="text-right">
						<button
							class="text-text-muted hover:text-danger text-xs transition-colors"
							onclick={(e) => handleDelete(e, ds.id)}
						>delete</button>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</div>
