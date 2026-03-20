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
		updateDatapoint,
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
	let queueSelectedId: string | null = $state(null);
	let editedJson = $state('');
	const selectedQueueItem = $derived(queueItems.find((q) => q.id === queueSelectedId) ?? null);
	const selectedQueueDatapoint = $derived(selectedQueueItem ? datapoints.find((d) => d.id === selectedQueueItem.datapoint_id) ?? null : null);

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
	let evalSelectedRunId: string | null = $state(null);
	const selectedEvalRun = $derived(evalRuns.find((r) => r.id === evalSelectedRunId) ?? null);

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
	let selectedDatapointId: string | null = $state(null);
	const selectedDatapoint = $derived(datapoints.find((d) => d.id === selectedDatapointId) ?? null);

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

	// ── Datapoint search, sort, filter ────────────────────────────────
	let dpSearch = $state('');
	let dpSortField: 'kind' | 'source' | 'created_at' | 'preview' = $state('created_at');
	let dpSortDir: 'asc' | 'desc' = $state('desc');
	let dpShowColumns: { kind: boolean; preview: boolean; source: boolean; created_at: boolean } = $state({ kind: true, preview: true, source: true, created_at: true });
	let showColumnToggle = $state(false);

	const filteredDatapoints = $derived.by(() => {
		let list = datapoints;
		if (dpSearch.trim()) {
			const q = dpSearch.trim().toLowerCase();
			list = list.filter((dp) => {
				const preview = datapointPreview(dp).toLowerCase();
				const kind = dp.kind.type.toLowerCase();
				const source = dp.source.toLowerCase();
				const id = dp.id.toLowerCase();
				return preview.includes(q) || kind.includes(q) || source.includes(q) || id.includes(q);
			});
		}
		const sorted = [...list].sort((a, b) => {
			let cmp = 0;
			if (dpSortField === 'kind') cmp = a.kind.type.localeCompare(b.kind.type);
			else if (dpSortField === 'source') cmp = a.source.localeCompare(b.source);
			else if (dpSortField === 'created_at') cmp = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
			else if (dpSortField === 'preview') cmp = datapointPreview(a).localeCompare(datapointPreview(b));
			return dpSortDir === 'asc' ? cmp : -cmp;
		});
		return sorted;
	});

	function toggleDpSort(field: typeof dpSortField) {
		if (dpSortField === field) {
			dpSortDir = dpSortDir === 'asc' ? 'desc' : 'asc';
		} else {
			dpSortField = field;
			dpSortDir = 'asc';
		}
	}

	function sortIndicator(field: typeof dpSortField): string {
		if (dpSortField !== field) return '';
		return dpSortDir === 'asc' ? ' \u2191' : ' \u2193';
	}

	// ── Detail format toggle ──────────────────────────────────────────
	let detailFormat: 'pretty' | 'json' | 'yaml' = $state('pretty');

	function toYaml(value: unknown, indent = 0): string {
		if (value === null || value === undefined) return 'null';
		if (typeof value === 'string') return value;
		if (typeof value === 'number' || typeof value === 'boolean') return String(value);
		if (Array.isArray(value)) {
			return value.map((v) => '  '.repeat(indent) + '- ' + toYaml(v, indent + 1)).join('\n');
		}
		if (typeof value === 'object') {
			return Object.entries(value as Record<string, unknown>).map(([k, v]) => '  '.repeat(indent) + k + ': ' + toYaml(v, indent + 1)).join('\n');
		}
		return String(value);
	}

	function formatForMode(value: unknown, mode: 'pretty' | 'json' | 'yaml'): string {
		if (mode === 'json') return JSON.stringify(value, null, 2) ?? 'null';
		if (mode === 'yaml') return toYaml(value);
		return formatJson(value);
	}

	// ── Inline editing ────────────────────────────────────────────────
	let editingCell: { dpId: string; field: string } | null = $state(null);
	let editingCellValue = $state('');

	function startCellEdit(dpId: string, field: string, currentValue: string) {
		editingCell = { dpId, field };
		editingCellValue = currentValue;
	}

	function cancelCellEdit() {
		editingCell = null;
		editingCellValue = '';
	}

	async function saveCellEdit() {
		if (!editingCell) return;
		const dp = datapoints.find((d) => d.id === editingCell!.dpId);
		if (!dp) { cancelCellEdit(); return; }
		const kind = { ...dp.kind } as Record<string, unknown>;
		try {
			if (editingCell.field === 'input') {
				try { kind.input = JSON.parse(editingCellValue); } catch { kind.input = editingCellValue; }
			} else if (editingCell.field === 'target') {
				try { kind.target = JSON.parse(editingCellValue); } catch { kind.target = editingCellValue; }
				if (dp.kind.type === 'llm_conversation') {
					try { kind.expected_output = JSON.parse(editingCellValue); } catch { kind.expected_output = editingCellValue; }
				}
			} else if (editingCell.field === 'metadata') {
				try { kind.metadata = JSON.parse(editingCellValue); } catch { kind.metadata = editingCellValue; }
			}
			const updated = await updateDatapoint(datasetId, dp.id, kind as DatapointKind);
			datapoints = datapoints.map((d) => d.id === updated.id ? updated : d);
		} catch {
			// error — revert silently
		}
		cancelCellEdit();
	}

	function handleCellKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			saveCellEdit();
		} else if (e.key === 'Escape') {
			cancelCellEdit();
		}
	}

	// ── Dataset stats ─────────────────────────────────────────────────
	const datasetStats = $derived.by(() => {
		const totalRows = datapoints.length;
		const avgEvalScore = avgScore;
		let lastImport: string | null = null;
		const importedDps = datapoints.filter((d) => d.source === 'import');
		if (importedDps.length > 0) {
			const sorted = [...importedDps].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
			lastImport = sorted[0].created_at;
		}
		return { totalRows, avgEvalScore, lastImport };
	});

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
			queueSelectedId = updated.id;
			getQueue(datasetId).then((q) => (queueItems = q.items)).catch(() => {});
		} catch {
			// error
		}
	}

	function startEditing(item: QueueItem) {
		queueSelectedId = item.id;
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

	function datapointData(dp: Datapoint): unknown {
		const kind = dp.kind as any;
		if (dp.kind.type === 'llm_conversation') return kind.messages ?? [];
		return kind.input;
	}

	function datapointTarget(dp: Datapoint): unknown {
		const kind = dp.kind as any;
		if (dp.kind.type === 'llm_conversation') return kind.expected_output ?? null;
		return kind.target ?? null;
	}

	function datapointMetadata(dp: Datapoint): unknown {
		const kind = dp.kind as any;
		return kind.metadata ?? null;
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

	$effect(() => {
		if (selectedDatapointId && !datapoints.some((d) => d.id === selectedDatapointId)) {
			selectedDatapointId = null;
		}
	});

	$effect(() => {
		if (activeTab !== 'datapoints') selectedDatapointId = null;
	});

	$effect(() => {
		if (queueSelectedId && !queueItems.some((q) => q.id === queueSelectedId)) {
			queueSelectedId = null;
		}
	});

	$effect(() => {
		if (activeTab !== 'queue') {
			queueSelectedId = null;
			editingItem = null;
		}
	});

	$effect(() => {
		if (evalSelectedRunId && !evalRuns.some((r) => r.id === evalSelectedRunId)) {
			evalSelectedRunId = null;
		}
	});

	$effect(() => {
		if (activeTab !== 'evals') evalSelectedRunId = null;
	});

	$effect(() => {
		if (activeTab === 'evals' && !evalSelectedRunId && evalRuns.length > 0) {
			evalSelectedRunId = evalRuns[0].id;
		}
	});

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

<div class="app-shell-wide space-y-4">
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
							class="control-input h-8 text-sm" />
					</div>
					<div class="flex-1">
						<label for="edit-ds-desc" class="block text-xs text-text-muted uppercase mb-1">Description</label>
						<input id="edit-ds-desc" type="text" bind:value={editDescription} placeholder="Optional description"
							class="control-input h-8 text-sm" />
					</div>
					<button type="submit" disabled={editSaving || !editName.trim()} class="btn-primary h-8 text-xs">
						{editSaving ? 'Saving...' : 'Save'}
					</button>
					<button type="button" onclick={() => (showEditDataset = false)} class="btn-ghost h-8 text-xs">Cancel</button>
				</form>
			{:else}
				<h1 class="text-xl font-semibold tracking-tight">{dataset.name}</h1>
				<span class="text-[11px] text-text-muted font-mono tabular-nums">
					{dataset.datapoint_count} datapoints
				</span>
				{#if dataset.description}
					<span class="text-text-muted text-xs">{dataset.description}</span>
				{/if}
				<button class="btn-ghost h-6 text-[11px]" onclick={startEditDataset}>edit</button>
			{/if}
		{:else if !loading}
			<h1 class="text-xl font-semibold tracking-tight text-text-muted">Dataset not found</h1>
		{/if}
	</div>

	<!-- Stats + tabs bar -->
	{#if dataset && !loading}
		<div class="flex items-center gap-3 border-b border-border/40 pb-2">
			<div class="flex items-center gap-1">
				<button class="query-chip {activeTab === 'datapoints' ? 'query-chip-active' : ''}" onclick={() => (activeTab = 'datapoints')}>
					Datapoints
					<span class="text-text-muted/60 ml-0.5">{datapoints.length}</span>
				</button>
				<button class="query-chip {activeTab === 'import' ? 'query-chip-active' : ''}" onclick={() => (activeTab = 'import')}>Import</button>
				<button class="query-chip {activeTab === 'queue' ? 'query-chip-active' : ''}" onclick={() => (activeTab = 'queue')}>
					Queue
					{#if queueItemsByStatus.pending.length > 0}
						<span class="text-warning ml-0.5">{queueItemsByStatus.pending.length}</span>
					{/if}
				</button>
				<button class="query-chip {activeTab === 'evals' ? 'query-chip-active' : ''}" onclick={() => (activeTab = 'evals')}>
					Evals
					{#if runningEvalCount > 0}
						<span class="text-purple-400 ml-0.5 animate-pulse">{runningEvalCount}</span>
					{/if}
				</button>
				<button class="query-chip {activeTab === 'rules' ? 'query-chip-active' : ''}" onclick={() => (activeTab = 'rules')}>
					Rules
					{#if enabledRuleCount > 0}
						<span class="text-accent ml-0.5">{enabledRuleCount}</span>
					{/if}
				</button>
			</div>
			<div class="flex-1"></div>
			<div class="flex items-center gap-3 text-[11px] text-text-muted">
				<span><span class="text-text font-mono tabular-nums">{datasetStats.totalRows}</span> rows</span>
				<span>eval: <span class="text-text font-mono tabular-nums">{datasetStats.avgEvalScore != null ? datasetStats.avgEvalScore.toFixed(2) : '\u2014'}</span></span>
				<span>import: <span class="text-text font-mono tabular-nums">{datasetStats.lastImport ? formatDate(datasetStats.lastImport) : '\u2014'}</span></span>
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="text-text-muted text-sm text-center py-8">Loading...</div>
	{:else if !dataset}
		<div class="text-text-muted text-sm text-center py-8">Dataset not found</div>
	{:else}
		<!-- Content area -->
		<div class="min-w-0">
				<!-- Tab: Datapoints -->
				{#if activeTab === 'datapoints'}
					<div class="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_420px] gap-0 items-start">
						<div class="space-y-3 xl:pr-3">
							<div class="flex items-center gap-2 flex-wrap">
								<button class="btn-secondary h-8" onclick={() => (showAddForm = !showAddForm)}>{showAddForm ? 'Cancel' : '+ Add Datapoint'}</button>
								{#if selected.size > 0}
									<button class="btn-primary h-8" onclick={handleEnqueueSelected}>Enqueue {selected.size} selected</button>
								{/if}
								{#if datapoints.length > 0}
									<button class="btn-ghost h-8" onclick={exportDatasetJson}>Export JSON</button>
								{/if}
							</div>

							{#if showAddForm}
								<div class="table-float p-4 space-y-3">
									<div>
										<label for="dp-kind" class="label-micro block uppercase mb-1">DatapointKind (JSON)</label>
										<textarea id="dp-kind" bind:value={addKindJson} rows={8} class="control-textarea text-xs font-mono"></textarea>
									</div>
									{#if addError}
										<div class="alert-danger">{addError}</div>
									{/if}
									<button class="btn-primary" disabled={adding} onclick={handleAddDatapoint}>{adding ? 'Adding...' : 'Add'}</button>
								</div>
							{/if}

							<!-- Search and column controls -->
							<div class="flex items-center gap-2">
								<div class="flex-1 relative">
									<label for="dp-search" class="sr-only">Search datapoints</label>
									<input
										id="dp-search"
										type="text"
										bind:value={dpSearch}
										placeholder="Search datapoints..."
										class="w-full bg-bg-tertiary border border-border rounded-md px-3 py-1.5 text-sm text-text placeholder:text-text-muted pl-8"
									/>
									<svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
										<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
									</svg>
								</div>
								<div class="relative">
									<button
										class="btn-ghost h-8 text-xs"
										aria-label="Toggle column visibility"
										onclick={() => (showColumnToggle = !showColumnToggle)}
									>Columns</button>
									{#if showColumnToggle}
										<div class="absolute right-0 top-full mt-1 bg-bg-secondary border border-border rounded-md p-2 space-y-1 z-10 shadow-lg min-w-[140px]">
											<label class="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
												<input type="checkbox" bind:checked={dpShowColumns.kind} class="accent-amber-400" />
												Kind
											</label>
											<label class="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
												<input type="checkbox" bind:checked={dpShowColumns.preview} class="accent-amber-400" />
												Preview
											</label>
											<label class="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
												<input type="checkbox" bind:checked={dpShowColumns.source} class="accent-amber-400" />
												Source
											</label>
											<label class="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
												<input type="checkbox" bind:checked={dpShowColumns.created_at} class="accent-amber-400" />
												Created
											</label>
										</div>
									{/if}
								</div>
							</div>

							<div class="table-float overflow-hidden">
								<!-- Dynamic column header -->
								<div class="flex gap-3 px-3 py-2 table-head-compact items-center border-b border-border/55">
									<div class="w-[32px] shrink-0">
										<label class="flex items-center justify-center">
											<input type="checkbox" checked={selected.size === filteredDatapoints.length && filteredDatapoints.length > 0} onchange={toggleSelectAll} class="accent-amber-400" aria-label="Select all datapoints" />
										</label>
									</div>
									{#if dpShowColumns.kind}
										<button
											class="w-[84px] shrink-0 text-left text-xs hover:text-text transition-colors cursor-pointer"
											onclick={() => toggleDpSort('kind')}
											aria-label="Sort by kind"
										>Kind{sortIndicator('kind')}</button>
									{/if}
									{#if dpShowColumns.preview}
										<button
											class="flex-1 min-w-0 text-left text-xs hover:text-text transition-colors cursor-pointer"
											onclick={() => toggleDpSort('preview')}
											aria-label="Sort by preview"
										>Preview{sortIndicator('preview')}</button>
									{/if}
									{#if dpShowColumns.source}
										<button
											class="w-[90px] shrink-0 text-left text-xs hover:text-text transition-colors cursor-pointer"
											onclick={() => toggleDpSort('source')}
											aria-label="Sort by source"
										>Source{sortIndicator('source')}</button>
									{/if}
									{#if dpShowColumns.created_at}
										<button
											class="w-[120px] shrink-0 text-left text-xs hover:text-text transition-colors cursor-pointer"
											onclick={() => toggleDpSort('created_at')}
											aria-label="Sort by created date"
										>Created{sortIndicator('created_at')}</button>
									{/if}
									<span class="w-[80px] shrink-0 text-right text-xs">Actions</span>
								</div>

								{#if filteredDatapoints.length === 0}
									<div class="text-text-muted text-sm text-center py-10">{dpSearch.trim() ? 'No matching datapoints' : 'No datapoints yet'}</div>
								{:else}
									<div class="space-y-0">
										{#each filteredDatapoints as dp (dp.id)}
											<div
												class="flex gap-3 items-center px-3 py-2 text-sm border-b border-border/45 hover:bg-bg-secondary/45 motion-row cursor-pointer {selectedDatapointId === dp.id ? 'bg-bg-secondary/72' : ''}"
												onclick={() => (selectedDatapointId = dp.id)}
												onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (selectedDatapointId = dp.id)}
												role="button"
												tabindex={0}
											>
												<div class="w-[32px] shrink-0">
													<label class="flex items-center justify-center">
														<input type="checkbox" checked={selected.has(dp.id)} onclick={(e) => e.stopPropagation()} onchange={() => toggleSelect(dp.id)} class="accent-amber-400" aria-label="Select datapoint {shortId(dp.id)}" />
													</label>
												</div>
												{#if dpShowColumns.kind}
													<div class="w-[84px] shrink-0">
														<span class="text-xs px-1.5 py-0.5 rounded border {dp.kind.type === 'llm_conversation' ? 'bg-purple-400/10 text-purple-400 border-purple-400/20' : 'bg-text-muted/10 text-text-secondary border-text-muted/20'}">
															{dp.kind.type === 'llm_conversation' ? 'LLM' : 'Generic'}
														</span>
													</div>
												{/if}
												{#if dpShowColumns.preview}
													<div class="flex-1 min-w-0">
														{#if editingCell && editingCell.dpId === dp.id && editingCell.field === 'preview'}
															<!-- svelte-ignore a11y_autofocus -->
															<input
																type="text"
																bind:value={editingCellValue}
																onblur={saveCellEdit}
																onkeydown={handleCellKeydown}
																onclick={(e) => e.stopPropagation()}
																class="w-full bg-bg-tertiary border border-amber-400/50 rounded px-1.5 py-0.5 text-xs text-text font-mono"
																autofocus
															/>
														{:else}
															<!-- svelte-ignore a11y_no_static_element_interactions -->
														<span
																class="text-text-secondary text-xs truncate font-mono block"
																ondblclick={(e) => { e.stopPropagation(); startCellEdit(dp.id, 'input', String(datapointData(dp) ?? '')); }}
																title="Double-click to edit"
															>{datapointPreview(dp).slice(0, 110)}</span>
														{/if}
													</div>
												{/if}
												{#if dpShowColumns.source}
													<span class="w-[90px] shrink-0 text-text-muted text-xs">{dp.source}</span>
												{/if}
												{#if dpShowColumns.created_at}
													<span class="w-[120px] shrink-0 text-text-muted text-xs">{formatDate(dp.created_at)}</span>
												{/if}
												<div class="w-[80px] shrink-0 text-right flex items-center justify-end gap-2">
													<button class="text-text-muted hover:text-danger text-xs transition-colors" onclick={(e) => { e.stopPropagation(); handleDeleteDatapoint(dp.id); }}>delete</button>
												</div>
											</div>
										{/each}
									</div>
								{/if}
							</div>
						</div>

						<div class="min-h-[420px] border border-border/55 xl:rounded-l-none rounded-xl bg-bg-secondary/24 p-3.5 motion-fade-in">
							{#if selectedDatapoint}
								<div class="space-y-3">
									<div class="flex items-center justify-between">
										<div>
											<div class="text-[14px] font-semibold text-text">Row {shortId(selectedDatapoint.id)}</div>
											<div class="text-[11px] text-text-muted">{formatDate(selectedDatapoint.created_at)}</div>
										</div>
										<button class="btn-ghost h-7 text-[12px]" onclick={() => (selectedDatapointId = null)}>Close</button>
									</div>

									<div class="grid grid-cols-2 gap-2 text-[12px]">
										<div class="surface-quiet px-2.5 py-2">
											<div class="text-text-muted">Source</div>
											<div class="text-text mt-0.5">{selectedDatapoint.source}</div>
										</div>
										<div class="surface-quiet px-2.5 py-2">
											<div class="text-text-muted">Kind</div>
											<div class="text-text mt-0.5">{selectedDatapoint.kind.type}</div>
										</div>
									</div>

									<!-- Format toggle tabs -->
									<div class="flex items-center gap-1 border-b border-border/50 pb-1">
										<button
											class="px-2 py-1 text-[11px] rounded-t {detailFormat === 'pretty' ? 'bg-bg-tertiary/85 text-text border border-b-0 border-border/60' : 'text-text-muted hover:text-text'} transition-colors"
											onclick={() => (detailFormat = 'pretty')}
										>Pretty</button>
										<button
											class="px-2 py-1 text-[11px] rounded-t {detailFormat === 'json' ? 'bg-bg-tertiary/85 text-text border border-b-0 border-border/60' : 'text-text-muted hover:text-text'} transition-colors"
											onclick={() => (detailFormat = 'json')}
										>JSON</button>
										<button
											class="px-2 py-1 text-[11px] rounded-t {detailFormat === 'yaml' ? 'bg-bg-tertiary/85 text-text border border-b-0 border-border/60' : 'text-text-muted hover:text-text'} transition-colors"
											onclick={() => (detailFormat = 'yaml')}
										>YAML</button>
									</div>

									<div class="space-y-1.5">
										<div class="flex items-center justify-between">
											<div class="label-micro uppercase">Data</div>
											<button
												class="text-[10px] text-text-muted hover:text-amber-400 transition-colors"
												onclick={() => startCellEdit(selectedDatapoint.id, 'input', formatJson(datapointData(selectedDatapoint)))}
												aria-label="Edit data field"
											>edit</button>
										</div>
										{#if editingCell && editingCell.dpId === selectedDatapoint.id && editingCell.field === 'input'}
											<!-- svelte-ignore a11y_autofocus -->
											<textarea
												bind:value={editingCellValue}
												onblur={saveCellEdit}
												onkeydown={handleCellKeydown}
												onclick={(e) => e.stopPropagation()}
												rows={6}
												class="w-full bg-bg-tertiary border border-amber-400/50 rounded-lg p-2.5 text-[12px] text-text font-mono resize-y"
												autofocus
											></textarea>
										{:else}
											<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap font-mono">{formatForMode(datapointData(selectedDatapoint), detailFormat)}</pre>
										{/if}
									</div>
									<div class="space-y-1.5">
										<div class="flex items-center justify-between">
											<div class="label-micro uppercase">Target</div>
											<button
												class="text-[10px] text-text-muted hover:text-amber-400 transition-colors"
												onclick={() => startCellEdit(selectedDatapoint.id, 'target', formatJson(datapointTarget(selectedDatapoint)))}
												aria-label="Edit target field"
											>edit</button>
										</div>
										{#if editingCell && editingCell.dpId === selectedDatapoint.id && editingCell.field === 'target'}
											<!-- svelte-ignore a11y_autofocus -->
											<textarea
												bind:value={editingCellValue}
												onblur={saveCellEdit}
												onkeydown={handleCellKeydown}
												onclick={(e) => e.stopPropagation()}
												rows={4}
												class="w-full bg-bg-tertiary border border-amber-400/50 rounded-lg p-2.5 text-[12px] text-text font-mono resize-y"
												autofocus
											></textarea>
										{:else}
											<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap font-mono">{formatForMode(datapointTarget(selectedDatapoint), detailFormat)}</pre>
										{/if}
									</div>
									<div class="space-y-1.5">
										<div class="flex items-center justify-between">
											<div class="label-micro uppercase">Metadata</div>
											<button
												class="text-[10px] text-text-muted hover:text-amber-400 transition-colors"
												onclick={() => startCellEdit(selectedDatapoint.id, 'metadata', formatJson(datapointMetadata(selectedDatapoint)))}
												aria-label="Edit metadata field"
											>edit</button>
										</div>
										{#if editingCell && editingCell.dpId === selectedDatapoint.id && editingCell.field === 'metadata'}
											<!-- svelte-ignore a11y_autofocus -->
											<textarea
												bind:value={editingCellValue}
												onblur={saveCellEdit}
												onkeydown={handleCellKeydown}
												onclick={(e) => e.stopPropagation()}
												rows={4}
												class="w-full bg-bg-tertiary border border-amber-400/50 rounded-lg p-2.5 text-[12px] text-text font-mono resize-y"
												autofocus
											></textarea>
										{:else}
											<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap font-mono">{formatForMode(datapointMetadata(selectedDatapoint), detailFormat)}</pre>
										{/if}
									</div>
								</div>
							{:else}
								<div class="h-full flex flex-col justify-center items-center text-center px-3">
									<div class="text-[14px] font-semibold text-text">Datapoint preview</div>
									<p class="text-[12px] text-text-muted mt-1">Select a row on the left to inspect data, target, and metadata here.</p>
								</div>
							{/if}
						</div>
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
					<div class="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_420px] gap-3 items-start">
						<div class="space-y-3">
							<div class="flex items-center gap-3 text-sm">
								<span class="px-2 py-0.5 rounded text-xs border bg-warning/20 text-warning border-warning/30">{queueItemsByStatus.pending.length} pending</span>
								<span class="px-2 py-0.5 rounded text-xs border bg-accent/20 text-accent border-accent/30">{queueItemsByStatus.claimed.length} claimed</span>
								<span class="px-2 py-0.5 rounded text-xs border bg-success/20 text-success border-success/30">{queueItemsByStatus.completed.length} completed</span>
								<div class="flex-1"></div>
								<label for="claim-name" class="label-micro">Claim as:</label>
								<input id="claim-name" type="text" bind:value={claimName} class="control-input w-32" />
							</div>

							{#if queueItems.length === 0}
								<div class="text-text-muted text-sm text-center py-8">Queue is empty. Select datapoints and click "Enqueue" to start annotation.</div>
							{:else}
								<div class="table-float overflow-hidden">
									<div class="grid grid-cols-[96px_120px_120px_120px_86px] gap-3 px-3 py-2 table-head-compact border-b border-border/55">
										<span>Status</span>
										<span>Datapoint</span>
										<span>Owner</span>
										<span>Created</span>
										<span class="text-right">Action</span>
									</div>
									{#each queueItems as item (item.id)}
										<div
											class="grid grid-cols-[96px_120px_120px_120px_86px] gap-3 items-center px-3 py-2 border-b border-border/45 hover:bg-bg-secondary/45 motion-row cursor-pointer {queueSelectedId === item.id ? 'bg-bg-secondary/72' : ''}"
											role="button"
											tabindex={0}
											onclick={() => (queueSelectedId = item.id)}
											onkeydown={(e) => {
												if (e.key === 'Enter' || e.key === ' ') {
													e.preventDefault();
													queueSelectedId = item.id;
												}
											}}
										>
											<div>
												{#if item.status === 'pending'}
													<span class="px-2 py-0.5 rounded text-xs border bg-warning/20 text-warning border-warning/30">pending</span>
												{:else if item.status === 'claimed'}
													<span class="px-2 py-0.5 rounded text-xs border bg-accent/20 text-accent border-accent/30">claimed</span>
												{:else}
													<span class="px-2 py-0.5 rounded text-xs border bg-success/20 text-success border-success/30">completed</span>
												{/if}
											</div>
											<div class="text-text-secondary text-xs font-mono truncate">{shortId(item.datapoint_id)}</div>
											<div class="text-text-muted text-xs truncate">{item.claimed_by ?? '-'}</div>
											<div class="text-text-muted text-xs">{formatDate(item.created_at)}</div>
											<div class="text-right">
												{#if item.status === 'pending'}
													<button class="btn-ghost h-7 text-[12px]" onclick={(e) => { e.stopPropagation(); handleClaim(item.id); }}>Claim</button>
												{:else if item.status === 'claimed'}
													<button class="btn-ghost h-7 text-[12px]" onclick={(e) => { e.stopPropagation(); startEditing(item); }}>Edit</button>
												{:else}
													<span class="text-text-muted text-xs">-</span>
												{/if}
											</div>
										</div>
									{/each}
								</div>
							{/if}
						</div>

						<div class="surface-panel sticky top-24 p-3.5 min-h-[420px] motion-slide-in-right">
							{#if selectedQueueItem}
								<div class="space-y-3">
									<div class="flex items-center justify-between">
										<div>
											<div class="text-[14px] font-semibold text-text">Queue {shortId(selectedQueueItem.id)}</div>
											<div class="text-[11px] text-text-muted">{selectedQueueItem.status} - {formatDate(selectedQueueItem.created_at)}</div>
										</div>
										<button class="btn-ghost h-7 text-[12px]" onclick={() => { queueSelectedId = null; editingItem = null; }}>Close</button>
									</div>

									<div class="grid grid-cols-2 gap-2 text-[12px]">
										<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Datapoint</div><div class="text-text mt-0.5 font-mono">{shortId(selectedQueueItem.datapoint_id)}</div></div>
										<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Reviewer</div><div class="text-text mt-0.5">{selectedQueueItem.claimed_by ?? '-'}</div></div>
									</div>

									<div class="space-y-1.5">
										<div class="label-micro uppercase">Original</div>
										<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap">{formatJson(selectedQueueItem.original_data)}</pre>
									</div>

									{#if selectedQueueItem.status === 'claimed'}
										<div class="space-y-1.5">
											<div class="label-micro uppercase">Edited</div>
											<textarea bind:value={editedJson} rows={8} class="control-textarea text-xs font-mono"></textarea>
											<div class="flex items-center gap-2">
												<button class="btn-primary" onclick={handleSubmit}>Submit</button>
												<button class="btn-secondary" onclick={() => { editingItem = null; editedJson = ''; }}>Reset</button>
											</div>
										</div>
									{:else if selectedQueueItem.edited_data}
										<div class="space-y-1.5">
											<div class="label-micro uppercase">Edited Result</div>
											<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text whitespace-pre-wrap">{formatJson(selectedQueueItem.edited_data)}</pre>
										</div>
									{/if}

									{#if selectedQueueDatapoint}
										<div class="space-y-1.5">
											<div class="label-micro uppercase">Datapoint Snapshot</div>
											<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap">{formatJson(selectedQueueDatapoint.kind)}</pre>
										</div>
									{/if}
								</div>
							{:else}
								<div class="h-full flex flex-col justify-center items-center text-center px-3">
									<div class="text-[14px] font-semibold text-text">Queue item preview</div>
									<p class="text-[12px] text-text-muted mt-1">Select a queue row to inspect or edit it here.</p>
								</div>
							{/if}
						</div>
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
								<div class="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_420px] gap-3 items-start">
									<div class="table-float overflow-hidden">
										<div class="grid grid-cols-[1fr_120px_80px_80px_80px_100px_72px] gap-3 px-3 py-2 table-head-compact border-b border-border/55">
											{#if evalCompareMode}<span></span>{/if}
											<span>Name</span>
											<span>Status</span>
											<span>Score</span>
											<span>Pass Rate</span>
											<span>Datapoints</span>
											<span>Date</span>
											<span class="text-right">Actions</span>
										</div>
										{#each evalRuns as run (run.id)}
											<div
												class="grid grid-cols-[1fr_120px_80px_80px_80px_100px_72px] gap-3 items-center px-3 py-2 text-sm border-b border-border/45 hover:bg-bg-secondary/45 motion-row cursor-pointer {evalSelectedRunId === run.id ? 'bg-bg-secondary/72' : ''}"
												role="button"
												tabindex={evalCompareMode ? -1 : 0}
												aria-label={`Select eval run ${run.name ?? run.id}`}
												onclick={() => {
													if (!evalCompareMode) evalSelectedRunId = run.id;
												}}
												onkeydown={(e) => {
													if (evalCompareMode) return;
													if (e.key === 'Enter' || e.key === ' ') {
														e.preventDefault();
														evalSelectedRunId = run.id;
													}
												}}
											>
												<div class="flex items-center gap-2 min-w-0">
													{#if evalCompareMode}
														<button
															type="button"
															class="inline-flex"
															aria-label={`Toggle compare selection for ${run.name ?? run.id}`}
															onclick={(e) => { e.stopPropagation(); toggleCompareSelect(run.id); }}
														>
															<input type="checkbox" checked={evalCompareSelected.has(run.id)} class="accent-purple-400" />
														</button>
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
												<div><EvalScoreBadge score={run.results.scores.mean} size="xs" /></div>
												<div class="text-xs font-mono text-text-secondary">{run.results.scores.pass_rate != null ? `${Math.round(run.results.scores.pass_rate * 100)}%` : '\u2014'}</div>
												<div class="text-xs font-mono text-text-secondary">{run.results.completed}/{run.results.total}</div>
												<div class="text-xs text-text-muted">{relativeTime(run.created_at)}</div>
												<div class="text-right">
													{#if run.status === 'running'}
														<button class="text-text-muted hover:text-warning text-xs transition-colors" onclick={(e) => { e.stopPropagation(); handleCancelEvalRun(run.id); }}>cancel</button>
													{:else}
														<button class="text-text-muted hover:text-danger text-xs transition-colors" onclick={(e) => { e.stopPropagation(); handleDeleteEvalRun(run.id); }}>{evalDeleteConfirm === run.id ? 'confirm?' : 'delete'}</button>
													{/if}
												</div>
											</div>
										{/each}
									</div>

									<div class="surface-panel sticky top-24 p-3.5 min-h-[420px] motion-slide-in-right">
										{#if selectedEvalRun}
											<div class="space-y-3">
												<div class="flex items-center justify-between">
													<div>
														<div class="text-[14px] font-semibold text-text">{selectedEvalRun.name ?? selectedEvalRun.config.model}</div>
														<div class="text-[11px] text-text-muted">{selectedEvalRun.config.model} - {relativeTime(selectedEvalRun.created_at)}</div>
													</div>
													<button class="btn-ghost h-7 text-[12px]" onclick={() => (evalSelectedRunId = null)}>Close</button>
												</div>

												<div class="grid grid-cols-2 gap-2 text-[12px]">
													<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Status</div><div class="text-text mt-0.5 capitalize">{selectedEvalRun.status}</div></div>
													<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Score</div><div class="text-text mt-0.5 font-mono">{selectedEvalRun.results.scores.mean != null ? selectedEvalRun.results.scores.mean.toFixed(3) : '\u2014'}</div></div>
													<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Pass Rate</div><div class="text-text mt-0.5 font-mono">{selectedEvalRun.results.scores.pass_rate != null ? `${Math.round(selectedEvalRun.results.scores.pass_rate * 100)}%` : '\u2014'}</div></div>
													<div class="surface-quiet px-2.5 py-2"><div class="text-text-muted">Completed</div><div class="text-text mt-0.5 font-mono">{selectedEvalRun.results.completed}/{selectedEvalRun.results.total}</div></div>
												</div>

												<div class="space-y-1.5">
													<div class="label-micro uppercase">Config</div>
													<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap">{formatJson(selectedEvalRun.config)}</pre>
												</div>

												<div class="space-y-1.5">
													<div class="label-micro uppercase">Score Breakdown</div>
													<pre class="query-float rounded-lg border border-border/60 p-2.5 text-[12px] text-text-secondary whitespace-pre-wrap">{formatJson(selectedEvalRun.results.scores)}</pre>
												</div>

												<button class="btn-primary" onclick={() => goto(`/datasets/${datasetId}/eval/${selectedEvalRun.id}`)}>Open full run</button>
											</div>
										{:else}
											<div class="h-full flex flex-col justify-center items-center text-center px-3">
												<div class="text-[14px] font-semibold text-text">Eval preview</div>
												<p class="text-[12px] text-text-muted mt-1">Select an eval run to inspect config and quick metrics.</p>
											</div>
										{/if}
									</div>
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
								<button class="btn-secondary h-7 text-[12px]" onclick={() => (showNewRuleForm = !showNewRuleForm)}>
									{showNewRuleForm ? 'Cancel' : '+ New Rule'}
								</button>
							</div>

							<!-- New Rule form -->
							{#if showNewRuleForm}
								<div class="table-float p-3 space-y-3 motion-rise-in">
									<div class="text-[13px] font-semibold text-text">New Capture Rule</div>

									<div>
										<label for="rule-name" class="label-micro block uppercase mb-1">Name *</label>
										<input id="rule-name" type="text" bind:value={ruleFormName} placeholder="e.g. slow production calls" class="control-input h-8 text-[12px]" />
									</div>

									<div class="label-micro uppercase">Filters</div>
									<div class="grid grid-cols-3 gap-2">
										<div>
											<label for="rule-kind" class="block text-[10px] text-text-muted mb-0.5">Span Kind</label>
											<select id="rule-kind" bind:value={ruleFormSpanKind} class="control-select h-7 text-[12px]">
												<option value="">any</option>
												<option value="llm_call">llm_call</option>
												<option value="tool_call">tool_call</option>
												<option value="fs_read">fs_read</option>
												<option value="fs_write">fs_write</option>
												<option value="custom">custom</option>
											</select>
										</div>
										<div>
											<label for="rule-model" class="block text-[10px] text-text-muted mb-0.5">Model</label>
											<input id="rule-model" type="text" bind:value={ruleFormModel} placeholder="any" class="control-input h-7 text-[12px]" />
										</div>
										<div>
											<label for="rule-provider" class="block text-[10px] text-text-muted mb-0.5">Provider</label>
											<input id="rule-provider" type="text" bind:value={ruleFormProvider} placeholder="any" class="control-input h-7 text-[12px]" />
										</div>
									</div>
									<div class="grid grid-cols-3 gap-2">
										<div>
											<label for="rule-status" class="block text-[10px] text-text-muted mb-0.5">Status</label>
											<select id="rule-status" bind:value={ruleFormStatus} class="control-select h-7 text-[12px]">
												<option value="">any</option>
												<option value="completed">completed</option>
												<option value="failed">failed</option>
											</select>
										</div>
										<div>
											<label for="rule-name-contains" class="block text-[10px] text-text-muted mb-0.5">Name Contains</label>
											<input id="rule-name-contains" type="text" bind:value={ruleFormNameContains} class="control-input h-7 text-[12px]" />
										</div>
										<div>
											<label for="rule-sample" class="block text-[10px] text-text-muted mb-0.5">Sample Rate</label>
											<select id="rule-sample" bind:value={ruleFormSampleRate} class="control-select h-7 text-[12px]">
												<option value={1.0}>100%</option>
												<option value={0.5}>50%</option>
												<option value={0.25}>25%</option>
												<option value={0.1}>10%</option>
												<option value={0.01}>1%</option>
											</select>
										</div>
									</div>
									<div class="grid grid-cols-2 gap-2">
										<div>
											<label for="rule-latency" class="block text-[10px] text-text-muted mb-0.5">Min Latency (ms)</label>
											<input id="rule-latency" type="text" bind:value={ruleFormMinLatency} class="control-input h-7 text-[12px]" />
										</div>
										<div>
											<label for="rule-tokens" class="block text-[10px] text-text-muted mb-0.5">Min Tokens</label>
											<input id="rule-tokens" type="text" bind:value={ruleFormMinTokens} class="control-input h-7 text-[12px]" />
										</div>
									</div>
									<div class="flex items-center gap-2">
										<button class="btn-primary h-7 text-[12px]" disabled={ruleCreating || !ruleFormName.trim()} onclick={handleCreateRule}>
											{ruleCreating ? 'Saving...' : 'Save Rule'}
										</button>
										<button class="btn-ghost h-7 text-[12px]" onclick={() => (showNewRuleForm = false)}>Cancel</button>
									</div>
								</div>
							{/if}

							<!-- Rule list -->
							{#each captureRules as rule (rule.id)}
								<div class="table-float p-3">
									<div class="flex items-center gap-2">
										<span class="w-1.5 h-1.5 rounded-full shrink-0 {rule.enabled ? 'bg-success' : 'bg-text-muted'}"></span>
										<span class="text-[13px] text-text font-medium flex-1">{rule.name}</span>
										<button
											class="w-8 h-4 rounded-full transition-colors relative shrink-0 {rule.enabled ? 'bg-success' : 'bg-bg-tertiary border border-border'}"
											aria-label={`Toggle rule ${rule.name}`}
											onclick={() => handleToggleRule(rule.id)}
										>
											<span class="absolute top-0.5 left-0.5 w-3 h-3 rounded-full bg-white transition-transform {rule.enabled ? 'translate-x-4' : ''}"></span>
										</button>
										<button class="btn-ghost h-6 text-[10px] text-text-muted hover:text-danger" onclick={() => handleDeleteRule(rule.id)}>delete</button>
									</div>
									<div class="flex items-center gap-1 mt-1.5 flex-wrap">
										{#each ruleFilterTags(rule) as tag}
											<span class="query-chip h-5 text-[10px]">{tag}</span>
										{/each}
									</div>
									<div class="text-[11px] text-text-muted mt-1.5">
										Sample: {Math.round(rule.sample_rate * 100)}% · Captured: <span class="font-mono tabular-nums">{rule.captured_count}</span> datapoints
									</div>
								</div>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
	{/if}
</div>
