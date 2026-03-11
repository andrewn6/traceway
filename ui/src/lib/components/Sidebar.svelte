<script lang="ts">
	import { page } from '$app/state';
	import TracewayWordmark from './TracewayWordmark.svelte';

	let {
		authMe = null,
		projects = [],
		currentProject = null,
		pendingReviewCount = 0,
		isCloudMode = false,
		theme = 'system' as 'dark' | 'light' | 'system',
		onToggleTheme = () => {},
		onOpenSearch = () => {},
		onSwitchProject = (_id: string) => {},
		onCreateProject = () => {},
		onLogout = () => {},
	}: {
		authMe: any;
		projects: any[];
		currentProject: any;
		pendingReviewCount: number;
		isCloudMode: boolean;
		theme: 'dark' | 'light' | 'system';
		onToggleTheme?: () => void;
		onOpenSearch?: () => void;
		onSwitchProject?: (id: string) => void;
		onCreateProject?: () => void;
		onLogout?: () => void;
	} = $props();

	let projectDropdownOpen = $state(false);

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
	}

	const navItems = [
		{ href: '/', label: 'Dashboard', icon: 'home' },
		{ href: '/traces', label: 'Traces', icon: 'traces' },
		{ href: '/sessions', label: 'Sessions', icon: 'sessions' },
		{ href: '/review', label: 'Review', icon: 'review', badge: true },
		{ href: '/approvals', label: 'Approvals', icon: 'approvals' },
		{ href: '/analytics', label: 'Analytics', icon: 'analytics' },
		{ href: '/query', label: 'Search', icon: 'search' },
		{ href: '/datasets', label: 'Datasets', icon: 'datasets' },
	];

	const settingsItems = [
		{ href: '/settings', label: 'General' },
		{ href: '/settings/providers', label: 'Providers' },
		{ href: '/settings/api-keys', label: 'API Keys' },
		{ href: '/settings/team', label: 'Team' },
		{ href: '/settings/billing', label: 'Billing' },
	];

	const themeLabel = $derived(theme === 'system' ? 'Auto' : theme === 'light' ? 'Light' : 'Dark');
</script>

