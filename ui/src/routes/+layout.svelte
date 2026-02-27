<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getStats, getAuthConfig, getAuthMe, getProjects, createProject, deleteProject, switchProject, logout, subscribeEvents, getAllQueueItems, type Stats, type AuthConfig, type AuthMe, type Project } from '$lib/api';
	import { onMount } from 'svelte';

	let { children } = $props();

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let connected = $state(false);
	let authConfig: AuthConfig = $state({ mode: 'local', features: [] });
	let authMe: AuthMe | null = $state(null);
	let authChecked = $state(false);
	let apiUnreachable = $state(false);

	// Project state
	let projects: Project[] = $state([]);
	let projectDropdownOpen = $state(false);
	let showNewProjectModal = $state(false);
	let newProjectName = $state('');
	let newProjectError = $state('');
	let newProjectLoading = $state(false);
	let currentProject = $derived(
		projects.find(p => p.id === authMe?.project_id) || projects[0] || null
	);

	// Review queue pending count for sidebar badge
	let pendingReviewCount = $state(0);

	// Auth pages don't need sidebar or auth check
	const authPages = ['/login', '/signup', '/accept-invite', '/forgot-password', '/reset-password'];
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
						// Load projects for the org
						try {
							projects = await getProjects();
						} catch {
							// Projects not available yet (e.g. migration not run)
						}
					} catch {
						// Not authenticated - redirect to login
						goto('/login');
						return;
					}
				}

				authChecked = true;
			})
			.catch(() => {
				// If VITE_API_URL was explicitly set (deployed platform), show error
				// instead of silently falling through to local mode
				if (import.meta.env.VITE_API_URL) {
					apiUnreachable = true;
				}
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
		getAllQueueItems('pending').then((r) => (pendingReviewCount = r.items.length)).catch(() => {});
			} else if (event.type === 'queue_item_updated') {
				// Refresh pending count when queue changes
				getAllQueueItems('pending').then((r) => (pendingReviewCount = r.items.length)).catch(() => {});
			} else if (event.type === 'cleared') {
				stats = { trace_count: 0, span_count: 0 };
				pendingReviewCount = 0;
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

	async function handleSwitchProject(projectId: string) {
		try {
			await switchProject(projectId);
			authMe = await getAuthMe();
			projectDropdownOpen = false;
			location.reload();
		} catch (e) {
			console.error('Failed to switch project', e);
		}
	}

	function openNewProjectModal() {
		projectDropdownOpen = false;
		newProjectName = '';
		newProjectError = '';
		newProjectLoading = false;
		showNewProjectModal = true;
	}

	async function handleCreateProject() {
		const name = newProjectName.trim();
		if (!name) {
			newProjectError = 'Project name is required';
			return;
		}
		newProjectLoading = true;
		newProjectError = '';
		try {
			const project = await createProject(name);
			projects = await getProjects();
			showNewProjectModal = false;
			// Auto-switch to the new project
			await handleSwitchProject(project.id);
		} catch (e: any) {
			newProjectError = e?.message || 'Failed to create project';
			newProjectLoading = false;
		}
	}

	async function handleDeleteProject(projectId: string, projectName: string) {
		if (!confirm(`Delete project "${projectName}"? This will permanently remove all its data.`)) return;
		try {
			await deleteProject(projectId);
			projects = await getProjects();
			// If we just deleted the active project, switch to default
			if (authMe?.project_id === projectId && projects.length > 0) {
				await handleSwitchProject(projects[0].id);
			}
		} catch (e: any) {
			alert(e?.message || 'Failed to delete project');
		}
	}

	interface NavItem {
		href: string;
		label: string;
		icon: string;
	}

	interface NavSection {
		label?: string;
		items: NavItem[];
	}

	const navSections = $derived((): NavSection[] => {
		const sections: NavSection[] = [
			{
				items: [
					{ href: '/', label: 'Dashboard', icon: 'dashboard' },
				]
			},
			{
				label: 'Observe',
				items: [
					{ href: '/traces', label: 'Traces', icon: 'trace' },
					{ href: '/query', label: 'Query', icon: 'query' },
					{ href: '/analytics', label: 'Analytics', icon: 'analysis' },
				]
			},
			{
				label: 'Evaluate',
				items: [
					{ href: '/datasets', label: 'Datasets', icon: 'dataset' },
					{ href: '/review', label: 'Review', icon: 'review' },
				]
			},
			{
				label: 'Configure',
				items: [
					{ href: '/settings/providers', label: 'Providers', icon: 'provider' },
					{ href: '/settings', label: 'Settings', icon: 'settings' },
					...(isCloudMode ? [
						{ href: '/settings/team', label: 'Team', icon: 'team' },
						{ href: '/settings/api-keys', label: 'API Keys', icon: 'key' },
						{ href: '/settings/billing', label: 'Billing', icon: 'billing' },
					] : []),
				]
			},
		];
		return sections;
	});

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		// For /settings, only exact match (not /settings/providers etc.)
		if (href === '/settings') return page.url.pathname === '/settings';
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
{:else if apiUnreachable}
	<div class="min-h-screen flex items-center justify-center bg-bg">
		<div class="text-center space-y-3">
			<div class="text-text font-bold text-lg">Cannot reach API</div>
			<div class="text-text-muted text-sm max-w-md">
				Unable to connect to the Traceway backend. Check that the daemon is running and <code class="text-accent">VITE_API_URL</code> is set correctly.
			</div>
			<button onclick={() => location.reload()} class="mt-2 px-4 py-1.5 text-sm bg-accent/10 text-accent rounded hover:bg-accent/20 transition-colors">
				Retry
			</button>
		</div>
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
			<nav class="flex-1 py-2 px-2">
				{#each navSections() as section, sIdx}
					{#if sIdx > 0}
						<div class="my-2"></div>
					{/if}

					{#if section.label}
						<div class="px-3 pt-2 pb-1 text-[10px] font-semibold text-text-muted/50 uppercase tracking-widest">{section.label}</div>
					{/if}

					<div class="space-y-0.5">
						{#each section.items as item}
							<a
								href={item.href}
								class="flex items-center gap-2.5 px-3 py-1.5 rounded text-[13px] transition-colors
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
									<path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z" />
								</svg>
							{:else if item.icon === 'settings'}
								<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
									<path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z" />
									<path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
								</svg>
							{:else if item.icon === 'provider'}
								<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
									<path stroke-linecap="round" stroke-linejoin="round" d="M14.25 6.087c0-.355.186-.676.401-.959.221-.29.349-.634.349-1.003 0-1.036-1.007-1.875-2.25-1.875s-2.25.84-2.25 1.875c0 .369.128.713.349 1.003.215.283.401.604.401.959v0a.64.64 0 0 1-.657.643 48.39 48.39 0 0 1-4.163-.3c.186 1.613.293 3.25.315 4.907a.656.656 0 0 1-.658.663v0c-.355 0-.676-.186-.959-.401a1.647 1.647 0 0 0-1.003-.349c-1.036 0-1.875 1.007-1.875 2.25s.84 2.25 1.875 2.25c.369 0 .713-.128 1.003-.349.283-.215.604-.401.959-.401v0c.31 0 .555.26.532.57a48.039 48.039 0 0 1-.642 5.056c1.518.19 3.058.309 4.616.354a.64.64 0 0 0 .657-.643v0c0-.355-.186-.676-.401-.959a1.647 1.647 0 0 1-.349-1.003c0-1.035 1.008-1.875 2.25-1.875 1.243 0 2.25.84 2.25 1.875 0 .369-.128.713-.349 1.003-.215.283-.4.604-.4.959v0c0 .333.277.599.61.58a48.1 48.1 0 0 0 5.427-.63 48.05 48.05 0 0 0 .582-4.717.532.532 0 0 0-.533-.57v0c-.355 0-.676.186-.959.401-.29.221-.634.349-1.003.349-1.035 0-1.875-1.007-1.875-2.25s.84-2.25 1.875-2.25c.37 0 .713.128 1.003.349.283.215.604.401.96.401v0a.656.656 0 0 0 .658-.663 48.422 48.422 0 0 0-.37-5.36c-1.886.342-3.81.574-5.766.689a.578.578 0 0 1-.61-.58Z" />
								</svg>
							{:else if item.icon === 'key'}
								<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
									<path stroke-linecap="round" stroke-linejoin="round" d="M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z" />
								</svg>
							{:else if item.icon === 'team'}
								<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
									<path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 0 0 2.625.372 9.337 9.337 0 0 0 4.121-.952 4.125 4.125 0 0 0-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 0 1 8.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0 1 11.964-3.07M12 6.375a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0Zm8.25 2.25a2.625 2.625 0 1 1-5.25 0 2.625 2.625 0 0 1 5.25 0Z" />
								</svg>
						{:else if item.icon === 'billing'}
							<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 8.25h19.5M2.25 9h19.5m-16.5 5.25h6m-6 2.25h3m-3.75 3h15a2.25 2.25 0 0 0 2.25-2.25V6.75A2.25 2.25 0 0 0 19.5 4.5h-15a2.25 2.25 0 0 0-2.25 2.25v10.5A2.25 2.25 0 0 0 4.5 19.5Z" />
							</svg>
						{:else if item.icon === 'review'}
							<svg class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
								<path stroke-linecap="round" stroke-linejoin="round" d="M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 0 0 2.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 0 0-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75 2.25 2.25 0 0 0-.1-.664m-5.8 0A2.251 2.251 0 0 1 13.5 2.25H15a2.25 2.25 0 0 1 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25ZM6.75 12h.008v.008H6.75V12Zm0 3h.008v.008H6.75V15Zm0 3h.008v.008H6.75V18Z" />
							</svg>
						{/if}
							{item.label}
							{#if item.icon === 'review' && pendingReviewCount > 0}
								<span class="ml-auto px-1.5 py-0.5 rounded bg-warning/20 text-warning text-[10px] leading-none">{pendingReviewCount}</span>
							{/if}
							</a>
						{/each}
					</div>
				{/each}
			</nav>

			<!-- Status footer -->
			<div class="px-3 py-3 border-t border-border space-y-2.5">
				{#if isCloudMode && projects.length > 0}
					<!-- Project switcher -->
					<div class="relative">
						<button
							onclick={() => projectDropdownOpen = !projectDropdownOpen}
							class="w-full flex items-center justify-between gap-1.5 px-2 py-1.5 rounded border border-border bg-bg text-left text-xs hover:border-text-muted/40 transition-colors cursor-pointer"
						>
							<div class="flex items-center gap-1.5 min-w-0">
								<span class="w-2 h-2 rounded-sm bg-accent shrink-0"></span>
								<span class="truncate text-text">{currentProject?.name ?? 'Select project'}</span>
							</div>
							<svg class="w-3 h-3 shrink-0 text-text-muted transition-transform {projectDropdownOpen ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								<path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" />
							</svg>
						</button>

						{#if projectDropdownOpen}
							<!-- Backdrop -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="fixed inset-0 z-40"
								onclick={() => projectDropdownOpen = false}
								onkeydown={(e) => e.key === 'Escape' && (projectDropdownOpen = false)}
							></div>
							<!-- Dropdown opens upward from footer -->
							<div class="absolute left-0 right-0 bottom-full mb-1 z-50 bg-bg-secondary border border-border rounded-md shadow-lg overflow-hidden">
								<div class="py-1 max-h-48 overflow-y-auto">
									{#each projects as project}
										<div class="group flex items-center">
											<button
												onclick={() => handleSwitchProject(project.id)}
												class="flex-1 flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-left transition-colors cursor-pointer
													{project.id === currentProject?.id
														? 'text-text bg-bg-tertiary'
														: 'text-text-secondary hover:text-text hover:bg-bg-tertiary/50'}"
											>
												<span class="w-2 h-2 rounded-sm shrink-0 {project.id === currentProject?.id ? 'bg-accent' : 'bg-text-muted/30'}"></span>
												<span class="truncate">{project.name}</span>
											</button>
											{#if project.slug !== 'default'}
												<button
													onclick={(e) => { e.stopPropagation(); handleDeleteProject(project.id, project.name); }}
													class="px-2 py-1.5 text-text-muted/0 group-hover:text-text-muted hover:!text-error transition-colors cursor-pointer"
													title="Delete project"
												>
													<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
														<path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
													</svg>
												</button>
											{/if}
										</div>
									{/each}
								</div>
								<!-- New project button -->
								<div class="border-t border-border">
									<button
										onclick={openNewProjectModal}
										class="w-full flex items-center gap-1.5 px-2.5 py-2 text-xs text-text-muted hover:text-text hover:bg-bg-tertiary/50 transition-colors cursor-pointer"
									>
										<svg class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
											<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
										</svg>
										New Project
									</button>
								</div>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Stats -->
				<div class="flex items-center gap-2 text-[11px] text-text-muted">
					<span class="w-1.5 h-1.5 rounded-full {connected ? 'bg-success' : 'bg-text-muted'}"></span>
					<span>{stats.trace_count} traces</span>
					<span class="text-text-muted/40">/</span>
					<span>{stats.span_count} spans</span>
				</div>

				<!-- Docs & Support -->
				<div class="flex items-center gap-3">
					<a
						href="https://docs.traceway.ai"
						target="_blank"
						rel="noopener noreferrer"
						class="flex items-center gap-1.5 text-[11px] text-text-muted hover:text-accent transition-colors"
					>
						<svg class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M12 6.042A8.967 8.967 0 0 0 6 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 0 1 6 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 0 1 6-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0 0 18 18a8.967 8.967 0 0 0-6 2.292m0-14.25v14.25" />
						</svg>
						Docs
					</a>
					<a
						href="mailto:support@traceway.ai"
						class="flex items-center gap-1.5 text-[11px] text-text-muted hover:text-accent transition-colors"
					>
						<svg class="w-3.5 h-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
							<path stroke-linecap="round" stroke-linejoin="round" d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 5.25h.008v.008H12v-.008Z" />
						</svg>
						Support
					</a>
				</div>

				{#if isCloudMode && authMe}
					<button
						onclick={handleLogout}
						class="w-full px-2 py-1.5 rounded border border-border bg-bg text-[11px] text-text-muted hover:text-text hover:border-text-muted/40 transition-colors cursor-pointer text-left"
					>
						Sign out
					</button>
				{/if}

				<!-- Branding -->
				<div class="text-[10px] text-text-muted/40 tracking-wide">
					TracewayAI &middot; 2026
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

	<!-- New Project Modal -->
	{#if showNewProjectModal}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
			onclick={(e) => { if (e.target === e.currentTarget) showNewProjectModal = false; }}
			onkeydown={(e) => { if (e.key === 'Escape') showNewProjectModal = false; }}
		>
			<div class="bg-bg-secondary border border-border rounded-lg shadow-xl w-full max-w-sm mx-4">
				<div class="px-5 pt-5 pb-4">
					<h3 class="text-sm font-semibold text-text">New Project</h3>
					<p class="text-xs text-text-muted mt-1">Projects share your org's usage quota.</p>
				</div>

				<form
					onsubmit={(e) => { e.preventDefault(); handleCreateProject(); }}
					class="px-5 pb-5 space-y-3"
				>
					<div>
						<label for="project-name" class="block text-xs font-medium text-text-secondary mb-1">Project name</label>
						<input
							id="project-name"
							type="text"
							bind:value={newProjectName}
							placeholder="e.g. Production, Staging"
							class="w-full px-3 py-2 text-sm bg-bg border border-border rounded-md text-text placeholder:text-text-muted/50 focus:outline-none focus:ring-1 focus:ring-accent focus:border-accent"
							disabled={newProjectLoading}
						/>
					</div>

					{#if newProjectError}
						<p class="text-xs text-error">{newProjectError}</p>
					{/if}

					<div class="flex justify-end gap-2 pt-1">
						<button
							type="button"
							onclick={() => showNewProjectModal = false}
							class="px-3 py-1.5 text-xs text-text-secondary hover:text-text transition-colors cursor-pointer"
							disabled={newProjectLoading}
						>
							Cancel
						</button>
						<button
							type="submit"
							class="px-3 py-1.5 text-xs bg-accent text-white rounded-md hover:bg-accent/90 transition-colors disabled:opacity-50 cursor-pointer"
							disabled={newProjectLoading || !newProjectName.trim()}
						>
							{newProjectLoading ? 'Creating...' : 'Create Project'}
						</button>
					</div>
				</form>
			</div>
		</div>
	{/if}
{/if}
