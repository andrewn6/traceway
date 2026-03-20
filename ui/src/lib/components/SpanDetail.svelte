<script lang="ts">
	import type { Span, DatasetWithCount } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs, spanError, spanKindLabel, shortId, getDatasets, exportSpanToDataset, exportSpanAndEnqueue, completeSpan, failSpan, deleteSpan, getFileContent } from '$lib/api';
	import StatusBadge from './StatusBadge.svelte';
	import SpanKindIcon from './SpanKindIcon.svelte';

	let { span, onSpanAction, allSpans = [], onClose }: { span: Span | null; onSpanAction?: () => void; allSpans?: Span[]; onClose?: () => void } = $props();

	let activeTab: 'messages' | 'metadata' = $state('messages');
	let formatMode: 'pretty' | 'json' | 'yaml' = $state('pretty');
	let inputExpanded = $state(true);
	let outputExpanded = $state(true);

	// File content (for fs_read / fs_write spans)
	let fileContent = $state('');
	let fileContentLoading = $state(false);
	let fileContentError = $state('');
	let fileContentLoaded = $state(false);

	const isFileSpan = $derived(span?.kind?.type === 'fs_read' || span?.kind?.type === 'fs_write');
	const fileHash = $derived(
		span?.kind?.type === 'fs_read' ? span.kind.file_version
		: span?.kind?.type === 'fs_write' ? span.kind.file_version
		: null
	);
	const filePath = $derived(
		span?.kind?.type === 'fs_read' ? span.kind.path
		: span?.kind?.type === 'fs_write' ? span.kind.path
		: null
	);

	async function loadFileContent() {
		if (!fileHash || fileContentLoaded) return;
		fileContentLoading = true;
		fileContentError = '';
		try {
			fileContent = await getFileContent(fileHash);
			fileContentLoaded = true;
		} catch {
			const out = span?.output as Record<string, unknown> | null | undefined;
			const fallback = out && typeof out === 'object'
				? (typeof out.file_content === 'string' ? out.file_content
					: typeof out.content === 'string' ? out.content
					: typeof out.preview === 'string' ? out.preview : '')
				: '';
			if (fallback) { fileContent = fallback; fileContentLoaded = true; }
			else { fileContentError = 'Could not load file content'; }
		}
		fileContentLoading = false;
	}

	function inferLanguage(path: string): string {
		const ext = path.split('.').pop()?.toLowerCase() ?? '';
		const map: Record<string, string> = { ts: 'typescript', tsx: 'tsx', js: 'javascript', py: 'python', rs: 'rust', go: 'go', json: 'json', md: 'markdown', sh: 'bash', sql: 'sql', css: 'css', html: 'html', svelte: 'svelte', yaml: 'yaml', toml: 'toml' };
		return map[ext] ?? 'plaintext';
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	// Export to dataset
	let showExportDropdown = $state(false);
	let exportDatasets: DatasetWithCount[] = $state([]);
	let exportLoading = $state(false);
	let exportSuccess = $state('');
	let showReviewDropdown = $state(false);
	let reviewDatasets: DatasetWithCount[] = $state([]);
	let reviewLoading = $state(false);
	let reviewSuccess = $state('');

	async function openExportDropdown() {
		showExportDropdown = !showExportDropdown;
		showReviewDropdown = false;
		if (showExportDropdown) {
			try { const result = await getDatasets(); exportDatasets = result.datasets; } catch { exportDatasets = []; }
		}
	}

	async function doExport(datasetId: string) {
		if (!span) return;
		exportLoading = true; exportSuccess = '';
		try {
			await exportSpanToDataset(datasetId, span.id);
			const ds = exportDatasets.find((d) => d.id === datasetId);
			exportSuccess = `Exported to ${ds?.name ?? shortId(datasetId)}`;
			showExportDropdown = false;
		} catch { exportSuccess = 'Export failed'; }
		exportLoading = false;
		setTimeout(() => (exportSuccess = ''), 3000);
	}

	async function openReviewDropdown() {
		showReviewDropdown = !showReviewDropdown;
		showExportDropdown = false;
		if (showReviewDropdown) {
			try { const result = await getDatasets(); reviewDatasets = result.datasets; } catch { reviewDatasets = []; }
		}
	}

	async function doReview(datasetId: string) {
		if (!span) return;
		reviewLoading = true; reviewSuccess = '';
		try {
			await exportSpanAndEnqueue(datasetId, span.id);
			const ds = reviewDatasets.find((d) => d.id === datasetId);
			reviewSuccess = `Sent to review in ${ds?.name ?? shortId(datasetId)}`;
			showReviewDropdown = false;
		} catch { reviewSuccess = 'Failed to send to review'; }
		reviewLoading = false;
		setTimeout(() => (reviewSuccess = ''), 3000);
	}

	// Delete span
	let confirmDeleteSpan = $state(false);
	async function handleDeleteSpan() {
		if (!span) return;
		if (!confirmDeleteSpan) { confirmDeleteSpan = true; setTimeout(() => (confirmDeleteSpan = false), 3000); return; }
		try { await deleteSpan(span.id); onSpanAction?.(); } catch {}
		confirmDeleteSpan = false;
	}

	// Complete / Fail
	let showCompleteForm = $state(false);
	let completeOutput = $state('');
	let showFailForm = $state(false);
	let failError = $state('');
	let actionLoading = $state(false);

	async function handleComplete() {
		if (!span) return;
		actionLoading = true;
		try {
			const output = completeOutput.trim() ? JSON.parse(completeOutput) : undefined;
			await completeSpan(span.id, output);
			showCompleteForm = false; completeOutput = '';
			onSpanAction?.();
		} catch {}
		actionLoading = false;
	}

	async function handleFail() {
		if (!span || !failError.trim()) return;
		actionLoading = true;
		try {
			await failSpan(span.id, failError.trim());
			showFailForm = false; failError = '';
			onSpanAction?.();
		} catch {}
		actionLoading = false;
	}

	// Formatting
	function formatJson(value: unknown): string {
		if (value === null || value === undefined) return '(none)';
		if (typeof value === 'string') return value;
		return JSON.stringify(value, null, 2);
	}

	function jsonToYaml(value: unknown, indent = 0): string {
		const pfx = '  '.repeat(indent);
		if (value === null || value === undefined) return 'null';
		if (typeof value === 'string') return value.includes('\n') ? `|\n${value.split('\n').map(l => pfx + '  ' + l).join('\n')}` : JSON.stringify(value);
		if (typeof value === 'number' || typeof value === 'boolean') return String(value);
		if (Array.isArray(value)) {
			if (value.length === 0) return '[]';
			return value.map(v => `${pfx}- ${jsonToYaml(v, indent + 1).trimStart()}`).join('\n');
		}
		if (typeof value === 'object') {
			const entries = Object.entries(value);
			if (entries.length === 0) return '{}';
			return entries.map(([k, v]) => {
				const val = jsonToYaml(v, indent + 1);
				if (typeof v === 'object' && v !== null) return `${pfx}${k}:\n${val}`;
				return `${pfx}${k}: ${val}`;
			}).join('\n');
		}
		return String(value);
	}

	function renderValue(value: unknown): string {
		if (formatMode === 'json') return formatJson(value);
		if (formatMode === 'yaml') return jsonToYaml(value);
		return formatJson(value);
	}

	function kindMeta(s: Span): Record<string, string> {
		if (!s.kind) return {};
		switch (s.kind.type) {
			case 'fs_read': return { 'Path': s.kind.path, 'Version': s.kind.file_version ?? '-', 'Bytes': s.kind.bytes_read.toLocaleString() };
			case 'fs_write': return { 'Path': s.kind.path, 'Version': s.kind.file_version, 'Bytes': s.kind.bytes_written.toLocaleString() };
			case 'llm_call': {
				const meta: Record<string, string> = { 'Model': s.kind.model };
				if (s.kind.provider) meta['Provider'] = s.kind.provider;
				if (s.kind.input_tokens != null) meta['Input tokens'] = s.kind.input_tokens.toLocaleString();
				if (s.kind.output_tokens != null) meta['Output tokens'] = s.kind.output_tokens.toLocaleString();
				if (s.kind.cost != null) meta['Cost'] = `$${s.kind.cost.toFixed(6)}`;
				return meta;
			}
			case 'custom': return { 'Kind': s.kind.kind };
			default: return {};
		}
	}

	// Chat message detection
	interface ChatMessage { role: string; content: string; }

	function extractMessages(value: unknown): ChatMessage[] | null {
		if (!value) return null;
		if (Array.isArray(value) && value.length > 0 && value[0].role && value[0].content !== undefined) return value as ChatMessage[];
		if (typeof value === 'object' && value !== null) {
			const obj = value as Record<string, unknown>;
			if (Array.isArray(obj.messages) && obj.messages.length > 0 && (obj.messages[0] as Record<string, unknown>).role) return obj.messages as ChatMessage[];
		}
		return null;
	}

	function roleColor(role: string): string {
		switch (role.toLowerCase()) {
			case 'system': return 'text-warning';
			case 'user': return 'text-accent';
			case 'assistant': return 'text-success';
			case 'tool': return 'text-purple-400';
			default: return 'text-text-secondary';
		}
	}

	function wordCount(text: unknown): number {
		if (typeof text !== 'string') return 0;
		return text.trim().split(/\s+/).filter(Boolean).length;
	}

	function charCount(text: unknown): number {
		if (typeof text !== 'string') return 0;
		return text.length;
	}

	function copyText(text: unknown) {
		const str = typeof text === 'string' ? text : JSON.stringify(text, null, 2);
		navigator.clipboard.writeText(str);
	}

	const inputMessages = $derived(span ? extractMessages(span.input) : null);
	const outputMessages = $derived(span ? extractMessages(span.output) : null);
	const childSpans = $derived(span ? allSpans.filter((s) => s.parent_id === span.id) : []);

	$effect(() => {
		if (span) {
			inputExpanded = true;
			outputExpanded = true;
			formatMode = 'pretty';
			fileContent = ''; fileContentLoading = false; fileContentError = ''; fileContentLoaded = false;
			showExportDropdown = false; showReviewDropdown = false;
			showCompleteForm = false; showFailForm = false;
			confirmDeleteSpan = false;
			if (isFileSpan) { activeTab = 'messages'; loadFileContent(); }
			else { activeTab = 'messages'; }
		}
	});

	function formatDuration(ms: number | null): string {
		if (ms === null) return '...';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}
</script>

{#if span}
	{@const status = spanStatus(span)}
	{@const duration = spanDurationMs(span)}
	{@const error = spanError(span)}
	{@const meta = kindMeta(span)}

	<div class="flex flex-col h-full min-h-0 text-[12px]">
		<!-- Header -->
		<div class="px-3 py-2 border-b border-border/40 shrink-0 space-y-1.5">
			<div class="flex items-center gap-1.5">
				<SpanKindIcon {span} />
				<h2 class="font-semibold text-[13px] text-text flex-1 truncate">Span {shortId(span.id)}</h2>
				<StatusBadge {status} />
				{#if onClose}
					<button onclick={onClose} class="w-6 h-6 rounded flex items-center justify-center text-text-muted hover:text-text hover:bg-bg-tertiary/70 transition-colors" aria-label="Close">
						<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" /></svg>
					</button>
				{/if}
			</div>

			{#if span.kind?.type === 'llm_call'}
				<div class="flex items-center gap-1.5 flex-wrap text-[11px]">
					<span class="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-bg-tertiary border border-border text-text-secondary font-mono">
						<svg class="w-3 h-3 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09ZM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 0 0-2.455 2.456Z" /></svg>
						{span.kind.model}
					</span>
					<span class="px-2 py-0.5 rounded bg-bg-tertiary border border-border text-text-muted font-mono">
						{((span.kind.input_tokens ?? 0) + (span.kind.output_tokens ?? 0)).toLocaleString()} tok
					</span>
					<span class="px-2 py-0.5 rounded bg-success/10 border border-success/20 text-success font-mono">
						${(span.kind.cost ?? 0).toFixed(4)}
					</span>
					<span class="px-2 py-0.5 rounded bg-bg-tertiary border border-border text-text-muted font-mono">
						{span.kind.input_tokens ?? 0} &rarr; {span.kind.output_tokens ?? 0}
					</span>
				</div>
			{:else if isFileSpan}
				<div class="flex items-center gap-1.5 flex-wrap text-[11px]">
					<span class="px-2 py-0.5 rounded bg-bg-tertiary border border-border text-text-secondary font-mono truncate max-w-80">{filePath}</span>
					{#if span.kind?.type === 'fs_read'}<span class="px-2 py-0.5 rounded bg-accent/10 border border-accent/20 text-accent">read</span><span class="px-2 py-0.5 rounded bg-bg-tertiary border border-border text-text-muted">{formatFileSize(span.kind.bytes_read)}</span>
					{:else if span.kind?.type === 'fs_write'}<span class="px-2 py-0.5 rounded bg-success/10 border border-success/20 text-success">write</span><span class="px-2 py-0.5 rounded bg-bg-tertiary border border-border text-text-muted">{formatFileSize(span.kind.bytes_written)}</span>{/if}
				</div>
			{/if}

			<div class="flex items-center gap-1.5 text-[11px] text-text-muted">
				<span class="font-mono">{shortId(span.id)}</span>
				<span>&middot;</span>
				<a href="/traces/{span.trace_id}" class="text-accent hover:underline font-mono">trace:{shortId(span.trace_id)}</a>
				{#if span.parent_id}<span>&middot;</span><span class="font-mono">parent:{shortId(span.parent_id)}</span>{/if}
				<span>&middot;</span>
				<span>{new Date(spanStartedAt(span)).toLocaleString()}</span>
			</div>
		</div>

		{#if error}
			<div class="px-4 py-2 bg-danger/10 border-b border-danger/20 text-danger text-xs font-mono shrink-0">{error}</div>
		{/if}

		<!-- Action bar -->
		<div class="flex items-center gap-1 px-3 py-1 border-b border-border/40 shrink-0 flex-wrap">
			<div class="relative">
				<button class="query-chip h-6 text-[11px] gap-1" onclick={openExportDropdown}>
					Add to dataset
					<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
				</button>
				{#if showExportDropdown}
					<div class="absolute left-0 top-full mt-1 w-56 bg-bg-secondary border border-border rounded-lg shadow-xl z-20 overflow-hidden">
						{#if exportDatasets.length === 0}
							<div class="px-3 py-2 text-xs text-text-muted">No datasets. Create one first.</div>
						{:else}
							{#each exportDatasets as ds (ds.id)}
								<button class="w-full text-left px-3 py-2 text-xs hover:bg-bg-tertiary transition-colors" disabled={exportLoading} onclick={() => doExport(ds.id)}>
									<div class="text-text">{ds.name}</div>
									<div class="text-text-muted text-[10px]">{ds.datapoint_count} datapoints</div>
								</button>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
			<div class="relative">
				<button class="query-chip h-6 text-[11px] gap-1" onclick={openReviewDropdown}>
					Send to review
					<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
				</button>
				{#if showReviewDropdown}
					<div class="absolute left-0 top-full mt-1 w-56 bg-bg-secondary border border-border rounded-lg shadow-xl z-20 overflow-hidden">
						{#if reviewDatasets.length === 0}
							<div class="px-3 py-2 text-xs text-text-muted">No datasets. Create one first.</div>
						{:else}
							{#each reviewDatasets as ds (ds.id)}
								<button class="w-full text-left px-3 py-2 text-xs hover:bg-bg-tertiary transition-colors" disabled={reviewLoading} onclick={() => doReview(ds.id)}>
									<div class="text-text">{ds.name}</div>
									<div class="text-text-muted text-[10px]">{ds.datapoint_count} datapoints</div>
								</button>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
			{#if span.kind?.type === 'llm_call'}
				<a href="/replay/{span.trace_id}?span={span.id}" class="query-chip h-6 text-[11px] gap-1 text-accent border-accent/30 hover:bg-accent/10 no-underline">
					<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M5.25 5.653c0-.856.917-1.398 1.667-.986l11.54 6.347a1.125 1.125 0 0 1 0 1.972l-11.54 6.347a1.125 1.125 0 0 1-1.667-.986V5.653Z" /></svg>
					Playground
				</a>
			{/if}
			{#if exportSuccess}<span class="text-[10px] text-success">{exportSuccess}</span>{/if}
			{#if reviewSuccess}<span class="text-[10px] text-success">{reviewSuccess}</span>{/if}
			<div class="flex-1"></div>
			{#if status === 'running'}
				<button class="query-chip h-6 text-[11px] text-success border-success/30 hover:bg-success/10" onclick={() => { showCompleteForm = !showCompleteForm; showFailForm = false; }}>Complete</button>
				<button class="query-chip h-6 text-[11px] text-danger border-danger/30 hover:bg-danger/10" onclick={() => { showFailForm = !showFailForm; showCompleteForm = false; }}>Fail</button>
			{/if}
			<button class="query-chip h-6 text-[11px] {confirmDeleteSpan ? 'text-danger border-danger/30' : ''}" onclick={handleDeleteSpan}>{confirmDeleteSpan ? 'Confirm?' : 'Delete'}</button>
		</div>

		<!-- Complete/Fail forms (expandable) -->
		{#if showCompleteForm}
			<form class="px-4 py-2 border-b border-border/55 bg-bg-tertiary/20 shrink-0 space-y-2 motion-rise-in" onsubmit={(e) => { e.preventDefault(); handleComplete(); }}>
				<label for="complete-output" class="block text-[10px] text-text-muted uppercase">Output (optional JSON)</label>
				<textarea id="complete-output" bind:value={completeOutput} rows={2} placeholder={'{"result": "success"}'} class="w-full bg-bg border border-border rounded px-2 py-1.5 text-xs text-text font-mono placeholder:text-text-muted"></textarea>
				<button type="submit" disabled={actionLoading} class="btn-primary h-7 text-xs">{actionLoading ? 'Completing...' : 'Complete Span'}</button>
			</form>
		{/if}
		{#if showFailForm}
			<form class="px-4 py-2 border-b border-border/55 bg-bg-tertiary/20 shrink-0 space-y-2 motion-rise-in" onsubmit={(e) => { e.preventDefault(); handleFail(); }}>
				<label for="fail-error" class="block text-[10px] text-text-muted uppercase">Error message</label>
				<input id="fail-error" type="text" bind:value={failError} placeholder="What went wrong?" class="w-full bg-bg border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted" />
				<button type="submit" disabled={actionLoading || !failError.trim()} class="px-3 py-1 text-xs bg-danger text-bg font-semibold rounded hover:bg-danger/80 transition-colors disabled:opacity-50">{actionLoading ? 'Failing...' : 'Fail Span'}</button>
			</form>
		{/if}

		<!-- Tabs -->
		<div class="flex items-center border-b border-border/40 shrink-0">
			<button class="px-3 py-1.5 text-[11px] font-medium border-b-2 transition-colors {activeTab === 'messages' ? 'border-accent text-text' : 'border-transparent text-text-muted hover:text-text-secondary'}" onclick={() => (activeTab = 'messages')}>
				{isFileSpan ? 'File' : 'Messages'}
			</button>
			<button class="px-3 py-1.5 text-[11px] font-medium border-b-2 transition-colors {activeTab === 'metadata' ? 'border-accent text-text' : 'border-transparent text-text-muted hover:text-text-secondary'}" onclick={() => (activeTab = 'metadata')}>
				Metadata
			</button>
			<div class="flex-1"></div>
			{#if activeTab === 'messages' && !isFileSpan}
				<div class="flex items-center gap-0 pr-2 text-[10px]">
					<button class="px-1.5 py-0.5 rounded transition-colors {formatMode === 'pretty' ? 'bg-bg-tertiary text-text border border-border' : 'text-text-muted hover:text-text-secondary'}" onclick={() => (formatMode = 'pretty')}>Pretty</button>
					<button class="px-1.5 py-0.5 rounded transition-colors {formatMode === 'json' ? 'bg-bg-tertiary text-text border border-border' : 'text-text-muted hover:text-text-secondary'}" onclick={() => (formatMode = 'json')}>JSON</button>
					<button class="px-1.5 py-0.5 rounded transition-colors {formatMode === 'yaml' ? 'bg-bg-tertiary text-text border border-border' : 'text-text-muted hover:text-text-secondary'}" onclick={() => (formatMode = 'yaml')}>YAML</button>
				</div>
			{/if}
		</div>

		<!-- Content -->
		<div class="flex-1 min-h-0 overflow-y-auto">
			{#if activeTab === 'messages'}
				{#if isFileSpan}
					<!-- File content view -->
					<div class="p-4 space-y-3">
						<div class="flex items-center justify-between glass-soft rounded-xl border border-border/50 px-3 py-2 text-xs">
							<div class="flex items-center gap-2 min-w-0">
								<svg class="w-4 h-4 shrink-0 {span?.kind?.type === 'fs_read' ? 'text-accent' : 'text-success'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" /></svg>
								<span class="font-mono text-text truncate">{filePath}</span>
							</div>
							{#if fileHash}<span class="font-mono text-accent text-[10px]">{fileHash.slice(0, 12)}</span>{/if}
						</div>
						{#if fileContentLoading}
							<div class="text-text-muted text-xs text-center py-8">Loading file content...</div>
						{:else if fileContentError}
							<div class="text-text-muted text-xs text-center py-8">{fileContentError}</div>
						{:else if fileContentLoaded}
							<div class="glass-soft border border-border/55 rounded-xl overflow-hidden">
								<div class="px-3 py-1.5 border-b border-border/40 text-[10px] text-text-muted flex items-center justify-between">
									<span>{inferLanguage(filePath ?? '')}</span>
									<button class="hover:text-text transition-colors" onclick={() => copyText(fileContent)}>Copy</button>
								</div>
								<pre class="p-3 text-xs font-mono text-text overflow-x-auto max-h-[60vh] overflow-y-auto whitespace-pre">{fileContent}</pre>
							</div>
						{:else}
							<div class="text-text-muted text-xs text-center py-8">No file content available</div>
						{/if}
					</div>
				{:else}
					<!-- Input section -->
					<div class="border-b border-border/40">
						<button class="flex items-center gap-2 px-4 py-2.5 w-full text-left text-[12px] text-text-muted hover:bg-bg-secondary/20 transition-colors" onclick={() => (inputExpanded = !inputExpanded)}>
							<svg class="w-3 h-3 transition-transform {inputExpanded ? '' : '-rotate-90'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
							Input
						</button>
						{#if inputExpanded}
							<div class="px-4 pb-3">
								{#if formatMode === 'pretty' && inputMessages}
									{#each inputMessages as msg, idx}
										<div class="flex gap-3 py-3 {idx > 0 ? 'border-t border-border/25' : ''}">
											<div class="w-16 shrink-0 pt-0.5">
												<span class="text-[12px] font-bold capitalize {roleColor(msg.role)}">{msg.role}</span>
											</div>
											<div class="flex-1 min-w-0">
												<div class="text-[11px] text-text-muted mb-1">{wordCount(msg.content)}w &middot; {charCount(msg.content)}c</div>
												<div class="text-[13px] text-text whitespace-pre-wrap break-words">{typeof msg.content === 'string' ? msg.content : JSON.stringify(msg.content, null, 2)}</div>
											</div>
											<div class="flex items-start gap-0.5 shrink-0">
												<button class="w-6 h-6 rounded flex items-center justify-center text-text-muted/40 hover:text-text-muted transition-colors" onclick={() => copyText(msg.content)} aria-label="Copy">
													<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9.75a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" /></svg>
												</button>
											</div>
										</div>
									{/each}
								{:else if span.input !== undefined && span.input !== null}
									<pre class="text-[12px] font-mono text-text whitespace-pre-wrap break-words">{renderValue(span.input)}</pre>
								{:else}
									<div class="text-[12px] text-text-muted py-4 text-center">No input data</div>
								{/if}
							</div>
						{/if}
					</div>

					<!-- Output section -->
					<div class="border-b border-border/40">
						<button class="flex items-center gap-2 px-4 py-2.5 w-full text-left text-[12px] text-text-muted hover:bg-bg-secondary/20 transition-colors" onclick={() => (outputExpanded = !outputExpanded)}>
							<svg class="w-3 h-3 transition-transform {outputExpanded ? '' : '-rotate-90'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
							Output
						</button>
						{#if outputExpanded}
							<div class="px-4 pb-3">
								{#if span.kind?.type === 'llm_call'}
									<div class="flex items-center gap-2 mb-3 text-[11px]">
										<span class="px-2 py-0.5 rounded bg-bg-tertiary border border-border text-text-secondary font-mono">Rerun {span.kind.model}</span>
									</div>
								{/if}
								{#if formatMode === 'pretty' && outputMessages}
									{#each outputMessages as msg, idx}
										<div class="flex gap-3 py-3 {idx > 0 ? 'border-t border-border/25' : ''}">
											<div class="w-16 shrink-0 pt-0.5">
												<span class="text-[12px] font-bold capitalize {roleColor(msg.role)}">{msg.role}</span>
											</div>
											<div class="flex-1 min-w-0">
												<div class="text-[11px] text-text-muted mb-1">{wordCount(msg.content)}w &middot; {charCount(msg.content)}c</div>
												<div class="text-[13px] text-text whitespace-pre-wrap break-words">{typeof msg.content === 'string' ? msg.content : JSON.stringify(msg.content, null, 2)}</div>
											</div>
											<div class="flex items-start gap-0.5 shrink-0">
												<button class="w-6 h-6 rounded flex items-center justify-center text-text-muted/40 hover:text-text-muted transition-colors" onclick={() => copyText(msg.content)} aria-label="Copy">
													<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9.75a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184" /></svg>
												</button>
											</div>
										</div>
									{/each}
								{:else if span.output !== undefined && span.output !== null}
									<pre class="text-[12px] font-mono text-text whitespace-pre-wrap break-words">{renderValue(span.output)}</pre>
								{:else}
									<div class="text-[12px] text-text-muted py-4 text-center">No output data</div>
								{/if}
							</div>
						{/if}
					</div>

					<!-- Model footer -->
					{#if span.kind?.type === 'llm_call'}
						<div class="px-4 py-3 flex items-center gap-2 text-[12px] bg-bg-secondary/10">
							<span class="text-text-secondary font-mono">{span.kind.model}</span>
							<span class="text-text-muted">&middot;</span>
							<span class="text-text-muted font-mono">{formatDuration(duration)}</span>
						</div>
					{/if}
				{/if}

			{:else}
				<!-- Metadata tab -->
				<div class="p-4 space-y-3">
					<div class="glass-soft rounded-xl border border-border/55 p-3">
						<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">Identifiers</div>
						<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
							<div><span class="text-text-muted">Span ID</span><div class="text-text font-mono text-[11px]">{span.id}</div></div>
							<div><span class="text-text-muted">Trace ID</span><div class="font-mono text-[11px]"><a href="/traces/{span.trace_id}" class="text-accent hover:underline">{span.trace_id}</a></div></div>
							{#if span.parent_id}<div><span class="text-text-muted">Parent ID</span><div class="text-text font-mono text-[11px]">{span.parent_id}</div></div>{/if}
						</div>
					</div>

					<div class="glass-soft rounded-xl border border-border/55 p-3">
						<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">Timing</div>
						<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
							<div><span class="text-text-muted">Started</span><div class="text-text font-mono text-[11px]">{new Date(spanStartedAt(span)).toLocaleString()}</div></div>
							{#if spanEndedAt(span)}<div><span class="text-text-muted">Ended</span><div class="text-text font-mono text-[11px]">{new Date(spanEndedAt(span)!).toLocaleString()}</div></div>{/if}
							{#if duration !== null}<div><span class="text-text-muted">Duration</span><div class="text-text font-mono text-[11px]">{formatDuration(duration)}</div></div>{/if}
						</div>
					</div>

					{#if Object.keys(meta).length > 0}
						<div class="glass-soft rounded-xl border border-border/55 p-3">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">{spanKindLabel(span)}</div>
							<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
								{#each Object.entries(meta) as [key, value]}
									<div><span class="text-text-muted">{key}</span><div class="text-text font-mono text-[11px] break-all">{value}</div></div>
								{/each}
							</div>
						</div>
					{/if}

					{#if span.kind?.type === 'custom' && span.kind.attributes && Object.keys(span.kind.attributes).length > 0}
						<div class="glass-soft rounded-xl border border-border/55 p-3">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">Custom Attributes</div>
							<pre class="text-xs text-text bg-bg-tertiary/55 border border-border/45 rounded-lg p-3 overflow-x-auto whitespace-pre-wrap font-mono">{JSON.stringify(span.kind.attributes, null, 2)}</pre>
						</div>
					{/if}

					{#if childSpans.length > 0}
						<div class="glass-soft rounded-xl border border-border/55 p-3">
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">Children ({childSpans.length})</div>
							<div class="space-y-1">
								{#each childSpans as child (child.id)}
									{@const cStatus = spanStatus(child)}
									{@const cDuration = spanDurationMs(child)}
									<div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-bg-tertiary/60 border border-border/45 text-xs">
										<SpanKindIcon span={child} />
										<span class="text-text font-medium truncate flex-1">{child.name}</span>
										{#if child.kind?.type === 'llm_call'}<span class="text-accent text-[10px]">{child.kind.model}</span>{/if}
										<StatusBadge status={cStatus} />
										<span class="text-text-muted font-mono text-[11px]">{formatDuration(cDuration)}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}

					</div>
			{/if}
		</div>
	</div>
{:else}
	<div class="flex items-center justify-center h-full text-text-muted text-sm">
		Select a span to view details
	</div>
{/if}