<aside class="sidebar-shell">
	<!-- Logo -->
	<div class="flex items-center gap-2 px-4 py-3 border-b border-border">
		<a href="/" class="flex items-center">
			<TracewayWordmark className="h-5 w-auto text-text" />
		</a>
	</div>

	<!-- Project switcher (cloud mode) -->
	{#if isCloudMode && projects.length > 0}
		<div class="px-3 py-2 border-b border-border">
			<div class="relative">
				<button
					onclick={() => (projectDropdownOpen = !projectDropdownOpen)}
					class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-md text-sm text-text-secondary hover:text-text hover:bg-bg-tertiary transition-colors"
				>
					<span class="w-2 h-2 rounded-full bg-accent shrink-0"></span>
					<span class="truncate flex-1 text-left">{currentProject?.name ?? 'Select project'}</span>
					<svg class="w-3 h-3 text-text-muted shrink-0 transition-transform {projectDropdownOpen ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
				</button>

				{#if projectDropdownOpen}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div class="fixed inset-0 z-40" onclick={() => (projectDropdownOpen = false)} onkeydown={(e) => e.key === 'Escape' && (projectDropdownOpen = false)}></div>
					<div class="absolute left-0 right-0 top-full mt-1 z-50 bg-bg-secondary border border-border rounded-md shadow-lg overflow-hidden">
						<div class="py-1">
							{#each projects as project}
								<button
									onclick={() => { onSwitchProject(project.id); projectDropdownOpen = false; }}
									class="w-full text-left px-3 py-1.5 text-sm transition-colors {project.id === currentProject?.id ? 'text-accent bg-accent/10' : 'text-text-secondary hover:text-text hover:bg-bg-tertiary'}"
								>
									{project.name}
								</button>
							{/each}
						</div>
						<div class="border-t border-border py-1">
							<button onclick={() => { onCreateProject(); projectDropdownOpen = false; }} class="w-full text-left px-3 py-1.5 text-sm text-text-secondary hover:text-text hover:bg-bg-tertiary transition-colors">
								Create project...
							</button>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Search trigger -->
	<div class="px-3 pt-3 pb-1">
		<button
			onclick={onOpenSearch}
			class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-md border border-border bg-bg text-sm text-text-muted hover:text-text-secondary hover:border-text-muted transition-colors"
		>
			<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
			</svg>
			<span class="flex-1 text-left">Search...</span>
			<kbd class="query-kbd">&#8984;K</kbd>
		</button>
	</div>

	<!-- Main nav -->
	<nav class="flex-1 px-2 py-2 space-y-0.5 overflow-y-auto">
		{#each navItems as item}
			<a
				href={item.href}
				class="sidebar-nav-item {isActive(item.href) ? 'sidebar-nav-item-active' : ''}"
			>
				{#if item.icon === 'home'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="m2.25 12 8.954-8.955a1.126 1.126 0 0 1 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25" /></svg>
				{:else if item.icon === 'traces'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 0 1 0 3.75H5.625a1.875 1.875 0 0 1 0-3.75Z" /></svg>
				{:else if item.icon === 'sessions'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M20.25 8.511c.884.284 1.5 1.128 1.5 2.097v4.286c0 1.136-.847 2.1-1.98 2.193-.34.027-.68.052-1.02.072v3.091l-3-3c-1.354 0-2.694-.055-4.02-.163a2.115 2.115 0 0 1-.825-.242m9.345-8.334a2.126 2.126 0 0 0-.476-.095 48.64 48.64 0 0 0-8.048 0c-1.131.094-1.976 1.057-1.976 2.192v4.286c0 .837.46 1.58 1.155 1.951m9.345-8.334V6.637c0-1.621-1.152-3.026-2.76-3.235A48.455 48.455 0 0 0 11.25 3c-2.115 0-4.198.137-6.24.402-1.608.209-2.76 1.614-2.76 3.235v6.226c0 1.621 1.152 3.026 2.76 3.235.577.075 1.157.14 1.74.194V21l4.155-4.155" /></svg>
				{:else if item.icon === 'review'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 0 0 2.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 0 0-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75 2.25 2.25 0 0 0-.1-.664m-5.8 0A2.251 2.251 0 0 1 13.5 2.25H15a2.25 2.25 0 0 1 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25ZM6.75 12h.008v.008H6.75V12Zm0 3h.008v.008H6.75V15Zm0 3h.008v.008H6.75V18Z" /></svg>
				{:else if item.icon === 'approvals'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
				{:else if item.icon === 'analytics'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z" /></svg>
				{:else if item.icon === 'search'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" /></svg>
				{:else if item.icon === 'datasets'}
					<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" /></svg>
				{/if}
				<span>{item.label}</span>
				{#if item.badge && pendingReviewCount > 0}
					<span class="ml-auto px-1.5 py-0.5 rounded-full bg-warning/15 text-warning text-[10px] font-medium leading-none">{pendingReviewCount}</span>
				{/if}
			</a>
		{/each}

		<!-- Settings section -->
		<div class="pt-4">
			<div class="sidebar-section-label">Settings</div>
			{#each settingsItems as item}
				<a
					href={item.href}
					class="sidebar-nav-item {isActive(item.href) && (item.href === '/settings' ? page.url.pathname === '/settings' : true) ? 'sidebar-nav-item-active' : ''}"
				>
					<span>{item.label}</span>
				</a>
			{/each}
		</div>
	</nav>

	<!-- Footer -->
	<div class="border-t border-border px-3 py-2 space-y-1">
		<button
			onclick={onToggleTheme}
			class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-md text-sm text-text-muted hover:text-text hover:bg-bg-tertiary transition-colors"
		>
			{#if theme === 'light'}
				<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m0 13.5V21m8.25-9H18m-13.5 0H3m15.114 6.364-1.591-1.591M7.477 7.477 5.886 5.886m12.228 0-1.591 1.591M7.477 16.523l-1.591 1.591M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" /></svg>
			{:else if theme === 'dark'}
				<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0 1 12 21a9 9 0 0 1 0-18c.338 0 .671.019 1 .056a7.5 7.5 0 0 0 8.752 11.946Z" /></svg>
			{:else}
				<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9 17.25V15m0 0V9m0 6h6m-6 0H5.25m13.5 0H15m0 0V9m0 6v2.25M3.75 7.5h16.5M3.75 16.5h16.5" /></svg>
			{/if}
			<span>{themeLabel}</span>
		</button>

		{#if isCloudMode && authMe}
			<button
				onclick={onLogout}
				class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-md text-sm text-text-muted hover:text-danger hover:bg-danger/5 transition-colors"
			>
				<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" /></svg>
				<span>Log out</span>
			</button>
		{/if}
	</div>
</aside>
