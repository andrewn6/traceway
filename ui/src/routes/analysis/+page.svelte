<script lang="ts">
	import { getTraces } from '$lib/api';
	import { shortId } from '$lib/api';
	import { onMount } from 'svelte';

	let activeTab: 'diff' | 'context' | 'unused' | 'impact' = $state('diff');
	let traceIds: string[] = $state([]);
	let loading = $state(true);

	// Trace diff state
	let traceA = $state('');
	let traceB = $state('');

	// Impact state
	let impactPath = $state('');

	// Unused context state
	let unusedTraceId = $state('');

	onMount(async () => {
		try {
			const result = await getTraces();
			traceIds = result.traces.map(t => t.id);
		} catch {
			// daemon not running
		}
		loading = false;
	});
</script>

<div class="max-w-5xl space-y-4">
	<h1 class="text-xl font-bold">Analysis</h1>

	<!-- Tab bar -->
	<div class="flex gap-0 border-b border-border">
		<button
			class="px-4 py-2 text-sm transition-colors border-b-2
				{activeTab === 'diff' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
			onclick={() => activeTab = 'diff'}
		>Trace Diff</button>
		<button
			class="px-4 py-2 text-sm transition-colors border-b-2
				{activeTab === 'context' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
			onclick={() => activeTab = 'context'}
		>Context Diff</button>
		<button
			class="px-4 py-2 text-sm transition-colors border-b-2
				{activeTab === 'unused' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
			onclick={() => activeTab = 'unused'}
		>Unused Context</button>
		<button
			class="px-4 py-2 text-sm transition-colors border-b-2
				{activeTab === 'impact' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
			onclick={() => activeTab = 'impact'}
		>Impact</button>
	</div>

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else}

		<!-- Trace Diff -->
		{#if activeTab === 'diff'}
			<div class="bg-bg-secondary border border-border rounded p-6 space-y-4">
				<p class="text-text-secondary text-sm">Compare two traces to see what changed — files read, files written, LLM calls, and outputs.</p>
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label for="diff-trace-a" class="text-text-muted text-xs uppercase block mb-1">Trace A</label>
						<select id="diff-trace-a" bind:value={traceA} class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text">
							<option value="">Select trace...</option>
							{#each traceIds as id}
								<option value={id}>{shortId(id)}</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="diff-trace-b" class="text-text-muted text-xs uppercase block mb-1">Trace B</label>
						<select id="diff-trace-b" bind:value={traceB} class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text">
							<option value="">Select trace...</option>
							{#each traceIds as id}
								<option value={id}>{shortId(id)}</option>
							{/each}
						</select>
					</div>
				</div>
				{#if traceA && traceB}
					<button
						class="bg-accent text-bg px-4 py-2 rounded text-sm font-semibold hover:opacity-90 transition-opacity"
						onclick={() => {/* TODO: call analysis/diff endpoint */}}
					>
						Compare Traces
					</button>
					<div class="border border-border rounded p-4 text-text-muted text-sm text-center">
						Trace diff requires the analysis API endpoint.<br/>
						<span class="text-text-muted text-xs">Waiting for foundation (T021) to ship.</span>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Context Diff -->
		{#if activeTab === 'context'}
			<div class="bg-bg-secondary border border-border rounded p-6 space-y-4">
				<p class="text-text-secondary text-sm">Compare the input context between two traces — which files were the same version vs different.</p>
				<div class="grid grid-cols-2 gap-4">
					<div>
						<label for="ctx-trace-a" class="text-text-muted text-xs uppercase block mb-1">Trace A</label>
						<select id="ctx-trace-a" bind:value={traceA} class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text">
							<option value="">Select trace...</option>
							{#each traceIds as id}
								<option value={id}>{shortId(id)}</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="ctx-trace-b" class="text-text-muted text-xs uppercase block mb-1">Trace B</label>
						<select id="ctx-trace-b" bind:value={traceB} class="w-full bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text">
							<option value="">Select trace...</option>
							{#each traceIds as id}
								<option value={id}>{shortId(id)}</option>
							{/each}
						</select>
					</div>
				</div>
				{#if traceA && traceB}
					<div class="border border-border rounded p-4 text-text-muted text-sm text-center">
						Context diff requires the analysis API endpoint.<br/>
						<span class="text-text-muted text-xs">Waiting for foundation (T021) to ship.</span>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Unused Context -->
		{#if activeTab === 'unused'}
			<div class="bg-bg-secondary border border-border rounded p-6 space-y-4">
				<p class="text-text-secondary text-sm">Find files that were read by a trace but whose content didn't influence the LLM output — potential context bloat.</p>
				<div>
					<label for="unused-trace" class="text-text-muted text-xs uppercase block mb-1">Trace</label>
					<select id="unused-trace" bind:value={unusedTraceId} class="w-full max-w-sm bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text">
						<option value="">Select trace...</option>
						{#each traceIds as id}
							<option value={id}>{shortId(id)}</option>
						{/each}
					</select>
				</div>
				{#if unusedTraceId}
					<div class="border border-border rounded p-4 text-text-muted text-sm text-center">
						Unused context analysis requires the analysis API endpoint.<br/>
						<span class="text-text-muted text-xs">Waiting for foundation (T021) to ship.</span>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Impact / Fan-out -->
		{#if activeTab === 'impact'}
			<div class="bg-bg-secondary border border-border rounded p-6 space-y-4">
				<p class="text-text-secondary text-sm">Select a file to see which traces read it downstream — trace the impact of a bad write.</p>
				<div>
					<label for="impact-path" class="text-text-muted text-xs uppercase block mb-1">File path</label>
					<input
						id="impact-path"
						type="text"
						placeholder="/prompts/system.md"
						bind:value={impactPath}
						class="w-full max-w-sm bg-bg-tertiary border border-border rounded px-3 py-2 text-sm text-text placeholder:text-text-muted"
					/>
				</div>
				{#if impactPath}
					<div class="border border-border rounded p-4 text-text-muted text-sm text-center">
						Impact analysis requires the file-impact API endpoint.<br/>
						<span class="text-text-muted text-xs">Waiting for foundation (T021) to ship.</span>
					</div>
				{/if}
			</div>
		{/if}
	{/if}
</div>
