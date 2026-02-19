<script lang="ts">
	import { getFiles, subscribeEvents, type FileVersion } from '$lib/api';
	import { onMount } from 'svelte';

	let files: FileVersion[] = $state([]);
	let loading = $state(true);
	let search = $state('');
	let error = $state('');

	// Build tree structure from flat file list
	interface TreeNode {
		name: string;
		path: string;
		isDir: boolean;
		children: TreeNode[];
		file?: FileVersion;
		expanded: boolean;
	}

	function buildTree(files: FileVersion[]): TreeNode[] {
		const root: TreeNode[] = [];

		for (const file of files) {
			const parts = file.path.replace(/^\//, '').split('/');
			let current = root;

			for (let i = 0; i < parts.length; i++) {
				const part = parts[i];
				const isLast = i === parts.length - 1;
				const existingPath = parts.slice(0, i + 1).join('/');
				let existing = current.find((n) => n.name === part);

				if (!existing) {
					existing = {
						name: part,
						path: existingPath,
						isDir: !isLast,
						children: [],
						file: isLast ? file : undefined,
						expanded: true
					};
					current.push(existing);
				}

				if (!isLast) {
					existing.isDir = true;
					current = existing.children;
				}
			}
		}

		// Sort: directories first, then alphabetical
		function sortTree(nodes: TreeNode[]): TreeNode[] {
			return nodes
				.sort((a, b) => {
					if (a.isDir && !b.isDir) return -1;
					if (!a.isDir && b.isDir) return 1;
					return a.name.localeCompare(b.name);
				})
				.map((n) => ({ ...n, children: sortTree(n.children) }));
		}

		return sortTree(root);
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	async function loadFiles() {
		try {
			const res = await getFiles();
			files = res.files;
			error = '';
		} catch (e) {
			error = 'Could not load files. Is the daemon running?';
		}
		loading = false;
	}

	onMount(() => {
		loadFiles();

		const unsub = subscribeEvents((event) => {
			if (event.type === 'file_version_created') {
				loadFiles();
			}
		});

		return unsub;
	});

	const filteredFiles = $derived(
		search
			? files.filter((f) => f.path.toLowerCase().includes(search.toLowerCase()))
			: files
	);

	const tree = $derived(buildTree(filteredFiles));

	let expandedPaths: Set<string> = $state(new Set());

	function toggleDir(path: string) {
		if (expandedPaths.has(path)) {
			expandedPaths.delete(path);
		} else {
			expandedPaths.add(path);
		}
		expandedPaths = new Set(expandedPaths);
	}

	function isExpanded(path: string): boolean {
		return expandedPaths.has(path) || search.length > 0;
	}
</script>

<div class="max-w-4xl space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold">Files</h1>
		<input
			type="text"
			placeholder="Search files..."
			bind:value={search}
			class="bg-bg-tertiary border border-border rounded px-3 py-1.5 text-xs text-text placeholder:text-text-muted w-48"
		/>
	</div>

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading files...</div>
	{:else if error}
		<div class="bg-bg-secondary border border-border rounded p-6 text-center">
			<p class="text-text-muted text-sm">{error}</p>
		</div>
	{:else if files.length === 0}
		<div class="bg-bg-secondary border border-border rounded p-6 text-center">
			<p class="text-text-muted text-sm">No tracked files yet</p>
			<p class="text-text-muted text-xs mt-1">Files will appear when the filesystem is mounted and applications read/write to it.</p>
		</div>
	{:else}
		<div class="bg-bg-secondary border border-border rounded">
			{#snippet renderNode(nodes: TreeNode[], depth: number)}
				{#each nodes as node (node.path)}
					{#if node.isDir}
						<button
							class="w-full flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-bg-tertiary transition-colors"
							style="padding-left: {depth * 16 + 12}px"
							onclick={() => toggleDir(node.path)}
						>
							<svg class="w-3.5 h-3.5 text-text-muted transition-transform {isExpanded(node.path) ? 'rotate-90' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
							</svg>
							<svg class="w-4 h-4 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z" />
							</svg>
							<span class="text-text">{node.name}</span>
						</button>
						{#if isExpanded(node.path)}
							{@render renderNode(node.children, depth + 1)}
						{/if}
					{:else}
						<a
							href="/files/{node.path}"
							class="w-full flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-bg-tertiary transition-colors"
							style="padding-left: {depth * 16 + 28}px"
						>
							<svg class="w-4 h-4 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
							</svg>
							<span class="text-text flex-1">{node.name}</span>
							{#if node.file}
								<span class="text-text-muted text-xs font-mono">{formatSize(node.file.size)}</span>
								<span class="text-text-muted text-xs font-mono">
									{new Date(node.file.created_at).toLocaleDateString()}
								</span>
							{/if}
						</a>
					{/if}
				{/each}
			{/snippet}

			{@render renderNode(tree, 0)}
		</div>
	{/if}
</div>
