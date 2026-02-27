<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
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
		updateDataset,
		subscribeEvents,
		shortId,
		listEvalRuns,
		createEvalRun,
		deleteEvalRun,
		cancelEvalRun,
		listCaptureRules,
		createCaptureRule,
		deleteCaptureRule,
		toggleCaptureRule,
		listProviderConnections,
		listProviderModels,
		type DatasetWithCount,
		type Datapoint,
		type DatapointKind,
		type QueueItem,
		type EvalRun,
		type CaptureRule,
		type EvalConfig,
		type ScoringStrategy,
		type CaptureFilters,
		type ProviderConnectionInfo,
		type ProviderModelInfo
	} from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import EvalScoreBadge from '$lib/components/EvalScoreBadge.svelte';
	import EvalProgressBar from '$lib/components/EvalProgressBar.svelte';
	import { onMount } from 'svelte';

	const datasetId = $derived(page.params.id ?? '');

	let dataset: DatasetWithCount | null = $state(null);
	let datapoints: Datapoint[] = $state([]);
	let queueItems: QueueItem[] = $state([]);
	let evalRuns: EvalRun[] = $state([]);
	let captureRules: CaptureRule[] = $state([]);
	let loading = $state(true);
	let activeTab: 'datapoints' | 'import' | 'queue' | 'evals' | 'rules' = $state('datapoints');

	// ── Edit dataset state ────────────────────────────────────────────
	let showEditDataset = $state(false);
	let editName = $state('');
	let editDescription = $state('');
	let editSaving = $state(false);

	function startEditDataset() {
		if (!dataset) return;
		editName = dataset.name;
		editDescription = dataset.description ?? '';
		showEditDataset = true;
	}

	async function handleSaveDataset() {
		if (!dataset || !editName.trim()) return;
		editSaving = true;
		try {
			const updated = await updateDataset(datasetId, {
				name: editName.trim(),
				description: editDescription.trim() || undefined
			});
			dataset = updated;
			showEditDataset = false;
		} catch {
			// error
		}
		editSaving = false;
	}

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

	// ── Eval Run state ────────────────────────────────────────────────
	let showNewEvalForm = $state(false);
	let evalFormModel = $state('');
	let evalFormSystemPrompt = $state('');
	let evalFormTemp = $state('');
	let evalFormMaxTokens = $state('');
	let evalFormScoring: ScoringStrategy = $state('none');
	let evalFormName = $state('');
	let evalFormConnectionId = $state('');
	let evalCreating = $state(false);
	let providerConnections: ProviderConnectionInfo[] = $state([]);
	let connectionModels: ProviderModelInfo[] = $state([]);
	let loadingModels = $state(false);
	let evalCompareMode = $state(false);
	let evalCompareSelected: Set<string> = $state(new Set());
	let evalDeleteConfirm: string | null = $state(null);

	async function loadModelsForConnection(connId: string) {
		if (!connId) { connectionModels = []; return; }
		loadingModels = true;
		try {
			const resp = await listProviderModels(connId);
			if (resp.ok) connectionModels = resp.models;
			else connectionModels = [];
		} catch {
			connectionModels = [];
		}
		loadingModels = false;
	}

	const runningEvalCount = $derived(evalRuns.filter((r) => r.status === 'running').length);
	const completedEvalRuns = $derived(evalRuns.filter((r) => r.status === 'completed'));
	const avgScore = $derived.by(() => {
		const scores = completedEvalRuns.filter((r) => r.results.scores.mean != null).map((r) => r.results.scores.mean!);
		if (scores.length === 0) return null;
		return scores.reduce((a, b) => a + b, 0) / scores.length;
	});
	const bestRun = $derived.by(() => {
		if (completedEvalRuns.length === 0) return null;
		return completedEvalRuns.reduce((best, r) =>
			(r.results.scores.mean ?? 0) > (best.results.scores.mean ?? 0) ? r : best
		);
	});

	async function handleCreateEvalRun() {
		if (!evalFormModel.trim() || !evalFormConnectionId) return;
		evalCreating = true;
		try {
			const conn = providerConnections.find(c => c.id === evalFormConnectionId);
			const config: EvalConfig = {
				model: evalFormModel.trim(),
				provider: conn?.provider || undefined,
				system_prompt: evalFormSystemPrompt.trim() || undefined,
				temperature: evalFormTemp ? parseFloat(evalFormTemp) : undefined,
				max_tokens: evalFormMaxTokens ? parseInt(evalFormMaxTokens) : undefined,
				provider_connection_id: evalFormConnectionId
			};
			await createEvalRun(datasetId, {
				name: evalFormName.trim() || undefined,
				config,
				scoring: evalFormScoring
			});
			showNewEvalForm = false;
			evalFormModel = '';
			evalFormName = '';
			evalFormSystemPrompt = '';
			evalFormTemp = '';
			evalFormMaxTokens = '';
			evalFormScoring = 'none';
			evalFormConnectionId = '';
			connectionModels = [];
			const resp = await listEvalRuns(datasetId);
			evalRuns = resp.items;
		} catch {
			// error
		}
		evalCreating = false;
	}

	async function handleDeleteEvalRun(runId: string) {
		if (evalDeleteConfirm !== runId) {
			evalDeleteConfirm = runId;
			return;
		}
		try {
			await deleteEvalRun(runId);
			evalRuns = evalRuns.filter((r) => r.id !== runId);
		} catch {
			// error
		}
		evalDeleteConfirm = null;
	}

	async function handleCancelEvalRun(runId: string) {
		try {
			await cancelEvalRun(runId);
			evalRuns = evalRuns.map((r) => r.id === runId ? { ...r, status: 'cancelled' as const } : r);
		} catch {
			// error
		}
	}

	function toggleCompareSelect(runId: string) {
		const next = new Set(evalCompareSelected);
		if (next.has(runId)) next.delete(runId);
		else next.add(runId);
		evalCompareSelected = next;
	}

	function goCompare() {
		if (evalCompareSelected.size < 2) return;
		goto(`/datasets/${datasetId}/compare?runs=${[...evalCompareSelected].join(',')}`);
	}

	// ── Capture Rule state ────────────────────────────────────────────
	let showNewRuleForm = $state(false);
	let ruleFormName = $state('');
	let ruleFormSpanKind = $state('');
	let ruleFormModel = $state('');
	let ruleFormProvider = $state('');
	let ruleFormStatus = $state('');
	let ruleFormNameContains = $state('');
	let ruleFormMinLatency = $state('');
	let ruleFormMinTokens = $state('');
	let ruleFormSampleRate = $state(1.0);
	let ruleCreating = $state(false);

	const enabledRuleCount = $derived(captureRules.filter((r) => r.enabled).length);

	async function handleCreateRule() {
		if (!ruleFormName.trim()) return;
		ruleCreating = true;
		try {
			const filters: CaptureFilters = {};
			if (ruleFormSpanKind) filters.span_kind = ruleFormSpanKind;
			if (ruleFormModel) filters.model = ruleFormModel;
			if (ruleFormProvider) filters.provider = ruleFormProvider;
			if (ruleFormStatus) filters.status = ruleFormStatus;
			if (ruleFormNameContains) filters.name_contains = ruleFormNameContains;
			if (ruleFormMinLatency) filters.min_latency_ms = parseInt(ruleFormMinLatency);
			if (ruleFormMinTokens) filters.min_tokens = parseInt(ruleFormMinTokens);
			await createCaptureRule(datasetId, {
				name: ruleFormName.trim(),
				filters,
				sample_rate: ruleFormSampleRate
			});
			showNewRuleForm = false;
			ruleFormName = '';
			ruleFormSpanKind = '';
			ruleFormModel = '';
			ruleFormProvider = '';
			ruleFormStatus = '';
			ruleFormNameContains = '';
			ruleFormMinLatency = '';
			ruleFormMinTokens = '';
			ruleFormSampleRate = 1.0;
			const resp = await listCaptureRules(datasetId);
			captureRules = resp.items;
		} catch {
			// error
		}
		ruleCreating = false;
	}

	async function handleToggleRule(ruleId: string) {
		try {
			const updated = await toggleCaptureRule(ruleId);
			captureRules = captureRules.map((r) => r.id === updated.id ? updated : r);
		} catch {
			// error
		}
	}

	async function handleDeleteRule(ruleId: string) {
		try {
			await deleteCaptureRule(ruleId);
			captureRules = captureRules.filter((r) => r.id !== ruleId);
		} catch {
			// error
		}
	}

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
		const [ds, dp, q, er, cr, pc] = await Promise.all([
			getDataset(datasetId),
			getDatapoints(datasetId),
			getQueue(datasetId),
			listEvalRuns(datasetId).catch(() => ({ items: [] as EvalRun[], total: null, next_cursor: null, has_more: false })),
			listCaptureRules(datasetId).catch(() => ({ items: [] as CaptureRule[], total: null, next_cursor: null, has_more: false })),
			listProviderConnections().catch(() => ({ connections: [] as ProviderConnectionInfo[], count: 0 }))
		]);
		dataset = ds;
		datapoints = dp.items;
		queueItems = q.items;
		evalRuns = er.items;
		captureRules = cr.items;
		providerConnections = pc.connections;
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
			queueItems = queueItems.some((i) => i.id === event.item.id)
				? queueItems.map((i) => (i.id === event.item.id ? event.item : i))
				: [...queueItems, event.item];
			getQueue(datasetId).then((q) => (queueItems = q.items)).catch(() => {});
			} else if (event.type === 'dataset_deleted' && event.dataset_id === datasetId) {
				dataset = null;
				datapoints = [];
			} else if (event.type === 'eval_run_created' && event.run.dataset_id === datasetId) {
				evalRuns = [event.run, ...evalRuns];
			} else if (event.type === 'eval_run_updated' && event.run.dataset_id === datasetId) {
				evalRuns = evalRuns.map((r) => r.id === event.run.id ? event.run : r);
			} else if (event.type === 'eval_run_completed' && event.run.dataset_id === datasetId) {
				evalRuns = evalRuns.map((r) => r.id === event.run.id ? event.run : r);
			} else if (event.type === 'capture_rule_fired' && event.datapoint.dataset_id === datasetId) {
				// Refresh rules to update captured_count
				listCaptureRules(datasetId).then((resp) => (captureRules = resp.items)).catch(() => {});
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
			queueItems = q.items;
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
		datapoints = dp.items;
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
			queueItems = queueItems.map((i) => (i.id === updated.id ? updated : i));
			getQueue(datasetId).then((q) => (queueItems = q.items)).catch(() => {});
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
			queueItems = queueItems.map((i) => (i.id === updated.id ? updated : i));
			editingItem = null;
			getQueue(datasetId).then((q) => (queueItems = q.items)).catch(() => {});
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

	function exportDatasetJson() {
		if (datapoints.length === 0) return;
		const exportData = datapoints.map(dp => dp.kind);
		const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `${dataset?.name ?? 'dataset'}-${datapoints.length}-datapoints.json`;
		a.click();
		URL.revokeObjectURL(url);
	}

	const queueItemsByStatus = $derived.by(() => {
		const pending = queueItems.filter((i) => i.status === 'pending');
		const claimed = queueItems.filter((i) => i.status === 'claimed');
		const completed = queueItems.filter((i) => i.status === 'completed');
		return { pending, claimed, completed };
	});

	function relativeTime(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 1) return 'just now';
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}

	function ruleFilterTags(rule: CaptureRule): string[] {
		const tags: string[] = [];
		if (rule.filters.span_kind) tags.push(`kind:${rule.filters.span_kind}`);
		if (rule.filters.model) tags.push(`model:${rule.filters.model}`);
		if (rule.filters.provider) tags.push(`provider:${rule.filters.provider}`);
		if (rule.filters.status) tags.push(`status:${rule.filters.status}`);
		if (rule.filters.name_contains) tags.push(`name:*${rule.filters.name_contains}*`);
		if (rule.filters.min_latency_ms) tags.push(`latency>${rule.filters.min_latency_ms}ms`);
		if (rule.filters.min_tokens) tags.push(`tokens>${rule.filters.min_tokens}`);
		return tags;
	}
