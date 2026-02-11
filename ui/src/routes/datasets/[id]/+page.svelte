<script lang="ts">
	import { page } from '$app/state';
	import {
		getDataset,
		getDatapoints,
		getQueue,
		createDatapoint,
		deleteDatapoint,
		enqueueDatapoints,
		claimQueueItem,
		submitQueueItem,
		importFile,
		subscribeEvents,
		shortId,
		type DatasetWithCount,
		type Datapoint,
		type DatapointKind,
		type QueueItem,
		type QueueList
	} from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { onMount } from 'svelte';

	const datasetId = $derived(page.params.id ?? '');

	let dataset: DatasetWithCount | null = $state(null);
	let datapoints: Datapoint[] = $state([]);
	let queue: QueueList = $state({ items: [], counts: { pending: 0, claimed: 0, completed: 0 } });
	let loading = $state(true);
	let activeTab: 'datapoints' | 'import' | 'queue' = $state('datapoints');

	// ── Datapoint form state ───────────────────────────────────────────
	let showAddForm = $state(false);
	let addKindJson = $state('{\n  "type": "generic",\n  "input": "",\n  "metadata": {}\n}');
	let addError = $state('');
	let adding = $state(false);

	// ── Import state ───────────────────────────────────────────────────
	let importResult: { imported: number } | null = $state(null);
	let importing = $state(false);
	let importError = $state('');
	let dragover = $state(false);

	// ── Queue state ────────────────────────────────────────────────────
	let claimName = $state('reviewer');
	let editingItem: QueueItem | null = $state(null);
	let editedJson = $state('');

	// ── Bulk select ────────────────────────────────────────────────────
	let selected: Set<string> = $state(new Set());

	function toggleSelect(id: string) {
		const next = new Set(selected);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selected = next;
	}

	function toggleSelectAll() {
		if (selected.size === datapoints.length) {
			selected = new Set();
		} else {
			selected = new Set(datapoints.map((d) => d.id));
		}
	}

	// ── Load ───────────────────────────────────────────────────────────
	async function load() {
		try {
			const [ds, dp, q] = await Promise.all([
				getDataset(datasetId),
				getDatapoints(datasetId),
				getQueue(datasetId)
			]);
			dataset = ds;
			datapoints = dp.datapoints;
			queue = q;
		} catch {
			// not found
		}
		loading = false;
	}

	onMount(() => {
		load();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'datapoint_created' && event.datapoint.dataset_id === datasetId) {
				datapoints = [...datapoints, event.datapoint];
				if (dataset) dataset = { ...dataset, datapoint_count: dataset.datapoint_count + 1 };
			} else if (event.type === 'queue_item_updated' && event.item.dataset_id === datasetId) {
				queue = {
					...queue,
					items: queue.items.some((i) => i.id === event.item.id)
						? queue.items.map((i) => (i.id === event.item.id ? event.item : i))
						: [...queue.items, event.item]
				};
				// Refresh counts
				getQueue(datasetId).then((q) => (queue = q)).catch(() => {});
			} else if (event.type === 'dataset_deleted' && event.dataset_id === datasetId) {
				dataset = null;
				datapoints = [];
			}
		});

		return unsub;
	});

	// ── Datapoint actions ──────────────────────────────────────────────
	async function handleAddDatapoint() {
		addError = '';
		adding = true;
		try {
			const kind: DatapointKind = JSON.parse(addKindJson);
			const dp = await createDatapoint(datasetId, kind);
			datapoints = [...datapoints, dp];
			showAddForm = false;
			addKindJson = '{\n  "type": "generic",\n  "input": "",\n  "metadata": {}\n}';
		} catch (e) {
			addError = e instanceof Error ? e.message : 'Invalid JSON';
		}
		adding = false;
	}

	async function handleDeleteDatapoint(dpId: string) {
		try {
			await deleteDatapoint(datasetId, dpId);
			datapoints = datapoints.filter((d) => d.id !== dpId);
			selected.delete(dpId);
			selected = new Set(selected);
		} catch {
			// error
		}
	}

	async function handleEnqueueSelected() {
		if (selected.size === 0) return;
		try {
			await enqueueDatapoints(datasetId, [...selected]);
			selected = new Set();
			const q = await getQueue(datasetId);
			queue = q;
			activeTab = 'queue';
		} catch {
			// error
		}
	}

	// ── Import actions ─────────────────────────────────────────────────
	async function handleImport(file: File) {
		importing = true;
		importError = '';
		importResult = null;
		try {
			importResult = await importFile(datasetId, file);
			// Reload datapoints
			const dp = await getDatapoints(datasetId);
			datapoints = dp.datapoints;
			const ds = await getDataset(datasetId);
			dataset = ds;
		} catch (e) {
			importError = e instanceof Error ? e.message : 'Import failed';
		}
		importing = false;
	}

	function onFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (file) handleImport(file);
		input.value = '';
	}

	function onDrop(e: DragEvent) {
		e.preventDefault();
		dragover = false;
		const file = e.dataTransfer?.files?.[0];
		if (file) handleImport(file);
	}

	// ── Queue actions ──────────────────────────────────────────────────
	async function handleClaim(itemId: string) {
		try {
			const updated = await claimQueueItem(itemId, claimName);
			queue = {
				...queue,
				items: queue.items.map((i) => (i.id === updated.id ? updated : i))
			};
			getQueue(datasetId).then((q) => (queue = q)).catch(() => {});
		} catch {
			// error
		}
	}

	function startEditing(item: QueueItem) {
		editingItem = item;
		editedJson = item.original_data ? JSON.stringify(item.original_data, null, 2) : '{}';
	}

	async function handleSubmit() {
		if (!editingItem) return;
		try {
			const data = editedJson.trim() ? JSON.parse(editedJson) : undefined;
			const updated = await submitQueueItem(editingItem.id, data);
			queue = {
				...queue,
				items: queue.items.map((i) => (i.id === updated.id ? updated : i))
			};
			editingItem = null;
			getQueue(datasetId).then((q) => (queue = q)).catch(() => {});
		} catch {
			// error
		}
	}

	// ── Helpers ────────────────────────────────────────────────────────
	function datapointPreview(dp: Datapoint): string {
		if (dp.kind.type === 'llm_conversation') {
			const msg = dp.kind.messages[0];
			return msg ? `${msg.role}: ${msg.content}` : '(empty conversation)';
		}
		const input = dp.kind.input;
		if (typeof input === 'string') return input;
		return JSON.stringify(input);
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function formatJson(value: unknown): string {
		if (value === null || value === undefined) return '(none)';
		if (typeof value === 'string') return value;
		return JSON.stringify(value, null, 2);
	}

	const queueItemsByStatus = $derived.by(() => {
		const pending = queue.items.filter((i) => i.status === 'pending');
		const claimed = queue.items.filter((i) => i.status === 'claimed');
		const completed = queue.items.filter((i) => i.status === 'completed');
		return { pending, claimed, completed };
	});
</script>

<div class="max-w-6xl mx-auto space-y-4">
	<!-- Header -->
	<div class="flex items-center gap-2">
		<a href="/datasets" class="text-text-secondary hover:text-text text-sm">&larr; Datasets</a>
		<span class="text-text-muted">/</span>
		{#if dataset}
			<h1 class="text-lg font-bold">{dataset.name}</h1>
			<span class="px-2 py-0.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded">
				{dataset.datapoint_count} datapoints
			</span>
			{#if dataset.description}
				<span class="text-text-muted text-sm">{dataset.description}</span>
			{/if}
		{:else if !loading}
			<h1 class="text-lg font-bold text-text-muted">Dataset not found</h1>
		{/if}
	</div>

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading...</div>
	{:else if !dataset}
		<div class="text-text-muted text-sm text-center py-8">Dataset not found</div>
	{:else}
		<!-- Tabs -->
		<div class="flex gap-0 border-b border-border">
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'datapoints' ? 'border-amber-400 text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => (activeTab = 'datapoints')}
			>Datapoints</button>
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'import' ? 'border-amber-400 text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => (activeTab = 'import')}
			>Import</button>
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'queue' ? 'border-amber-400 text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => (activeTab = 'queue')}
			>
				Queue
				{#if queue.counts.pending > 0}
					<span class="ml-1 px-1.5 py-0.5 text-xs bg-warning/20 text-warning rounded">{queue.counts.pending}</span>
				{/if}
			</button>
		</div>

		<!-- Tab: Datapoints -->
		{#if activeTab === 'datapoints'}
			<div class="space-y-3">
				<!-- Actions row -->
				<div class="flex items-center gap-2">
					<button
						class="px-3 py-1.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors"
						onclick={() => (showAddForm = !showAddForm)}
					>
						{showAddForm ? 'Cancel' : '+ Add Datapoint'}
					</button>
					{#if selected.size > 0}
						<button
							class="px-3 py-1.5 text-xs bg-accent/10 text-accent border border-accent/20 rounded hover:bg-accent/20 transition-colors"
							onclick={handleEnqueueSelected}
						>
							Enqueue {selected.size} selected
						</button>
					{/if}
				</div>

				<!-- Add form -->
				{#if showAddForm}
					<div class="bg-bg-secondary border border-border rounded p-4 space-y-3">
						<div>
							<label for="dp-kind" class="block text-xs text-text-muted uppercase mb-1">DatapointKind (JSON)</label>
							<textarea
								id="dp-kind"
								bind:value={addKindJson}
								rows={8}
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-xs text-text font-mono placeholder:text-text-muted"
							></textarea>
						</div>
						{#if addError}
							<div class="text-danger text-xs">{addError}</div>
						{/if}
						<button
							class="px-4 py-1.5 text-xs bg-amber-400 text-bg font-semibold rounded hover:bg-amber-300 transition-colors disabled:opacity-50"
							disabled={adding}
							onclick={handleAddDatapoint}
						>
							{adding ? 'Adding...' : 'Add'}
						</button>
					</div>
				{/if}

				<!-- Table header -->
				<div class="grid grid-cols-[32px_80px_1fr_90px_120px_80px] gap-3 px-3 text-xs text-text-muted uppercase items-center">
					<label class="flex items-center justify-center">
						<input
							type="checkbox"
							checked={selected.size === datapoints.length && datapoints.length > 0}
							onchange={toggleSelectAll}
							class="accent-amber-400"
						/>
					</label>
					<span>Kind</span>
					<span>Preview</span>
					<span>Source</span>
					<span>Created</span>
					<span class="text-right">Actions</span>
				</div>

				{#if datapoints.length === 0}
					<div class="text-text-muted text-sm text-center py-8">No datapoints yet</div>
				{:else}
					<div class="space-y-0">
						{#each datapoints as dp (dp.id)}
							<div class="grid grid-cols-[32px_80px_1fr_90px_120px_80px] gap-3 items-center px-3 py-2 text-sm border-b border-border/50 hover:bg-bg-secondary transition-colors">
								<label class="flex items-center justify-center">
									<input
										type="checkbox"
										checked={selected.has(dp.id)}
										onchange={() => toggleSelect(dp.id)}
										class="accent-amber-400"
									/>
								</label>
								<span class="text-xs px-1.5 py-0.5 rounded border
									{dp.kind.type === 'llm_conversation'
										? 'bg-purple-400/10 text-purple-400 border-purple-400/20'
										: 'bg-text-muted/10 text-text-secondary border-text-muted/20'}">
									{dp.kind.type === 'llm_conversation' ? 'LLM' : 'Generic'}
								</span>
								<span class="text-text-secondary text-xs truncate font-mono">{datapointPreview(dp).slice(0, 80)}</span>
								<span class="text-text-muted text-xs">{dp.source}</span>
								<span class="text-text-muted text-xs">{formatDate(dp.created_at)}</span>
								<div class="text-right flex items-center justify-end gap-2">
									<button
										class="text-text-muted hover:text-danger text-xs transition-colors"
										onclick={() => handleDeleteDatapoint(dp.id)}
									>delete</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		<!-- Tab: Import -->
		{:else if activeTab === 'import'}
			<div class="space-y-4">
				<!-- Drop zone -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="border-2 border-dashed rounded-lg p-12 text-center transition-colors
						{dragover ? 'border-amber-400 bg-amber-400/5' : 'border-border hover:border-text-muted'}"
					ondragover={(e) => { e.preventDefault(); dragover = true; }}
					ondragleave={() => (dragover = false)}
					ondrop={onDrop}
				>
					<div class="text-text-muted mb-3">
						<svg class="w-10 h-10 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" />
						</svg>
						<p class="text-sm">Drag and drop a file here, or click to select</p>
						<p class="text-xs mt-1">Accepts .json, .jsonl, .csv</p>
					</div>
					<label class="inline-block px-4 py-1.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors cursor-pointer">
						Choose File
						<input type="file" accept=".json,.jsonl,.csv" class="hidden" onchange={onFileSelect} />
					</label>
				</div>

				{#if importing}
					<div class="text-text-muted text-sm text-center">Importing...</div>
				{/if}

				{#if importResult}
					<div class="bg-success/10 border border-success/20 rounded p-3 text-success text-sm">
						Imported {importResult.imported} datapoints
					</div>
				{/if}

				{#if importError}
					<div class="bg-danger/10 border border-danger/20 rounded p-3 text-danger text-sm">
						{importError}
					</div>
				{/if}

				<!-- Format hints -->
				<div class="bg-bg-secondary border border-border rounded p-4 space-y-3 text-xs text-text-secondary">
					<div class="text-text-muted uppercase text-xs font-semibold">Supported Formats</div>
					<div>
						<span class="text-text font-medium">JSON</span> — Array of DatapointKind objects:
						<pre class="mt-1 bg-bg-tertiary rounded p-2 text-text-muted font-mono overflow-x-auto">[{`{"type":"generic","input":"hello","metadata":{}}`}]</pre>
					</div>
					<div>
						<span class="text-text font-medium">JSONL</span> — One DatapointKind object per line
					</div>
					<div>
						<span class="text-text font-medium">CSV</span> — Columns mapped to generic datapoint fields (input, expected_output, etc.)
					</div>
				</div>
			</div>

		<!-- Tab: Annotation Queue -->
		{:else if activeTab === 'queue'}
			<div class="space-y-4">
				<!-- Counts -->
				<div class="flex items-center gap-3 text-sm">
					<span class="px-2 py-0.5 rounded text-xs border bg-warning/20 text-warning border-warning/30">
						{queue.counts.pending} pending
					</span>
					<span class="px-2 py-0.5 rounded text-xs border bg-accent/20 text-accent border-accent/30">
						{queue.counts.claimed} claimed
					</span>
					<span class="px-2 py-0.5 rounded text-xs border bg-success/20 text-success border-success/30">
						{queue.counts.completed} completed
					</span>
					<div class="flex-1"></div>
					<label for="claim-name" class="text-xs text-text-muted">Claim as:</label>
					<input
						id="claim-name"
						type="text"
						bind:value={claimName}
						class="bg-bg-tertiary border border-border rounded px-2 py-1 text-xs text-text w-28"
					/>
				</div>

				<!-- Editing overlay -->
				{#if editingItem}
					<div class="bg-bg-secondary border border-accent/30 rounded p-4 space-y-3">
						<div class="flex items-center justify-between">
							<span class="text-sm font-semibold text-text">Editing: {shortId(editingItem.id)}</span>
							<button class="text-text-muted hover:text-text text-xs" onclick={() => (editingItem = null)}>cancel</button>
						</div>
						<div class="grid grid-cols-2 gap-4">
							<div>
								<div class="text-xs text-text-muted uppercase mb-1">Original</div>
								<pre class="text-xs bg-bg-tertiary rounded p-3 overflow-auto max-h-48 font-mono text-text-secondary whitespace-pre-wrap">{formatJson(editingItem.original_data)}</pre>
							</div>
							<div>
								<div class="text-xs text-text-muted uppercase mb-1">Edited</div>
								<textarea
									bind:value={editedJson}
									rows={8}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-xs text-text font-mono"
								></textarea>
							</div>
						</div>
						<button
							class="px-4 py-1.5 text-xs bg-success text-bg font-semibold rounded hover:bg-success/80 transition-colors"
							onclick={handleSubmit}
						>Submit</button>
					</div>
				{/if}

				<!-- Pending -->
				{#if queueItemsByStatus.pending.length > 0}
					<div class="space-y-1">
						<div class="text-xs text-text-muted uppercase">Pending ({queueItemsByStatus.pending.length})</div>
						{#each queueItemsByStatus.pending as item (item.id)}
							<div class="flex items-center gap-3 px-3 py-2 bg-bg-secondary border border-border/50 rounded text-sm">
								<StatusBadge status="running" />
								<span class="text-text-secondary text-xs font-mono flex-1 truncate">{shortId(item.datapoint_id)}</span>
								<span class="text-text-muted text-xs">{formatDate(item.created_at)}</span>
								<button
									class="px-2 py-1 text-xs bg-accent/10 text-accent border border-accent/20 rounded hover:bg-accent/20 transition-colors"
									onclick={() => handleClaim(item.id)}
								>Claim</button>
							</div>
						{/each}
					</div>
				{/if}

				<!-- Claimed -->
				{#if queueItemsByStatus.claimed.length > 0}
					<div class="space-y-1">
						<div class="text-xs text-text-muted uppercase">Claimed ({queueItemsByStatus.claimed.length})</div>
						{#each queueItemsByStatus.claimed as item (item.id)}
							<div class="flex items-center gap-3 px-3 py-2 bg-bg-secondary border border-border/50 rounded text-sm">
								<span class="px-2 py-0.5 rounded text-xs border bg-accent/20 text-accent border-accent/30">claimed</span>
								<span class="text-text-secondary text-xs font-mono flex-1 truncate">{shortId(item.datapoint_id)}</span>
								{#if item.claimed_by}
									<span class="text-text-muted text-xs">by {item.claimed_by}</span>
								{/if}
								<button
									class="px-2 py-1 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors"
									onclick={() => startEditing(item)}
								>Edit & Submit</button>
							</div>
						{/each}
					</div>
				{/if}

				<!-- Completed -->
				{#if queueItemsByStatus.completed.length > 0}
					<div class="space-y-1">
						<div class="text-xs text-text-muted uppercase">Completed ({queueItemsByStatus.completed.length})</div>
						{#each queueItemsByStatus.completed as item (item.id)}
							<div class="flex items-center gap-3 px-3 py-2 bg-bg-secondary border border-border/50 rounded text-sm">
								<StatusBadge status="completed" />
								<span class="text-text-secondary text-xs font-mono truncate">{shortId(item.datapoint_id)}</span>
								{#if item.claimed_by}
									<span class="text-text-muted text-xs">by {item.claimed_by}</span>
								{/if}
								<div class="flex-1"></div>
								{#if item.edited_data}
									<details class="text-xs">
										<summary class="text-accent cursor-pointer">view edits</summary>
										<div class="grid grid-cols-2 gap-2 mt-2">
											<div>
												<div class="text-text-muted uppercase mb-0.5">Original</div>
												<pre class="bg-bg-tertiary rounded p-2 overflow-auto max-h-32 font-mono text-text-secondary whitespace-pre-wrap">{formatJson(item.original_data)}</pre>
											</div>
											<div>
												<div class="text-text-muted uppercase mb-0.5">Edited</div>
												<pre class="bg-bg-tertiary rounded p-2 overflow-auto max-h-32 font-mono text-text whitespace-pre-wrap">{formatJson(item.edited_data)}</pre>
											</div>
										</div>
									</details>
								{/if}
							</div>
						{/each}
					</div>
				{/if}

				{#if queue.items.length === 0}
					<div class="text-text-muted text-sm text-center py-8">
						Queue is empty. Select datapoints and click "Enqueue" to start annotation.
					</div>
				{/if}
			</div>
		{/if}
	{/if}
</div>
