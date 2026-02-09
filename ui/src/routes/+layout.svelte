<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { getStats, subscribeEvents, type Stats } from '$lib/api';
	import { onMount } from 'svelte';

	let { children } = $props();

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let connected = $state(false);

	onMount(() => {
		getStats().then((s) => (stats = s)).catch(() => {});

		const unsub = subscribeEvents((event) => {
			connected = true;
			if (event.type === 'span_created') {
				stats.span_count++;
			} else if (event.type === 'span_deleted') {
				stats.span_count = Math.max(0, stats.span_count - 1);
			} else if (event.type === 'trace_deleted') {
				stats.trace_count = Math.max(0, stats.trace_count - 1);
				getStats().then((s) => (stats = s)).catch(() => {});
			} else if (event.type === 'cleared') {
				stats = { trace_count: 0, span_count: 0 };
			}
		});

		const interval = setInterval(() => {
			getStats().then((s) => (stats = s)).catch(() => {});
		}, 5000);

		return () => {
			unsub();
			clearInterval(interval);
		};
	});

	const navItems = [
		{ href: '/traces', label: 'Traces', icon: 'trace' },
		{ href: '/files', label: 'Files', icon: 'file' },
		{ href: '/analysis', label: 'Analysis', icon: 'analysis' },
		{ href: '/settings', label: 'Settings', icon: 'settings' },
	];

	function isActive(href: string): boolean {
		return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
	}
</script>

<div class="min-h-screen flex">
	<!-- Left sidebar -->
	<aside class="w-48 shrink-0 border-r border-border bg-bg-secondary flex flex-col">
		<!-- Logo -->
		<div class="px-4 py-4 border-b border-border">
			<a href="/traces" class="text-text font-bold text-base tracking-tight hover:text-accent transition-colors">
				llmtrace
			</a>
		</div>

		<!-- Nav items -->
		<nav class="flex-1 py-2 px-2 space-y-0.5">
			{#each navItems as item}
				<a
					href={item.href}
					class="flex items-center gap-2.5 px-3 py-2 rounded text-sm transition-colors
						{isActive(item.href)
							? 'bg-bg-tertiary text-text'
							: 'text-text-secondary hover:text-text hover:bg-bg-tertiary/50'}"
				>
					<!-- Icons -->
					{#if item.icon === 'trace'}
						<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 0 1 0 3.75H5.625a1.875 1.875 0 0 1 0-3.75Z" />
						</svg>
					{:else if item.icon === 'file'}
						<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
						</svg>
					{:else if item.icon === 'analysis'}
						<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
						</svg>
					{:else if item.icon === 'settings'}
						<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z" />
							<path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
						</svg>
					{/if}
					{item.label}
				</a>
			{/each}
		</nav>

		<!-- Status footer -->
		<div class="px-3 py-3 border-t border-border space-y-1.5">
			<div class="flex items-center gap-1.5 text-xs text-text-muted">
				<span class="w-1.5 h-1.5 rounded-full {connected ? 'bg-success' : 'bg-text-muted'}"></span>
				{connected ? 'connected' : 'connecting'}
			</div>
			<div class="text-xs text-text-muted font-mono">
				{stats.trace_count} traces &middot; {stats.span_count} spans
			</div>
		</div>
	</aside>

	<!-- Main content -->
	<main class="flex-1 min-w-0 overflow-auto">
		<div class="p-6">
			{@render children()}
		</div>
	</main>
</div>
