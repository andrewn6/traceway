<script lang="ts">
	import { page } from '$app/state';
	import { getFileContent, getFileVersions, getFileTraces, shortId, type FileVersion, type Span } from '$lib/api';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { onMount } from 'svelte';

	const filePath = $derived(page.params.path ?? '');

	let content = $state('');
	let versions: FileVersion[] = $state([]);
	let readTraces: { trace_id: string; span_name: string; at: string }[] = $state([]);
	let writeTraces: { trace_id: string; span_name: string; at: string }[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let activeTab: 'content' | 'history' | 'traces' = $state('content');
	let selectedVersionA: string | null = $state(null);
	let selectedVersionB: string | null = $state(null);
	let showDiff = $state(false);

	async function loadFile(path: string) {
		loading = true;
		try {
			const [contentRes, versionsRes, tracesRes] = await Promise.all([
				getFileContent(path).catch(() => ''),
				getFileVersions(path).catch(() => []),
				getFileTraces(path).catch(() => ({ reads: [], writes: [] }))
			]);
			content = contentRes;
			versions = versionsRes;
			readTraces = tracesRes.reads;
			writeTraces = tracesRes.writes;
			error = '';
		} catch (e) {
			error = 'Could not load file. Is the daemon running?';
		}
		loading = false;
	}

	onMount(() => {
		loadFile(filePath);
	});

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function timeAgo(dateStr: string): string {
		const d = new Date(dateStr);
		const now = Date.now();
		const diff = now - d.getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 1) return 'just now';
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}
</script>

<div class="max-w-5xl space-y-4">
	<!-- Breadcrumb -->
	<div class="flex items-center gap-2 text-sm">
		<a href="/files" class="text-text-secondary hover:text-text">&larr; Files</a>
		<span class="text-text-muted">/</span>
		{#each filePath.split('/') as segment, i}
			{#if i > 0}<span class="text-text-muted">/</span>{/if}
			{#if i === filePath.split('/').length - 1}
				<span class="text-text font-semibold">{segment}</span>
			{:else}
				<a
					href="/files/{filePath.split('/').slice(0, i + 1).join('/')}"
					class="text-text-secondary hover:text-text"
				>{segment}</a>
			{/if}
		{/each}
	</div>

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if error}
		<div class="bg-bg-secondary border border-border rounded p-6 text-center">
			<p class="text-text-muted text-sm">{error}</p>
		</div>
	{:else}
		<!-- Tab bar -->
		<div class="flex gap-0 border-b border-border">
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'content' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => activeTab = 'content'}
			>
				Content
			</button>
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'history' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => activeTab = 'history'}
			>
				History
				{#if versions.length > 0}
					<span class="ml-1 text-text-muted text-xs">({versions.length})</span>
				{/if}
			</button>
			<button
				class="px-4 py-2 text-sm transition-colors border-b-2
					{activeTab === 'traces' ? 'border-accent text-text' : 'border-transparent text-text-secondary hover:text-text'}"
				onclick={() => activeTab = 'traces'}
			>
				Traces
				{#if readTraces.length + writeTraces.length > 0}
					<span class="ml-1 text-text-muted text-xs">({readTraces.length + writeTraces.length})</span>
				{/if}
			</button>
		</div>

		<!-- Content tab -->
		{#if activeTab === 'content'}
			<div class="bg-bg-secondary border border-border rounded">
				<div class="px-4 py-2 border-b border-border flex items-center justify-between">
					<span class="text-text-muted text-xs">{filePath}</span>
					{#if versions.length > 0}
						<span class="text-text-muted text-xs font-mono">
							{versions[0]?.hash?.slice(0, 8)} &middot; {formatSize(versions[0]?.size ?? 0)}
						</span>
					{/if}
				</div>
				<pre class="p-4 text-sm text-text overflow-x-auto whitespace-pre-wrap font-mono leading-relaxed">{content || '(empty file)'}</pre>
			</div>
		{/if}

		<!-- History tab -->
		{#if activeTab === 'history'}
			{#if versions.length === 0}
				<div class="text-text-muted text-sm py-4 text-center">No version history</div>
			{:else}
				<div class="space-y-1">
					{#each versions as version, i (version.hash)}
						<div class="bg-bg-secondary border border-border rounded px-4 py-3 flex items-center gap-4 text-sm">
							<span class="text-text-muted text-xs font-mono w-8">v{versions.length - i}</span>
							<span class="text-accent font-mono text-xs">{version.hash.slice(0, 12)}</span>
							<span class="text-text-secondary text-xs">{formatSize(version.size)}</span>
							<span class="text-text-muted text-xs flex-1">{timeAgo(version.created_at)}</span>
							{#if version.created_by_span}
								<a href="/traces/{version.created_by_trace}" class="text-accent text-xs hover:underline">
									trace {shortId(version.created_by_trace ?? '')}
								</a>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		{/if}

		<!-- Traces tab -->
		{#if activeTab === 'traces'}
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
				<!-- Read by -->
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase mb-3">Read by ({readTraces.length})</div>
					{#if readTraces.length === 0}
						<div class="text-text-muted text-xs">No traces have read this file</div>
					{:else}
						<div class="space-y-1">
							{#each readTraces as trace}
								<a
									href="/traces/{trace.trace_id}"
									class="flex items-center gap-2 text-xs py-1 hover:bg-bg-tertiary rounded px-2 transition-colors"
								>
									<svg class="w-3.5 h-3.5 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
										<path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" />
										<path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
									</svg>
									<span class="text-accent font-mono">{shortId(trace.trace_id)}</span>
									<span class="text-text-secondary flex-1 truncate">{trace.span_name}</span>
									<span class="text-text-muted">{timeAgo(trace.at)}</span>
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<!-- Written by -->
				<div class="bg-bg-secondary border border-border rounded p-4">
					<div class="text-text-muted text-xs uppercase mb-3">Written by ({writeTraces.length})</div>
					{#if writeTraces.length === 0}
						<div class="text-text-muted text-xs">No traces have written this file</div>
					{:else}
						<div class="space-y-1">
							{#each writeTraces as trace}
								<a
									href="/traces/{trace.trace_id}"
									class="flex items-center gap-2 text-xs py-1 hover:bg-bg-tertiary rounded px-2 transition-colors"
								>
									<svg class="w-3.5 h-3.5 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
										<path stroke-linecap="round" stroke-linejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" />
									</svg>
									<span class="text-accent font-mono">{shortId(trace.trace_id)}</span>
									<span class="text-text-secondary flex-1 truncate">{trace.span_name}</span>
									<span class="text-text-muted">{timeAgo(trace.at)}</span>
								</a>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	{/if}
</div>
