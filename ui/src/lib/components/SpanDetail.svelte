<script lang="ts">
	import type { Span, DatasetWithCount } from '$lib/api';
	import { spanStatus, spanStartedAt, spanEndedAt, spanDurationMs, spanError, spanKindLabel, shortId, getDatasets, exportSpanToDataset, completeSpan, failSpan } from '$lib/api';
	import StatusBadge from './StatusBadge.svelte';
	import SpanKindIcon from './SpanKindIcon.svelte';

	let { span, onSpanAction }: { span: Span | null; onSpanAction?: () => void } = $props();

	let activePayloadTab: 'input' | 'output' = $state('input');

	// Export to dataset
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

	// Complete / Fail actions
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

	function formatJson(value: unknown): string {
		if (value === null || value === undefined) return '(none)';
		if (typeof value === 'string') return value;
		return JSON.stringify(value, null, 2);
	}

	function kindMeta(s: Span): Record<string, string> {
		if (!s.kind) return {};
		if ('FsRead' in s.kind) {
			return {
				'Path': s.kind.FsRead.path,
				'Version': s.kind.FsRead.file_version ?? '-',
				'Bytes': s.kind.FsRead.bytes_read.toLocaleString(),
			};
		}
		if ('FsWrite' in s.kind) {
			return {
				'Path': s.kind.FsWrite.path,
				'Version': s.kind.FsWrite.file_version,
				'Bytes': s.kind.FsWrite.bytes_written.toLocaleString(),
			};
		}
		if ('LlmCall' in s.kind) {
			const k = s.kind.LlmCall;
			const meta: Record<string, string> = { 'Model': k.model };
			if (k.provider) meta['Provider'] = k.provider;
			if (k.input_tokens != null) meta['Input tokens'] = k.input_tokens.toLocaleString();
			if (k.output_tokens != null) meta['Output tokens'] = k.output_tokens.toLocaleString();
			return meta;
		}
		if ('Custom' in s.kind) {
			return { 'Kind': s.kind.Custom.kind };
		}
		return {};
	}
</script>

