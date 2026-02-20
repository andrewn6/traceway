<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getStats, getAuthConfig, getAuthMe, logout, subscribeEvents, type Stats, type AuthConfig, type AuthMe } from '$lib/api';
	import { onMount } from 'svelte';

	let { children } = $props();

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let connected = $state(false);
	let authConfig: AuthConfig = $state({ mode: 'local', features: [] });
	let authMe: AuthMe | null = $state(null);
	let authChecked = $state(false);

	// Auth pages don't need sidebar or auth check
	const authPages = ['/login', '/signup'];
	const isAuthPage = $derived(authPages.includes(page.url.pathname));
	const isCloudMode = $derived(authConfig.mode === 'cloud');
	const isAuthenticated = $derived(authMe !== null || authConfig.mode === 'local');

	onMount(() => {
		// Load auth config first, then conditionally check auth
		getAuthConfig()
			.then(async (c) => {
				authConfig = c;

				if (c.mode === 'cloud' && !isAuthPage) {
					try {
						authMe = await getAuthMe();
					} catch {
						// Not authenticated - redirect to login
						goto('/login');
						return;
					}
				}

				authChecked = true;
			})
			.catch(() => {
				// Can't reach API - assume local mode
				authChecked = true;
			});

		getStats().then((s) => (stats = s)).catch(() => {});

		const unsub = subscribeEvents((event) => {
			connected = true;
			if (event.type === 'span_created') {
				stats.span_count++;
			} else if (event.type === 'span_deleted') {
				stats.span_count = Math.max(0, stats.span_count - 1);
			} else if (event.type === 'trace_created') {
				stats.trace_count++;
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

	async function handleLogout() {
		await logout();
		authMe = null;
		goto('/login');
	}

	const navItems = $derived([
		{ href: '/', label: 'Dashboard', icon: 'dashboard' },
		{ href: '/traces', label: 'Traces', icon: 'trace' },
		{ href: '/query', label: 'Query', icon: 'query' },
		{ href: '/datasets', label: 'Datasets', icon: 'dataset' },
		{ href: '/analytics', label: 'Analytics', icon: 'analysis' },
		{ href: '/settings', label: 'Settings', icon: 'settings' },
		// Cloud-only items
		...(isCloudMode ? [
			{ href: '/settings/team', label: 'Team', icon: 'team' },
			{ href: '/settings/api-keys', label: 'API Keys', icon: 'key' },
		] : []),
	]);

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
	}
</script>

{#if isAuthPage}
	<!-- Auth pages: no sidebar, no auth guard -->
	{@render children()}
{:else if !authChecked}
	<!-- Loading auth state -->
	<div class="min-h-screen flex items-center justify-center bg-bg">
		<div class="text-text-muted text-sm">Loading...</div>
	</div>
{:else}
	<div class="min-h-screen flex">
		<!-- Left sidebar -->
		<aside class="w-48 shrink-0 border-r border-border bg-bg-secondary flex flex-col">
			<!-- Logo -->
			<div class="px-4 py-4 border-b border-border">
				<a href="/" class="text-text font-bold text-base tracking-tight hover:text-accent transition-colors">
					Traceway
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
					{#if item.icon === 'dashboard'}
						<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6ZM3.75 15.75A2.25 2.25 0 0 1 6 13.5h2.25a2.25 2.25 0 0 1 2.25 2.25V18a2.25 2.25 0 0 1-2.25 2.25H6A2.25 2.25 0 0 1 3.75 18v-2.25ZM13.5 6a2.25 2.25 0 0 1 2.25-2.25H18A2.25 2.25 0 0 1 20.25 6v2.25A2.25 2.25 0 0 1 18 10.5h-2.25a2.25 2.25 0 0 1-2.25-2.25V6ZM13.5 15.75a2.25 2.25 0 0 1 2.25-2.25H18a2.25 2.25 0 0 1 2.25 2.25V18A2.25 2.25 0 0 1 18 20.25h-2.25A2.25 2.25 0 0 1 13.5 18v-2.25Z" />
						</svg>
					{:else if item.icon === 'trace'}
							<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 0 1 0 3.75H5.625a1.875 1.875 0 0 1 0-3.75Z" />
							</svg>
						{:else if item.icon === 'query'}
							<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="m6.75 7.5 3 2.25-3 2.25m4.5 0h3m-9 8.25h13.5A2.25 2.25 0 0 0 21 18V6a2.25 2.25 0 0 0-2.25-2.25H5.25A2.25 2.25 0 0 0 3 6v12a2.25 2.25 0 0 0 2.25 2.25Z" />
							</svg>
					{:else if item.icon === 'dataset'}
						<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M3.375 19.5h17.25m-17.25 0a1.125 1.125 0 0 1-1.125-1.125M3.375 19.5h7.5c.621 0 1.125-.504 1.125-1.125m-9.75 0V5.625m0 12.75v-1.5c0-.621.504-1.125 1.125-1.125m18.375 2.625V5.625m0 12.75c0 .621-.504 1.125-1.125 1.125m1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125m0 3.75h-7.5A1.125 1.125 0 0 1 12 18.375m9.75-12.75c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125m19.5 0v1.5c0 .621-.504 1.125-1.125 1.125M2.25 5.625v1.5c0 .621.504 1.125 1.125 1.125m0 0h17.25m-17.25 0h7.5c.621 0 1.125.504 1.125 1.125M3.375 8.25c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125m17.25-3.75h-7.5c-.621 0-1.125.504-1.125 1.125m8.625-1.125c.621 0 1.125.504 1.125 1.125v1.5c0 .621-.504 1.125-1.125 1.125m-17.25 0h7.5m-7.5 0c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125M12 10.875v-1.5m0 1.5c0 .621-.504 1.125-1.125 1.125M12 10.875c0 .621.504 1.125 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125M10.875 12c-.621 0-1.125.504-1.125 1.125M12 12c.621 0 1.125.504 1.125 1.125m-2.25 0c.621 0 1.125.504 1.125 1.125m0 0v1.5c0 .621-.504 1.125-1.125 1.125M12 15.375c0-.621-.504-1.125-1.125-1.125" />
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
						{:else if item.icon === 'key'}
							<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z" />
							</svg>
						{:else if item.icon === 'team'}
							<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 0 0 2.625.372 9.337 9.337 0 0 0 4.121-.952 4.125 4.125 0 0 0-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 0 1 8.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0 1 11.964-3.07M12 6.375a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0Zm8.25 2.25a2.625 2.625 0 1 1-5.25 0 2.625 2.625 0 0 1 5.25 0Z" />
							</svg>
						{/if}
						{item.label}
					</a>
				{/each}
			</nav>

			<!-- Status footer -->
			<div class="px-3 py-3 border-t border-border space-y-1.5">
				{#if isCloudMode && authMe}
					<button
						onclick={handleLogout}
						class="text-xs text-text-muted hover:text-text transition-colors cursor-pointer"
					>
						{authMe.org_id.slice(0, 8)} &middot; Sign out
					</button>
				{/if}
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
{/if}