</script>

<div class="max-w-6xl mx-auto space-y-4">
	<!-- Header -->
	<div class="flex items-center gap-2">
		<a href="/datasets" class="text-text-secondary hover:text-text text-sm">&larr; Datasets</a>
		<span class="text-text-muted">/</span>
		{#if dataset}
			{#if showEditDataset}
				<form class="flex items-end gap-2 flex-1" onsubmit={(e) => { e.preventDefault(); handleSaveDataset(); }}>
					<div>
						<label for="edit-ds-name" class="block text-xs text-text-muted uppercase mb-1">Name</label>
						<input id="edit-ds-name" type="text" bind:value={editName}
							class="bg-bg-tertiary border border-border rounded px-2 py-1 text-sm text-text" />
					</div>
					<div class="flex-1">
						<label for="edit-ds-desc" class="block text-xs text-text-muted uppercase mb-1">Description</label>
						<input id="edit-ds-desc" type="text" bind:value={editDescription} placeholder="Optional description"
							class="w-full bg-bg-tertiary border border-border rounded px-2 py-1 text-sm text-text placeholder:text-text-muted" />
					</div>
					<button type="submit" disabled={editSaving || !editName.trim()}
						class="px-3 py-1 text-xs bg-amber-400 text-bg font-semibold rounded hover:bg-amber-300 transition-colors disabled:opacity-50">
						{editSaving ? 'Saving...' : 'Save'}
					</button>
					<button type="button" onclick={() => (showEditDataset = false)}
						class="px-3 py-1 text-xs text-text-muted hover:text-text transition-colors">Cancel</button>
				</form>
			{:else}
				<h1 class="text-lg font-bold">{dataset.name}</h1>
				<span class="px-2 py-0.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded">
					{dataset.datapoint_count} datapoints
				</span>
				{#if dataset.description}
					<span class="text-text-muted text-sm">{dataset.description}</span>
				{/if}
				<button
					class="text-text-muted hover:text-text text-xs transition-colors"
					onclick={startEditDataset}
				>edit</button>
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
				{#if queueItemsByStatus.pending.length > 0}
					<span class="ml-1 px-1.5 py-0.5 text-xs bg-warning/20 text-warning rounded">{queueItemsByStatus.pending.length}</span>
				{/if}
			</button>
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'evals' ? 'border-amber-400 text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => (activeTab = 'evals')}
			>
				Evals
				{#if runningEvalCount > 0}
					<span class="ml-1 px-1.5 py-0.5 text-xs bg-purple-400/20 text-purple-400 rounded animate-pulse">{runningEvalCount}</span>
				{/if}
			</button>
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'rules' ? 'border-amber-400 text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => (activeTab = 'rules')}
			>
				Rules
				{#if enabledRuleCount > 0}
					<span class="ml-1 px-1.5 py-0.5 text-xs bg-amber-400/20 text-amber-400 rounded">{enabledRuleCount}</span>
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
					{#if datapoints.length > 0}
						<button
							class="px-3 py-1.5 text-xs bg-bg-tertiary text-text-secondary border border-border rounded hover:text-text transition-colors"
							onclick={exportDatasetJson}
						>Export JSON</button>
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
					{queueItemsByStatus.pending.length} pending
				</span>
				<span class="px-2 py-0.5 rounded text-xs border bg-accent/20 text-accent border-accent/30">
					{queueItemsByStatus.claimed.length} claimed
				</span>
				<span class="px-2 py-0.5 rounded text-xs border bg-success/20 text-success border-success/30">
					{queueItemsByStatus.completed.length} completed
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

				{#if queueItems.length === 0}
					<div class="text-text-muted text-sm text-center py-8">
						Queue is empty. Select datapoints and click "Enqueue" to start annotation.
					</div>
				{/if}
			</div>

		<!-- Tab: Evals -->
		{:else if activeTab === 'evals'}
			<div class="space-y-4">
				{#if evalRuns.length === 0 && !showNewEvalForm}
					<!-- Empty state -->
					<div class="text-center py-12">
						<p class="text-text-muted text-sm mb-1">No eval runs yet.</p>
						<p class="text-text-muted text-xs mb-4">Run your dataset against a model to see how it performs.</p>
						<button
							class="px-4 py-2 text-sm bg-amber-400 text-bg font-semibold rounded hover:bg-amber-300 transition-colors"
							onclick={() => (showNewEvalForm = true)}
						>+ New Eval Run</button>
					</div>
				{:else}
					<!-- Summary bar -->
					{#if evalRuns.length > 0}
						<div class="bg-bg-secondary border border-border rounded p-3 flex items-center gap-3 text-sm">
							<span class="text-text-secondary">{evalRuns.length} runs</span>
							{#if avgScore != null}
								<span class="text-text-muted">&middot;</span>
								<span class="text-text-secondary">avg score <span class="font-mono text-text">{avgScore.toFixed(2)}</span></span>
							{/if}
							{#if bestRun && bestRun.results.scores.mean != null}
								<span class="text-text-muted">&middot;</span>
								<span class="text-text-secondary">best: <span class="text-purple-400">{bestRun.config.model}</span> ({bestRun.results.scores.mean.toFixed(2)})</span>
							{/if}
							<div class="flex-1"></div>
							<button
								class="px-3 py-1.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors"
								onclick={() => (showNewEvalForm = !showNewEvalForm)}
							>{showNewEvalForm ? 'Cancel' : '+ New Eval Run'}</button>
							{#if completedEvalRuns.length >= 2}
								{#if evalCompareMode}
									<button
										class="px-3 py-1.5 text-xs bg-purple-400/10 text-purple-400 border border-purple-400/20 rounded hover:bg-purple-400/20 transition-colors disabled:opacity-50"
										disabled={evalCompareSelected.size < 2}
										onclick={goCompare}
									>Compare Selected ({evalCompareSelected.size})</button>
									<button
										class="text-text-muted hover:text-text text-xs"
										onclick={() => { evalCompareMode = false; evalCompareSelected = new Set(); }}
									>Cancel</button>
								{:else}
									<button
										class="px-3 py-1.5 text-xs bg-bg-tertiary text-text-secondary border border-border rounded hover:text-text transition-colors"
										onclick={() => (evalCompareMode = true)}
									>Compare Runs</button>
								{/if}
							{/if}
						</div>
					{/if}

				<!-- New Eval Run form -->
				{#if showNewEvalForm}
					<div class="bg-bg-secondary border border-border rounded p-4 space-y-3">
						<div class="text-sm font-semibold text-text">New Eval Run</div>

						<div>
							<label for="eval-name" class="block text-xs text-text-muted uppercase mb-1">Name (optional)</label>
							<input id="eval-name" type="text" bind:value={evalFormName} placeholder="e.g. gpt-4o baseline"
								class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
						</div>

						<!-- Provider Connection -->
						<div>
							<label for="eval-connection" class="block text-xs text-text-muted uppercase mb-1">Provider Connection *</label>
							{#if providerConnections.length > 0}
								<select id="eval-connection" bind:value={evalFormConnectionId}
									onchange={() => {
										const conn = providerConnections.find(c => c.id === evalFormConnectionId);
										if (conn) {
											if (conn.default_model) evalFormModel = conn.default_model;
											loadModelsForConnection(evalFormConnectionId);
										} else {
											connectionModels = [];
										}
									}}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text">
									<option value="">Select a provider...</option>
									{#each providerConnections as conn}
										<option value={conn.id}>{conn.name}{conn.default_model ? ` — ${conn.default_model}` : ''}</option>
									{/each}
								</select>
							{:else}
								<div class="bg-bg-tertiary border border-border rounded px-3 py-3 text-sm text-text-muted">
									No provider connections configured. <a href="/settings/providers" class="text-accent hover:underline">Add a provider</a> to run evals.
								</div>
							{/if}
						</div>

						<!-- Model (shown after connection selected) -->
						{#if evalFormConnectionId}
							<div>
								<label for="eval-model" class="block text-xs text-text-muted uppercase mb-1">Model *</label>
								{#if loadingModels}
									<div class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text-muted">Loading models...</div>
								{:else if connectionModels.length > 0}
									<select id="eval-model" bind:value={evalFormModel}
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text font-mono">
										{#if !evalFormModel}
											<option value="">Select a model...</option>
										{/if}
										{#each connectionModels as model}
											<option value={model.id}>{model.id}</option>
										{/each}
									</select>
								{:else}
									<input id="eval-model" type="text" bind:value={evalFormModel} placeholder="gpt-4o"
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted font-mono" />
								{/if}
							</div>

							<div>
								<label for="eval-sys" class="block text-xs text-text-muted uppercase mb-1">System Prompt Override (optional)</label>
								<textarea id="eval-sys" bind:value={evalFormSystemPrompt} rows={3}
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted resize-y"></textarea>
							</div>

							<div class="grid grid-cols-3 gap-3">
								<div>
									<label for="eval-temp" class="block text-xs text-text-muted uppercase mb-1">Temperature</label>
									<input id="eval-temp" type="text" bind:value={evalFormTemp} placeholder="0"
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
								</div>
								<div>
									<label for="eval-tokens" class="block text-xs text-text-muted uppercase mb-1">Max Tokens</label>
									<input id="eval-tokens" type="text" bind:value={evalFormMaxTokens} placeholder="1024"
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
								</div>
								<div>
									<label for="eval-scoring" class="block text-xs text-text-muted uppercase mb-1">Scoring</label>
									<select id="eval-scoring" bind:value={evalFormScoring}
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text">
										<option value="none">none</option>
										<option value="exact_match">exact_match</option>
										<option value="contains">contains</option>
										<option value="llm_judge">llm_judge</option>
									</select>
								</div>
							</div>

							<div class="flex items-center gap-2">
								<button
									class="px-4 py-2 text-sm bg-amber-400 text-bg font-semibold rounded hover:bg-amber-300 transition-colors disabled:opacity-50"
									disabled={evalCreating || !evalFormModel.trim() || !evalFormConnectionId}
									onclick={handleCreateEvalRun}
								>{evalCreating ? 'Running...' : 'Run Eval'}</button>
								<button
									class="text-text-secondary hover:text-text text-sm transition-colors"
									onclick={() => (showNewEvalForm = false)}
								>Cancel</button>
							</div>
						{/if}
					</div>
				{/if}

					<!-- Eval runs table -->
					{#if evalRuns.length > 0}
						<div class="grid grid-cols-[1fr_120px_80px_80px_80px_100px_60px] gap-3 px-3 text-xs text-text-muted uppercase items-center">
							{#if evalCompareMode}<span></span>{/if}
							<span>Name</span>
							<span>Status</span>
							<span>Score</span>
							<span>Pass Rate</span>
							<span>Datapoints</span>
							<span>Date</span>
							<span class="text-right">Actions</span>
						</div>
						<div class="space-y-0">
							{#each evalRuns as run (run.id)}
								<div
									class="grid grid-cols-[1fr_120px_80px_80px_80px_100px_60px] gap-3 items-center px-3 py-2 text-sm border-b border-border/50 hover:bg-bg-secondary transition-colors cursor-pointer"
									onclick={() => {
										if (!evalCompareMode) goto(`/datasets/${datasetId}/eval/${run.id}`);
									}}
								>
									<div class="flex items-center gap-2 min-w-0">
										{#if evalCompareMode}
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<!-- svelte-ignore a11y_no_static_element_interactions -->
											<span onclick={(e) => { e.stopPropagation(); toggleCompareSelect(run.id); }}>
												<input type="checkbox" checked={evalCompareSelected.has(run.id)} class="accent-purple-400" />
											</span>
										{/if}
										<span class="truncate text-text">{run.name ?? run.config.model}</span>
										<span class="shrink-0 px-1.5 py-0.5 text-xs bg-purple-400/10 text-purple-400 rounded">{run.config.model}</span>
									</div>
									<div>
										{#if run.status === 'running'}
											<EvalProgressBar completed={run.results.completed} total={run.results.total} />
										{:else}
											<StatusBadge status={run.status === 'completed' ? 'completed' : run.status === 'failed' || run.status === 'cancelled' ? 'failed' : 'running'} />
										{/if}
									</div>
									<div>
										<EvalScoreBadge score={run.results.scores.mean} size="xs" />
									</div>
									<div class="text-xs font-mono text-text-secondary">
										{run.results.scores.pass_rate != null ? `${Math.round(run.results.scores.pass_rate * 100)}%` : '\u2014'}
									</div>
									<div class="text-xs font-mono text-text-secondary">
										{run.results.completed}/{run.results.total}
									</div>
									<div class="text-xs text-text-muted">{relativeTime(run.created_at)}</div>
									<div class="text-right">
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<span onclick={(e) => e.stopPropagation()}>
											{#if run.status === 'running'}
												<button
													class="text-text-muted hover:text-warning text-xs transition-colors"
													onclick={() => handleCancelEvalRun(run.id)}
												>cancel</button>
											{:else}
												<button
													class="text-text-muted hover:text-danger text-xs transition-colors"
													onclick={() => handleDeleteEvalRun(run.id)}
												>{evalDeleteConfirm === run.id ? 'confirm?' : 'delete'}</button>
											{/if}
										</span>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				{/if}
			</div>

		<!-- Tab: Rules -->
		{:else if activeTab === 'rules'}
			<div class="space-y-4">
				{#if captureRules.length === 0 && !showNewRuleForm}
					<div class="text-center py-12">
						<p class="text-text-muted text-sm mb-1">No capture rules.</p>
						<p class="text-text-muted text-xs mb-4">Automatically add matching spans to this dataset.</p>
						<button
							class="px-4 py-2 text-sm bg-amber-400 text-bg font-semibold rounded hover:bg-amber-300 transition-colors"
							onclick={() => (showNewRuleForm = true)}
						>+ New Rule</button>
					</div>
				{:else}
					<div class="flex items-center gap-2">
						<button
							class="px-3 py-1.5 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors"
							onclick={() => (showNewRuleForm = !showNewRuleForm)}
						>{showNewRuleForm ? 'Cancel' : '+ New Rule'}</button>
					</div>

					<!-- New Rule form -->
					{#if showNewRuleForm}
						<div class="bg-bg-secondary border border-border rounded p-4 space-y-3">
							<div class="text-sm font-semibold text-text">New Capture Rule</div>
							<div>
								<label for="rule-name" class="block text-xs text-text-muted uppercase mb-1">Name *</label>
								<input id="rule-name" type="text" bind:value={ruleFormName} placeholder="e.g. slow production calls"
									class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
							</div>
							<div class="text-xs text-text-muted uppercase">Filters</div>
							<div class="grid grid-cols-3 gap-3">
								<div>
									<label for="rule-kind" class="block text-xs text-text-muted mb-1">Span Kind</label>
									<select id="rule-kind" bind:value={ruleFormSpanKind}
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text">
										<option value="">any</option>
										<option value="llm_call">llm_call</option>
										<option value="tool_call">tool_call</option>
										<option value="fs_read">fs_read</option>
										<option value="fs_write">fs_write</option>
										<option value="custom">custom</option>
									</select>
								</div>
								<div>
									<label for="rule-model" class="block text-xs text-text-muted mb-1">Model</label>
									<input id="rule-model" type="text" bind:value={ruleFormModel} placeholder="any"
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
								</div>
								<div>
									<label for="rule-provider" class="block text-xs text-text-muted mb-1">Provider</label>
									<input id="rule-provider" type="text" bind:value={ruleFormProvider} placeholder="any"
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
								</div>
							</div>
							<div class="grid grid-cols-3 gap-3">
								<div>
									<label for="rule-status" class="block text-xs text-text-muted mb-1">Status</label>
									<select id="rule-status" bind:value={ruleFormStatus}
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text">
										<option value="">any</option>
										<option value="completed">completed</option>
										<option value="failed">failed</option>
									</select>
								</div>
								<div>
									<label for="rule-name-contains" class="block text-xs text-text-muted mb-1">Name Contains</label>
									<input id="rule-name-contains" type="text" bind:value={ruleFormNameContains} placeholder=""
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
								</div>
								<div>
									<label for="rule-sample" class="block text-xs text-text-muted mb-1">Sample Rate</label>
									<select id="rule-sample" bind:value={ruleFormSampleRate}
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text">
										<option value={1.0}>100%</option>
										<option value={0.5}>50%</option>
										<option value={0.25}>25%</option>
										<option value={0.1}>10%</option>
										<option value={0.01}>1%</option>
									</select>
								</div>
							</div>
							<div class="grid grid-cols-2 gap-3">
								<div>
									<label for="rule-latency" class="block text-xs text-text-muted mb-1">Min Latency (ms)</label>
									<input id="rule-latency" type="text" bind:value={ruleFormMinLatency} placeholder=""
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
								</div>
								<div>
									<label for="rule-tokens" class="block text-xs text-text-muted mb-1">Min Tokens</label>
									<input id="rule-tokens" type="text" bind:value={ruleFormMinTokens} placeholder=""
										class="w-full bg-bg-tertiary border border-border rounded px-3 py-1.5 text-sm text-text placeholder:text-text-muted" />
								</div>
							</div>
							<div class="flex items-center gap-2">
								<button
									class="px-4 py-2 text-sm bg-amber-400 text-bg font-semibold rounded hover:bg-amber-300 transition-colors disabled:opacity-50"
									disabled={ruleCreating || !ruleFormName.trim()}
									onclick={handleCreateRule}
								>{ruleCreating ? 'Saving...' : 'Save Rule'}</button>
								<button
									class="text-text-secondary hover:text-text text-sm transition-colors"
									onclick={() => (showNewRuleForm = false)}
								>Cancel</button>
							</div>
						</div>
					{/if}

					<!-- Rule list -->
					{#each captureRules as rule (rule.id)}
						<div class="bg-bg-secondary border border-border rounded p-3">
							<div class="flex items-center gap-2">
								<span class="w-2 h-2 rounded-full {rule.enabled ? 'bg-success' : 'bg-text-muted'}"></span>
								<span class="text-sm text-text font-medium flex-1">{rule.name}</span>
								<button
									class="w-9 h-5 rounded-full transition-colors relative {rule.enabled ? 'bg-success' : 'bg-bg-tertiary'}"
									onclick={() => handleToggleRule(rule.id)}
								>
									<span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {rule.enabled ? 'translate-x-4' : ''}"></span>
								</button>
								<button
									class="text-text-muted hover:text-danger text-xs transition-colors"
									onclick={() => handleDeleteRule(rule.id)}
								>delete</button>
							</div>
							<div class="flex items-center gap-1 mt-1.5">
								{#each ruleFilterTags(rule) as tag}
									<span class="bg-bg-tertiary text-text-secondary rounded px-1.5 py-0.5 text-xs">{tag}</span>
								{/each}
							</div>
							<div class="text-xs text-text-muted mt-1">
								Sample: {Math.round(rule.sample_rate * 100)}% &middot; Captured: {rule.captured_count} datapoints
							</div>
						</div>
					{/each}
				{/if}
			</div>
		{/if}
	{/if}
</div>
