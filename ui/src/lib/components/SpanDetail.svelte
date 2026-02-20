<script lang="ts">
	import type { Span, DatasetWithCount } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs, spanError, spanKindLabel, shortId, getDatasets, exportSpanToDataset, completeSpan, failSpan, deleteSpan } from '$lib/api';
	import StatusBadge from './StatusBadge.svelte';
	import SpanKindIcon from './SpanKindIcon.svelte';

	let { span, onSpanAction, allSpans = [] }: { span: Span | null; onSpanAction?: () => void; allSpans?: Span[] } = $props();

	// ── Tabs ──────────────────────────────────────────────────────────
	let activeTab: 'input' | 'output' | 'attributes' | 'events' = $state('input');

	// ── JSON/Text toggle for payloads ─────────────────────────────────
	let inputViewMode: 'formatted' | 'raw' = $state('formatted');
	let outputViewMode: 'formatted' | 'raw' = $state('formatted');

	// ── Export to dataset ─────────────────────────────────────────────
	let showExportDropdown = $state(false);
	let exportDatasets: DatasetWithCount[] = $state([]);
	let exportLoading = $state(false);
	let exportSuccess = $state('');

	async function openExportDropdown() {
		showExportDropdown = !showExportDropdown;
		if (showExportDropdown) {
			try {
				const result = await getDatasets();
				exportDatasets = result.datasets;
			} catch {
				exportDatasets = [];
			}
		}
	}

	async function doExport(datasetId: string) {
		if (!span) return;
		exportLoading = true;
		exportSuccess = '';
		try {
			await exportSpanToDataset(datasetId, span.id);
			const ds = exportDatasets.find((d) => d.id === datasetId);
			exportSuccess = `Exported to ${ds?.name ?? shortId(datasetId)}`;
			showExportDropdown = false;
		} catch {
			exportSuccess = 'Export failed';
		}
		exportLoading = false;
		setTimeout(() => (exportSuccess = ''), 3000);
	}

	// ── Delete span ───────────────────────────────────────────────────
	let confirmDeleteSpan = $state(false);

	async function handleDeleteSpan() {
		if (!span) return;
		if (!confirmDeleteSpan) {
			confirmDeleteSpan = true;
			setTimeout(() => (confirmDeleteSpan = false), 3000);
			return;
		}
		try {
			await deleteSpan(span.id);
			onSpanAction?.();
		} catch {
			// error
		}
		confirmDeleteSpan = false;
	}

	// ── Complete / Fail actions ───────────────────────────────────────
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
			showCompleteForm = false;
			completeOutput = '';
			onSpanAction?.();
		} catch {
			// error
		}
		actionLoading = false;
	}

	async function handleFail() {
		if (!span || !failError.trim()) return;
		actionLoading = true;
		try {
			await failSpan(span.id, failError.trim());
			showFailForm = false;
			failError = '';
			onSpanAction?.();
		} catch {
			// error
		}
		actionLoading = false;
	}

	// ── Formatting helpers ────────────────────────────────────────────
	function formatJson(value: unknown): string {
		if (value === null || value === undefined) return '(none)';
		if (typeof value === 'string') return value;
		return JSON.stringify(value, null, 2);
	}

	function formatJsonRaw(value: unknown): string {
		if (value === null || value === undefined) return '(none)';
		return JSON.stringify(value);
	}

	function kindMeta(s: Span): Record<string, string> {
		if (!s.kind) return {};
		switch (s.kind.type) {
			case 'fs_read':
				return {
					'Path': s.kind.path,
					'Version': s.kind.file_version ?? '-',
					'Bytes': s.kind.bytes_read.toLocaleString(),
				};
			case 'fs_write':
				return {
					'Path': s.kind.path,
					'Version': s.kind.file_version,
					'Bytes': s.kind.bytes_written.toLocaleString(),
				};
			case 'llm_call': {
				const meta: Record<string, string> = { 'Model': s.kind.model };
				if (s.kind.provider) meta['Provider'] = s.kind.provider;
				if (s.kind.input_tokens != null) meta['Input tokens'] = s.kind.input_tokens.toLocaleString();
				if (s.kind.output_tokens != null) meta['Output tokens'] = s.kind.output_tokens.toLocaleString();
				if (s.kind.cost != null) meta['Cost'] = `$${s.kind.cost.toFixed(6)}`;
				return meta;
			}
			case 'custom':
				return { 'Kind': s.kind.kind };
			default:
				return {};
		}
	}

	// ── LLM message detection ─────────────────────────────────────────
	// Detect if input/output contains chat messages (array of {role, content})
	interface ChatMessage {
		role: string;
		content: string;
	}

	function extractMessages(value: unknown): ChatMessage[] | null {
		if (!value) return null;
		// Direct array of messages
		if (Array.isArray(value)) {
			if (value.length > 0 && value[0].role && value[0].content !== undefined) {
				return value as ChatMessage[];
			}
		}
		// Object with messages array
		if (typeof value === 'object' && value !== null) {
			const obj = value as Record<string, unknown>;
			if (Array.isArray(obj.messages) && obj.messages.length > 0 && (obj.messages[0] as Record<string, unknown>).role) {
				return obj.messages as ChatMessage[];
			}
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

	function roleBgColor(role: string): string {
		switch (role.toLowerCase()) {
			case 'system': return 'bg-warning/5 border-warning/20';
			case 'user': return 'bg-accent/5 border-accent/20';
			case 'assistant': return 'bg-success/5 border-success/20';
			case 'tool': return 'bg-purple-400/5 border-purple-400/20';
			default: return 'bg-bg-tertiary border-border';
		}
	}

	// ── Derived values ────────────────────────────────────────────────
	const inputMessages = $derived(span ? extractMessages(span.input) : null);
	const outputMessages = $derived(span ? extractMessages(span.output) : null);

	// Children of current span
	const childSpans = $derived(
		span ? allSpans.filter((s) => s.parent_id === span.id) : []
	);

	// Collapsed state for individual messages
	let collapsedMessages: Set<number> = $state(new Set());

	function toggleMessage(idx: number) {
		const next = new Set(collapsedMessages);
		if (next.has(idx)) next.delete(idx);
		else next.add(idx);
		collapsedMessages = next;
	}

	// Reset tab and collapsed state when span changes
	$effect(() => {
		if (span) {
			collapsedMessages = new Set();
			// Auto-select best tab
			if (span.input !== undefined && span.input !== null) {
				activeTab = 'input';
			} else if (span.output !== undefined && span.output !== null) {
				activeTab = 'output';
			} else {
				activeTab = 'attributes';
			}
		}
	});

	function formatDuration(ms: number | null): string {
		if (ms === null) return '...';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}
</script>

{#if span}
	{@const meta = kindMeta(span)}
	{@const status = spanStatus(span)}
	{@const duration = spanDurationMs(span)}
	{@const error = spanError(span)}

	<div class="flex flex-col h-full min-h-0">
		<!-- Header -->
		<div class="px-4 py-3 border-b border-border shrink-0 space-y-2">
			<!-- Row 1: name + status -->
			<div class="flex items-center gap-2">
				<SpanKindIcon {span} />
				<h2 class="text-text font-semibold text-base flex-1 truncate">{span.name}</h2>
				<StatusBadge {status} />
			</div>

			<!-- Row 2: metadata badges -->
			<div class="flex items-center gap-2 flex-wrap">
				<!-- Duration badge -->
				<span class="inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] bg-bg-tertiary border border-border text-text-secondary">
					<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
					{formatDuration(duration)}
				</span>

				<!-- Started at -->
				<span class="text-[11px] text-text-muted font-mono">
					{new Date(spanStartedAt(span)).toLocaleTimeString()}
				</span>

				<!-- Span ID -->
				<span class="text-[11px] text-text-muted font-mono">
					{shortId(span.id)}
				</span>

				<!-- Kind-specific badges -->
				{#if span.kind?.type === 'llm_call'}
					<span class="inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] bg-purple-400/10 border border-purple-400/20 text-purple-400">
						{span.kind.model}
					</span>
					{#if span.kind.input_tokens != null || span.kind.output_tokens != null}
						<span class="inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] bg-bg-tertiary border border-border text-text-secondary">
							{#if span.kind.input_tokens != null}{span.kind.input_tokens.toLocaleString()} in{/if}
							{#if span.kind.input_tokens != null && span.kind.output_tokens != null} / {/if}
							{#if span.kind.output_tokens != null}{span.kind.output_tokens.toLocaleString()} out{/if}
						</span>
					{/if}
					{#if span.kind.cost != null}
						<span class="inline-flex items-center rounded px-2 py-0.5 text-[11px] bg-success/10 border border-success/20 text-success">
							${span.kind.cost.toFixed(4)}
						</span>
					{/if}
					{#if span.kind.provider}
						<span class="text-[11px] text-text-muted">{span.kind.provider}</span>
					{/if}
				{:else if span.kind?.type === 'fs_read' || span.kind?.type === 'fs_write'}
					<span class="inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] bg-bg-tertiary border border-border text-text-secondary font-mono truncate max-w-64">
						{span.kind.path}
					</span>
				{/if}
			</div>

			<!-- Row 3: action buttons -->
			<div class="flex items-center gap-2">
				{#if status === 'running'}
					<button
						class="px-2.5 py-1 text-[11px] bg-success/10 text-success border border-success/20 rounded hover:bg-success/20 transition-colors"
						onclick={() => { showCompleteForm = !showCompleteForm; showFailForm = false; }}
					>Complete</button>
					<button
						class="px-2.5 py-1 text-[11px] bg-danger/10 text-danger border border-danger/20 rounded hover:bg-danger/20 transition-colors"
						onclick={() => { showFailForm = !showFailForm; showCompleteForm = false; }}
					>Fail</button>
				{/if}

				<!-- Export to Dataset -->
				<div class="relative">
					<button
						class="px-2.5 py-1 text-[11px] bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors"
						onclick={openExportDropdown}
					>Add to dataset</button>
					{#if showExportDropdown}
						<div class="absolute left-0 top-full mt-1 w-56 bg-bg-secondary border border-border rounded shadow-lg z-10">
							{#if exportDatasets.length === 0}
								<div class="px-3 py-2 text-xs text-text-muted">No datasets. Create one first.</div>
							{:else}
								{#each exportDatasets as ds (ds.id)}
									<button
										class="w-full text-left px-3 py-2 text-xs hover:bg-bg-tertiary transition-colors text-text-secondary"
										disabled={exportLoading}
										onclick={() => doExport(ds.id)}
									>
										<div class="text-text">{ds.name}</div>
										<div class="text-text-muted">{ds.datapoint_count} datapoints</div>
									</button>
								{/each}
							{/if}
						</div>
					{/if}
				</div>

				{#if exportSuccess}
					<span class="text-[11px] text-success">{exportSuccess}</span>
				{/if}

				<div class="flex-1"></div>

				<button
					class="px-2.5 py-1 text-[11px] transition-colors border rounded {confirmDeleteSpan ? 'bg-danger/10 text-danger border-danger/30 font-semibold' : 'text-text-muted border-border hover:text-danger hover:border-danger/30'}"
					onclick={handleDeleteSpan}
				>{confirmDeleteSpan ? 'Confirm?' : 'Delete span'}</button>
			</div>

			<!-- Complete/Fail forms -->
			{#if showCompleteForm}
				<form class="bg-bg-tertiary rounded p-3 space-y-2" onsubmit={(e) => { e.preventDefault(); handleComplete(); }}>
					<label for="complete-output" class="block text-xs text-text-muted">Output (optional JSON)</label>
					<textarea id="complete-output" bind:value={completeOutput} rows={2} placeholder={'{"result": "success"}'}
						class="w-full bg-bg border border-border rounded px-2 py-1.5 text-xs text-text font-mono placeholder:text-text-muted"></textarea>
					<button type="submit" disabled={actionLoading}
						class="px-3 py-1 text-xs bg-success text-bg font-semibold rounded hover:bg-success/80 transition-colors disabled:opacity-50">
						{actionLoading ? 'Completing...' : 'Complete Span'}
					</button>
				</form>
			{/if}

			{#if showFailForm}
				<form class="bg-bg-tertiary rounded p-3 space-y-2" onsubmit={(e) => { e.preventDefault(); handleFail(); }}>
					<label for="fail-error" class="block text-xs text-text-muted">Error message</label>
					<input id="fail-error" type="text" bind:value={failError} placeholder="What went wrong?"
						class="w-full bg-bg border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted" />
					<button type="submit" disabled={actionLoading || !failError.trim()}
						class="px-3 py-1 text-xs bg-danger text-bg font-semibold rounded hover:bg-danger/80 transition-colors disabled:opacity-50">
						{actionLoading ? 'Failing...' : 'Fail Span'}
					</button>
				</form>
			{/if}
		</div>

		<!-- Error banner -->
		{#if error}
			<div class="px-4 py-2 bg-danger/10 border-b border-danger/20 text-danger text-xs font-mono shrink-0">
				{error}
			</div>
		{/if}

		<!-- Tabs -->
		<div class="flex border-b border-border shrink-0">
			<button
				class="px-4 py-2 text-xs transition-colors border-b-2
					{activeTab === 'input' ? 'border-accent text-text font-medium' : 'border-transparent text-text-muted hover:text-text-secondary'}"
				onclick={() => activeTab = 'input'}
			>Input</button>
			<button
				class="px-4 py-2 text-xs transition-colors border-b-2
					{activeTab === 'output' ? 'border-accent text-text font-medium' : 'border-transparent text-text-muted hover:text-text-secondary'}"
				onclick={() => activeTab = 'output'}
			>Output</button>
			<button
				class="px-4 py-2 text-xs transition-colors border-b-2
					{activeTab === 'attributes' ? 'border-accent text-text font-medium' : 'border-transparent text-text-muted hover:text-text-secondary'}"
				onclick={() => activeTab = 'attributes'}
			>Attributes</button>
			{#if childSpans.length > 0}
				<button
					class="px-4 py-2 text-xs transition-colors border-b-2
						{activeTab === 'events' ? 'border-accent text-text font-medium' : 'border-transparent text-text-muted hover:text-text-secondary'}"
					onclick={() => activeTab = 'events'}
				>Children <span class="text-text-muted">({childSpans.length})</span></button>
			{/if}
		</div>

		<!-- Tab content -->
		<div class="flex-1 min-h-0 overflow-y-auto">
			{#if activeTab === 'input'}
				<div class="p-4">
					{#if inputMessages}
						<!-- Chat message view -->
						<div class="space-y-2">
							{#each inputMessages as msg, idx}
								<div class="border rounded {roleBgColor(msg.role)}">
									<button
										class="w-full flex items-center gap-2 px-3 py-1.5 text-left"
										onclick={() => toggleMessage(idx)}
									>
										<span class="text-[10px] font-bold uppercase tracking-wider {roleColor(msg.role)}">{msg.role}</span>
										<div class="flex-1"></div>
										<span class="text-[10px] text-text-muted">{typeof msg.content === 'string' ? msg.content.length : 0} chars</span>
										<svg class="w-3 h-3 text-text-muted transition-transform {collapsedMessages.has(idx) ? '' : 'rotate-180'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
									</button>
									{#if !collapsedMessages.has(idx)}
										<div class="px-3 pb-3 text-xs text-text font-mono whitespace-pre-wrap break-words border-t border-inherit">
											<div class="pt-2">{typeof msg.content === 'string' ? msg.content : JSON.stringify(msg.content, null, 2)}</div>
										</div>
									{/if}
								</div>
							{/each}
						</div>
					{:else if span.input !== undefined && span.input !== null}
						<!-- Raw JSON view -->
						<div class="flex items-center justify-between mb-2">
							<span class="text-[10px] text-text-muted uppercase tracking-wider">Span Input</span>
							<div class="flex items-center bg-bg-tertiary rounded text-[10px]">
								<button
									class="px-2 py-0.5 rounded transition-colors {inputViewMode === 'formatted' ? 'bg-accent/20 text-accent' : 'text-text-muted hover:text-text'}"
									onclick={() => inputViewMode = 'formatted'}
								>Formatted</button>
								<button
									class="px-2 py-0.5 rounded transition-colors {inputViewMode === 'raw' ? 'bg-accent/20 text-accent' : 'text-text-muted hover:text-text'}"
									onclick={() => inputViewMode = 'raw'}
								>Raw</button>
							</div>
						</div>
						<pre class="text-xs text-text bg-bg-tertiary rounded p-3 overflow-x-auto whitespace-pre-wrap font-mono break-words">{inputViewMode === 'formatted' ? formatJson(span.input) : formatJsonRaw(span.input)}</pre>
					{:else}
						<div class="text-text-muted text-xs text-center py-8">No input data</div>
					{/if}
				</div>

			{:else if activeTab === 'output'}
				<div class="p-4">
					{#if outputMessages}
						<!-- Chat message view -->
						<div class="space-y-2">
							{#each outputMessages as msg, idx}
								{@const msgIdx = 1000 + idx}
								<div class="border rounded {roleBgColor(msg.role)}">
									<button
										class="w-full flex items-center gap-2 px-3 py-1.5 text-left"
										onclick={() => toggleMessage(msgIdx)}
									>
										<span class="text-[10px] font-bold uppercase tracking-wider {roleColor(msg.role)}">{msg.role}</span>
										<div class="flex-1"></div>
										<span class="text-[10px] text-text-muted">{typeof msg.content === 'string' ? msg.content.length : 0} chars</span>
										<svg class="w-3 h-3 text-text-muted transition-transform {collapsedMessages.has(msgIdx) ? '' : 'rotate-180'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
									</button>
									{#if !collapsedMessages.has(msgIdx)}
										<div class="px-3 pb-3 text-xs text-text font-mono whitespace-pre-wrap break-words border-t border-inherit">
											<div class="pt-2">{typeof msg.content === 'string' ? msg.content : JSON.stringify(msg.content, null, 2)}</div>
										</div>
									{/if}
								</div>
							{/each}
						</div>
					{:else if span.output !== undefined && span.output !== null}
						<!-- Raw JSON view -->
						<div class="flex items-center justify-between mb-2">
							<span class="text-[10px] text-text-muted uppercase tracking-wider">Span Output</span>
							<div class="flex items-center bg-bg-tertiary rounded text-[10px]">
								<button
									class="px-2 py-0.5 rounded transition-colors {outputViewMode === 'formatted' ? 'bg-accent/20 text-accent' : 'text-text-muted hover:text-text'}"
									onclick={() => outputViewMode = 'formatted'}
								>Formatted</button>
								<button
									class="px-2 py-0.5 rounded transition-colors {outputViewMode === 'raw' ? 'bg-accent/20 text-accent' : 'text-text-muted hover:text-text'}"
									onclick={() => outputViewMode = 'raw'}
								>Raw</button>
							</div>
						</div>
						<pre class="text-xs text-text bg-bg-tertiary rounded p-3 overflow-x-auto whitespace-pre-wrap font-mono break-words">{outputViewMode === 'formatted' ? formatJson(span.output) : formatJsonRaw(span.output)}</pre>
					{:else}
						<div class="text-text-muted text-xs text-center py-8">No output data</div>
					{/if}
				</div>

			{:else if activeTab === 'attributes'}
				<div class="p-4 space-y-4">
					<!-- Span identifiers -->
					<div>
						<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">Identifiers</div>
						<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
							<div>
								<span class="text-text-muted">Span ID</span>
								<div class="text-text font-mono text-[11px]">{span.id}</div>
							</div>
							<div>
								<span class="text-text-muted">Trace ID</span>
								<div class="font-mono text-[11px]">
									<a href="/traces/{span.trace_id}" class="text-accent hover:underline">{span.trace_id}</a>
								</div>
							</div>
							{#if span.parent_id}
								<div>
									<span class="text-text-muted">Parent ID</span>
									<div class="text-text font-mono text-[11px]">{span.parent_id}</div>
								</div>
							{/if}
						</div>
					</div>

					<!-- Timing -->
					<div>
						<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">Timing</div>
						<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
							<div>
								<span class="text-text-muted">Started</span>
								<div class="text-text font-mono text-[11px]">{new Date(spanStartedAt(span)).toLocaleString()}</div>
							</div>
							{#if spanEndedAt(span)}
								<div>
									<span class="text-text-muted">Ended</span>
									<div class="text-text font-mono text-[11px]">{new Date(spanEndedAt(span)!).toLocaleString()}</div>
								</div>
							{/if}
							{#if duration !== null}
								<div>
									<span class="text-text-muted">Duration</span>
									<div class="text-text font-mono text-[11px]">{formatDuration(duration)}</div>
								</div>
							{/if}
						</div>
					</div>

					<!-- Kind metadata -->
					{#if Object.keys(meta).length > 0}
						<div>
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">{spanKindLabel(span)}</div>
							<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
								{#each Object.entries(meta) as [key, value]}
									<div>
										<span class="text-text-muted">{key}</span>
										<div class="text-text font-mono text-[11px] break-all">{value}</div>
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<!-- Custom kind attributes -->
					{#if span.kind?.type === 'custom' && span.kind.attributes && Object.keys(span.kind.attributes).length > 0}
						<div>
							<div class="text-[10px] text-text-muted uppercase tracking-wider mb-2">Custom Attributes</div>
							<pre class="text-xs text-text bg-bg-tertiary rounded p-3 overflow-x-auto whitespace-pre-wrap font-mono">{JSON.stringify(span.kind.attributes, null, 2)}</pre>
						</div>
					{/if}
				</div>

			{:else if activeTab === 'events'}
				<div class="p-4">
					{#if childSpans.length > 0}
						<div class="space-y-1">
							{#each childSpans as child (child.id)}
								{@const cStatus = spanStatus(child)}
								{@const cDuration = spanDurationMs(child)}
								<div class="flex items-center gap-2 px-3 py-2 rounded bg-bg-tertiary text-xs">
									<SpanKindIcon span={child} />
									<span class="text-text font-medium truncate flex-1">{child.name}</span>
									{#if child.kind?.type === 'llm_call'}
										<span class="text-purple-400 text-[10px]">{child.kind.model}</span>
									{/if}
									<StatusBadge status={cStatus} />
									<span class="text-text-muted font-mono text-[11px]">{formatDuration(cDuration)}</span>
								</div>
							{/each}
						</div>
					{:else}
						<div class="text-text-muted text-xs text-center py-8">No child spans</div>
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
