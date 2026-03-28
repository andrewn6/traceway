<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { shortId } from '$lib/api';
	import { tick } from 'svelte';

	interface PageParams {
		traceId?: string;
	}

	interface LlmCallReplay {
		span_id: string;
		name: string;
		model: string;
		provider: string | null;
		input: unknown;
		output: unknown;
		input_tokens: number | null;
		output_tokens: number | null;
		cost: number;
		duration_ms: number | null;
		started_at: string;
		ended_at: string | null;
	}

	interface ReplayableTrace {
		trace_id: string;
		trace_name: string | null;
		llm_calls: LlmCallReplay[];
		total_cost: number;
		total_tokens: number;
	}

	let trace: ReplayableTrace | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let modifiedInputs: Map<string, string> = $state(new Map());
	let expandedCalls: Set<string> = $state(new Set());
	let selectedCall: string | null = $state(null);

	const traceId = $derived(($page.params as PageParams).traceId ?? '');
	const focusSpanId = $derived($page.url.searchParams.get('span') ?? '');

	async function loadTrace() {
		loading = true;
		error = '';
		try {
			const res = await fetch(`/api/replay/${traceId}`, { credentials: 'include' });
			if (!res.ok) {
				const data = await res.json();
				throw new Error(data.error || 'Failed to load trace');
			}
			trace = await res.json();

			// Auto-focus span from ?span= param
			if (focusSpanId && trace) {
				const match = trace.llm_calls.find(c => c.span_id === focusSpanId);
				if (match) {
					expandedCalls.add(match.span_id);
					expandedCalls = new Set(expandedCalls);
					selectedCall = match.span_id;
					await tick();
					document.getElementById(`replay-call-${match.span_id}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' });
				}
			}
		} catch (e: any) {
			error = e?.message || 'Failed to load trace';
		}
		loading = false;
	}

	function getInput(call: LlmCallReplay): string {
		const modified = modifiedInputs.get(call.span_id);
		if (modified !== undefined) return modified;
		return JSON.stringify(call.input, null, 2);
	}

	function setModifiedInput(spanId: string, value: string) {
		modifiedInputs.set(spanId, value);
		modifiedInputs = new Map(modifiedInputs);
	}

	function resetInput(spanId: string) {
		modifiedInputs.delete(spanId);
		modifiedInputs = new Map(modifiedInputs);
	}

	function toggleExpanded(spanId: string) {
		if (expandedCalls.has(spanId)) {
			expandedCalls.delete(spanId);
		} else {
			expandedCalls.add(spanId);
		}
		expandedCalls = new Set(expandedCalls);
	}

	function selectCall(spanId: string) {
		selectedCall = selectedCall === spanId ? null : spanId;
	}

	function formatDuration(ms: number | null): string {
		if (ms === null) return '-';
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(2)}s`;
	}

	function formatJson(val: unknown): string {
		return JSON.stringify(val, null, 2);
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
	}

	$effect(() => {
		if (traceId) {
			loadTrace();
		}
	});
</script>

<div class="app-shell-wide">
	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="text-text-muted text-sm">Loading trace...</div>
		</div>
	{:else if error}
		<div class="space-y-4">
			<div class="alert-danger">{error}</div>
			<button onclick={() => goto('/traces')} class="btn-ghost text-sm">
				← Back to traces
			</button>
		</div>
	{:else if trace}
		<div class="space-y-4">
			<!-- Header -->
			<div class="flex items-center justify-between">
				<div>
					<h1 class="text-lg font-semibold text-text">
						{trace.trace_name || 'Replay'} · {shortId(trace.trace_id)}
					</h1>
					<p class="text-xs text-text-muted">
						{trace.llm_calls.length} LLM calls · {trace.total_tokens.toLocaleString()} tokens · ${trace.total_cost.toFixed(4)}
					</p>
				</div>
				<div class="flex items-center gap-2">
					<a href="/traces/{trace.trace_id}" class="btn-ghost text-sm">
						View original trace
					</a>
					<button onclick={() => goto('/traces')} class="btn-ghost text-sm">
						← Back to traces
					</button>
				</div>
			</div>

			<!-- Info banner -->
			<div class="surface-panel p-4 rounded-lg border border-border/50">
				<h2 class="text-sm font-medium text-text mb-2">Agent Replay</h2>
				<p class="text-[12px] text-text-secondary mb-3">
					View and modify the inputs to see how the agent would behave differently. 
					Copy the modified payload to test in your code.
				</p>
				<div class="text-[11px] text-text-muted">
					<span class="font-medium">How it works:</span>
					Edit any input JSON below, then use the "Copy Modified Input" button to get the modified payload.
				</div>
			</div>

			<!-- LLM Calls -->
			{#if trace.llm_calls.length === 0}
				<div class="text-center py-8 text-text-muted text-sm">
					No LLM calls found in this trace.
				</div>
			{:else}
				<div class="space-y-3">
					{#each trace.llm_calls as call, i (call.span_id)}
						{@const hasModification = modifiedInputs.has(call.span_id)}
						{@const isExpanded = expandedCalls.has(call.span_id)}
						{@const isSelected = selectedCall === call.span_id}
						<div id="replay-call-{call.span_id}" class="surface-panel rounded-lg border {isSelected ? 'border-accent/50' : 'border-border/50'} overflow-hidden motion-row">
							<!-- Call header -->
							<button
								class="w-full flex items-center gap-3 px-4 py-3 hover:bg-bg-tertiary/30 transition-colors"
								onclick={() => toggleExpanded(call.span_id)}
							>
								<span class="text-[11px] text-text-muted font-mono w-6">{i + 1}</span>
								<span class="w-2 h-2 rounded-full {hasModification ? 'bg-warning' : 'bg-accent'}"></span>
								<div class="flex-1 text-left">
									<div class="text-[13px] font-medium text-text">{call.name}</div>
									<div class="text-[11px] text-text-muted">
										{call.model}
										{#if call.provider}· {call.provider}{/if}
										· {call.input_tokens ?? 0 + (call.output_tokens ?? 0)} tokens
										· {formatDuration(call.duration_ms)}
									</div>
								</div>
								{#if hasModification}
									<span class="text-[10px] px-2 py-0.5 rounded bg-warning/20 text-warning">Modified</span>
								{/if}
								<svg 
									class="w-4 h-4 text-text-muted transition-transform {isExpanded ? 'rotate-180' : ''}" 
									viewBox="0 0 24 24" 
									fill="none" 
									stroke="currentColor" 
									stroke-width="2"
								>
									<path d="M6 9l6 6 6-6"/>
								</svg>
							</button>

							<!-- Expanded content -->
							{#if isExpanded}
								<div class="border-t border-border/50">
									<div class="flex">
										<!-- Original -->
										<div class="flex-1 border-r border-border/50">
											<div class="flex items-center justify-between px-4 py-2 bg-bg-tertiary/20 border-b border-border/50">
												<span class="text-[11px] font-medium text-text-muted">Original Input</span>
												<button
													class="text-[10px] text-text-muted hover:text-text transition-colors"
													onclick={() => copyToClipboard(formatJson(call.input))}
												>
													Copy
												</button>
											</div>
											<div class="p-4">
												<pre class="text-[11px] text-text-secondary font-mono whitespace-pre-wrap bg-bg-tertiary/30 rounded p-3 max-h-64 overflow-auto">{formatJson(call.input)}</pre>
											</div>
											<div class="px-4 pb-4">
												<div class="text-[10px] text-text-muted mb-1">Original Output</div>
												<pre class="text-[11px] text-text-secondary font-mono whitespace-pre-wrap bg-bg-tertiary/30 rounded p-3 max-h-48 overflow-auto">{formatJson(call.output)}</pre>
											</div>
										</div>

										<!-- Modified -->
										<div class="flex-1">
											<div class="flex items-center justify-between px-4 py-2 bg-bg-tertiary/20 border-b border-border/50">
												<span class="text-[11px] font-medium {hasModification ? 'text-warning' : 'text-text-muted'}">
													{hasModification ? 'Modified Input' : 'Input (edit below)'}
												</span>
												<div class="flex items-center gap-2">
													{#if hasModification}
														<button
															class="text-[10px] text-text-muted hover:text-text transition-colors"
															onclick={() => copyToClipboard(modifiedInputs.get(call.span_id) || '')}
														>
															Copy Modified
														</button>
														<button
															class="text-[10px] text-danger hover:text-danger/80 transition-colors"
															onclick={() => resetInput(call.span_id)}
														>
															Reset
														</button>
													{/if}
												</div>
											</div>
											<div class="p-4">
												<textarea
													class="w-full text-[11px] text-text font-mono bg-bg-tertiary/30 rounded p-3 min-h-48 max-h-64 resize-y focus:outline-none focus:ring-1 focus:ring-accent/50"
													value={getInput(call)}
													oninput={(e) => setModifiedInput(call.span_id, (e.target as HTMLTextAreaElement).value)}
													placeholder="Enter modified input JSON..."
												></textarea>
											</div>
											{#if hasModification}
												<div class="px-4 pb-4">
													<div class="text-[10px] text-text-muted mb-1">New Output (run with modified input to see)</div>
													<div class="text-[11px] text-text-muted italic bg-bg-tertiary/20 rounded p-3">
														Execute this call to see the new output.
													</div>
												</div>
											{/if}
										</div>
									</div>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}

			<!-- Actions -->
			<div class="flex items-center gap-3 pt-4 border-t border-border/50">
				<button onclick={() => goto('/traces')} class="btn-ghost">
					← Back to traces
				</button>
				<div class="flex-1"></div>
				{#if modifiedInputs.size > 0}
					<span class="text-[11px] text-warning">
						{modifiedInputs.size} call{modifiedInputs.size > 1 ? 's' : ''} modified
					</span>
				{/if}
			</div>
		</div>
	{/if}
</div>
