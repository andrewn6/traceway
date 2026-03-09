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
	let datasetSearch = $state('');
	let filterMenuOpen = $state(false);
	let columnsMenuOpen = $state(false);
	let filterMode: 'all' | 'nonempty' | 'empty' = $state('all');
	let showIdCol = $state(true);
	let showCreatedCol = $state(true);
	let showCountCol = $state(true);

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
				datasets = [{ ...event.dataset, datapoint_count: 0 }, ...datasets];
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
			datasets = [{ ...ds, datapoint_count: 0 }, ...datasets];
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

	const visibleDatasets = $derived.by(() => {
		const q = datasetSearch.trim().toLowerCase();
		return datasets.filter((ds) => {
			if (filterMode === 'nonempty' && ds.datapoint_count === 0) return false;
			if (filterMode === 'empty' && ds.datapoint_count > 0) return false;
			if (!q) return true;
			return (
				ds.name.toLowerCase().includes(q) ||
				(ds.description ?? '').toLowerCase().includes(q) ||
				ds.id.toLowerCase().includes(q)
			);
		});
	});
</script>

<div class="app-shell-wide space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-semibold tracking-tight">Datasets</h1>
		<button class="btn-primary" onclick={() => (showForm = !showForm)}>{showForm ? 'Cancel' : '+ New Dataset'}</button>
	</div>

	<!-- Inline create form -->
	{#if showForm}
		<form class="table-float p-4 space-y-3" onsubmit={(e) => { e.preventDefault(); handleCreate(); }}>
			<div>
				<label for="ds-name" class="label-micro block uppercase mb-1">Name</label>
				<input
					id="ds-name"
					type="text"
					bind:value={newName}
					placeholder="e.g. eval-gpt4-coding"
					class="control-input"
				/>
			</div>
			<div>
				<label for="ds-desc" class="label-micro block uppercase mb-1">Description (optional)</label>
				<input
					id="ds-desc"
					type="text"
					bind:value={newDescription}
					placeholder="What is this dataset for?"
					class="control-input"
				/>
			</div>
			<button type="submit" disabled={creating || !newName.trim()} class="btn-primary disabled:opacity-50">
				{creating ? 'Creating...' : 'Create Dataset'}
			</button>
		</form>
	{/if}

	<div class="table-float p-2 space-y-2">
		<div class="flex items-center gap-1.5 flex-wrap">
			<div class="relative">
				<button class="btn-secondary h-7 text-[12px]" onclick={() => { filterMenuOpen = !filterMenuOpen; columnsMenuOpen = false; }}>
					+ Add filter
				</button>
				{#if filterMenuOpen}
					<div class="absolute left-0 top-full mt-1 z-20 w-44 surface-panel p-1.5">
						<button class="w-full text-left px-2 py-1.5 rounded-md hover:bg-bg-tertiary/60 text-[12px] {filterMode === 'all' ? 'text-text' : 'text-text-secondary'}" onclick={() => { filterMode = 'all'; filterMenuOpen = false; }}>All datasets</button>
						<button class="w-full text-left px-2 py-1.5 rounded-md hover:bg-bg-tertiary/60 text-[12px] {filterMode === 'nonempty' ? 'text-text' : 'text-text-secondary'}" onclick={() => { filterMode = 'nonempty'; filterMenuOpen = false; }}>Has datapoints</button>
						<button class="w-full text-left px-2 py-1.5 rounded-md hover:bg-bg-tertiary/60 text-[12px] {filterMode === 'empty' ? 'text-text' : 'text-text-secondary'}" onclick={() => { filterMode = 'empty'; filterMenuOpen = false; }}>Empty datasets</button>
					</div>
				{/if}
			</div>

			<div class="relative">
				<button class="btn-secondary h-7 text-[12px]" onclick={() => { columnsMenuOpen = !columnsMenuOpen; filterMenuOpen = false; }}>Columns</button>
				{#if columnsMenuOpen}
					<div class="absolute left-0 top-full mt-1 z-20 w-44 surface-panel p-2 space-y-1.5 text-[12px]">
						<label class="flex items-center gap-2"><input type="checkbox" bind:checked={showIdCol} class="accent-accent" /> ID</label>
						<label class="flex items-center gap-2"><input type="checkbox" bind:checked={showCountCol} class="accent-accent" /> Datapoints</label>
						<label class="flex items-center gap-2"><input type="checkbox" bind:checked={showCreatedCol} class="accent-accent" /> Created</label>
					</div>
				{/if}
			</div>

			<div class="command-input-shell min-w-[320px]">
				<div class="pl-2.5 pr-1.5 text-text-muted/80">
					<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
					</svg>
				</div>
				<input bind:value={datasetSearch} type="text" placeholder="Search by dataset name..." class="command-input" />
			</div>
		</div>

		{#if loading}
			<div class="text-text-muted text-sm text-center py-10">Loading...</div>
		{:else if visibleDatasets.length === 0}
			<div class="text-text-muted text-sm text-center py-10">No datasets match your current filters</div>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-[13px]">
					<thead>
						<tr class="table-head-compact border-b border-border/55">
							<th class="px-2.5 py-2.5 w-8 text-left">
								<input type="checkbox" class="accent-accent" />
							</th>
							{#if showIdCol}
								<th class="px-2 py-2.5 text-left">ID</th>
							{/if}
							<th class="px-2 py-2.5 text-left">Name</th>
							{#if showCountCol}
								<th class="px-2 py-2.5 text-left">Datapoints Count</th>
							{/if}
							{#if showCreatedCol}
								<th class="px-2 py-2.5 text-left">Created</th>
							{/if}
							<th class="px-2 py-2.5 text-right">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each visibleDatasets as ds (ds.id)}
							<tr class="border-b border-border/35 hover:bg-bg-secondary/40 transition-colors">
								<td class="px-2.5 py-2.5">
									<input type="checkbox" class="accent-accent" />
								</td>
								{#if showIdCol}
									<td class="px-2 py-2.5">
										<a href="/datasets/{ds.id}" class="font-mono text-[12px] text-text-secondary hover:text-text">{shortId(ds.id)}</a>
									</td>
								{/if}
								<td class="px-2 py-2.5">
									<a href="/datasets/{ds.id}" class="text-text font-medium hover:text-accent">{ds.name}</a>
									{#if ds.description}
										<div class="text-[12px] text-text-muted truncate mt-0.5">{ds.description}</div>
									{/if}
								</td>
								{#if showCountCol}
									<td class="px-2 py-2.5 text-text-secondary">{ds.datapoint_count}</td>
								{/if}
								{#if showCreatedCol}
									<td class="px-2 py-2.5 text-text-secondary">{formatDate(ds.created_at)}</td>
								{/if}
								<td class="px-2 py-2.5 text-right">
									<button class="btn-ghost h-7 text-[12px]" onclick={(e) => handleDelete(e, ds.id)}>Delete</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</div>