{#if span}
	{@const meta = kindMeta(span)}
	<div class="border border-border rounded bg-bg-secondary p-4 space-y-3 text-sm">
		<div class="flex items-center gap-2">
			<SpanKindIcon {span} />
			<h3 class="text-text font-semibold text-base flex-1">{span.name}</h3>
			<!-- Export to Dataset -->
			<div class="relative">
				<button
					class="px-2 py-1 text-xs bg-amber-400/10 text-amber-400 border border-amber-400/20 rounded hover:bg-amber-400/20 transition-colors"
					onclick={openExportDropdown}
				>Export to Dataset</button>
				{#if showExportDropdown}
					<div class="absolute right-0 top-full mt-1 w-56 bg-bg-secondary border border-border rounded shadow-lg z-10">
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
				<span class="text-xs text-success">{exportSuccess}</span>
			{/if}
			<StatusBadge status={spanStatus(span)} />
		</div>

		<!-- Complete / Fail actions for running spans -->
		{#if spanStatus(span) === 'running'}
			<div class="flex items-center gap-2">
				<button
					class="px-3 py-1 text-xs bg-success/10 text-success border border-success/20 rounded hover:bg-success/20 transition-colors"
					onclick={() => { showCompleteForm = !showCompleteForm; showFailForm = false; }}
				>Complete</button>
				<button
					class="px-3 py-1 text-xs bg-danger/10 text-danger border border-danger/20 rounded hover:bg-danger/20 transition-colors"
					onclick={() => { showFailForm = !showFailForm; showCompleteForm = false; }}
				>Fail</button>
			</div>

			{#if showCompleteForm}
				<form class="bg-bg-tertiary rounded p-3 space-y-2" onsubmit={(e) => { e.preventDefault(); handleComplete(); }}>
					<label for="complete-output" class="block text-xs text-text-muted">Output (optional JSON)</label>
					<textarea
						id="complete-output"
						bind:value={completeOutput}
						rows={3}
						placeholder={'{"result": "success"}'}
						class="w-full bg-bg border border-border rounded px-2 py-1.5 text-xs text-text font-mono placeholder:text-text-muted"
					></textarea>
					<button
						type="submit"
						disabled={actionLoading}
						class="px-3 py-1 text-xs bg-success text-bg font-semibold rounded hover:bg-success/80 transition-colors disabled:opacity-50"
					>{actionLoading ? 'Completing...' : 'Complete Span'}</button>
				</form>
			{/if}

			{#if showFailForm}
				<form class="bg-bg-tertiary rounded p-3 space-y-2" onsubmit={(e) => { e.preventDefault(); handleFail(); }}>
					<label for="fail-error" class="block text-xs text-text-muted">Error message</label>
					<input
						id="fail-error"
						type="text"
						bind:value={failError}
						placeholder="What went wrong?"
						class="w-full bg-bg border border-border rounded px-2 py-1.5 text-xs text-text placeholder:text-text-muted"
					/>
					<button
						type="submit"
						disabled={actionLoading || !failError.trim()}
						class="px-3 py-1 text-xs bg-danger text-bg font-semibold rounded hover:bg-danger/80 transition-colors disabled:opacity-50"
					>{actionLoading ? 'Failing...' : 'Fail Span'}</button>
				</form>
			{/if}
		{/if}

		<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-text-secondary">
			<div>
				<span class="text-text-muted text-xs uppercase">Span ID</span>
				<div class="font-mono text-xs">{shortId(span.id)}</div>
			</div>
			<div>
				<span class="text-text-muted text-xs uppercase">Trace ID</span>
				<div class="font-mono text-xs">
					<a href="/traces/{span.trace_id}" class="text-accent hover:underline">{shortId(span.trace_id)}</a>
				</div>
			</div>
			{#if span.parent_id}
				<div>
					<span class="text-text-muted text-xs uppercase">Parent ID</span>
					<div class="font-mono text-xs">{shortId(span.parent_id)}</div>
				</div>
			{/if}
			<div>
				<span class="text-text-muted text-xs uppercase">Started</span>
				<div class="font-mono text-xs">{new Date(spanStartedAt(span)).toLocaleTimeString()}</div>
			</div>
			{#if spanEndedAt(span)}
				<div>
					<span class="text-text-muted text-xs uppercase">Ended</span>
					<div class="font-mono text-xs">{new Date(spanEndedAt(span)!).toLocaleTimeString()}</div>
				</div>
			{/if}
			{#if spanDurationMs(span) !== null}
				<div>
					<span class="text-text-muted text-xs uppercase">Duration</span>
					<div class="font-mono text-xs">{spanDurationMs(span)}ms</div>
				</div>
			{/if}
		</div>

		<!-- SpanKind metadata -->
		{#if Object.keys(meta).length > 0}
			<div class="border-t border-border pt-2 space-y-1">
				<div class="text-text-muted text-xs uppercase">
					{spanKindLabel(span) ?? 'Metadata'}
				</div>
				<div class="grid grid-cols-2 gap-x-6 gap-y-1 text-xs">
					{#each Object.entries(meta) as [key, value]}
						<div>
							<span class="text-text-muted">{key}</span>
							<div class="text-text font-mono">{value}</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Legacy metadata (backward compat) -->
		{#if !span.kind && (span.metadata.model || span.metadata.input_tokens || span.metadata.output_tokens)}
			<div class="border-t border-border pt-2 space-y-1">
				<div class="text-text-muted text-xs uppercase">Metadata</div>
				<div class="grid grid-cols-3 gap-2 text-xs">
					{#if span.metadata.model}
						<div>
							<span class="text-text-muted">Model</span>
							<div class="text-accent">{span.metadata.model}</div>
						</div>
					{/if}
					{#if span.metadata.input_tokens}
						<div>
							<span class="text-text-muted">In tokens</span>
							<div>{span.metadata.input_tokens.toLocaleString()}</div>
						</div>
					{/if}
					{#if span.metadata.output_tokens}
						<div>
							<span class="text-text-muted">Out tokens</span>
							<div>{span.metadata.output_tokens.toLocaleString()}</div>
						</div>
					{/if}
				</div>
			</div>
		{/if}

		<!-- Input / Output payloads -->
		{#if span.input !== undefined || span.output !== undefined}
			<div class="border-t border-border pt-2 space-y-2">
				<div class="flex gap-0 border-b border-border">
					<button
						class="px-3 py-1 text-xs transition-colors border-b-2
							{activePayloadTab === 'input' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
						onclick={() => activePayloadTab = 'input'}
					>Input</button>
					<button
						class="px-3 py-1 text-xs transition-colors border-b-2
							{activePayloadTab === 'output' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
						onclick={() => activePayloadTab = 'output'}
					>Output</button>
				</div>
				<pre class="text-xs text-text bg-bg-tertiary rounded p-3 overflow-x-auto max-h-64 overflow-y-auto whitespace-pre-wrap font-mono">{activePayloadTab === 'input' ? formatJson(span.input) : formatJson(span.output)}</pre>
			</div>
		{/if}

		{#if spanError(span)}
			<div class="border-t border-border pt-2">
				<div class="text-text-muted text-xs uppercase">Error</div>
				<pre class="text-danger text-xs mt-1 whitespace-pre-wrap">{spanError(span)}</pre>
			</div>
		{/if}
	</div>
{:else}
	<div class="border border-border rounded bg-bg-secondary p-4 text-text-muted text-sm text-center">
		Select a span to view details
	</div>
{/if}
