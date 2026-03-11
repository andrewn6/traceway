<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getStats, getAuthConfig, getAuthMe, getProjects, createProject, deleteProject, switchProject, logout, subscribeEvents, getAllQueueItems, type Stats, type AuthConfig, type AuthMe, type Project } from '$lib/api';
	import SearchModal from '$lib/components/SearchModal.svelte';
	import TracewayWordmark from '$lib/components/TracewayWordmark.svelte';
	import { onMount } from 'svelte';

	let { children } = $props();

	let stats: Stats = $state({ trace_count: 0, span_count: 0 });
	let connected = $state(false);
	let authConfig: AuthConfig = $state({ mode: 'cloud', features: [] });
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

	// Cmd+K search modal
	let searchOpen = $state(false);
	let globalQueryText = $state('');
	let globalQueryInputEl: HTMLInputElement | null = $state(null);
	let showCommandHelp = $state(false);
	type ThemeMode = 'dark' | 'light' | 'system';
	let theme: ThemeMode = $state('system');

	const THEME_KEY = 'traceway:theme';

	function isTypingTarget(target: EventTarget | null): boolean {
		const el = target as HTMLElement | null;
		if (!el) return false;
		const tag = el.tagName;
		if (el.isContentEditable) return true;
		if (el.closest('[contenteditable="true"]')) return true;
		return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
	}

	function handleGlobalKeydown(e: KeyboardEvent) {
		if (!e.metaKey && !e.ctrlKey && !e.altKey && e.key === '/' && globalQueryInputEl && !isTypingTarget(e.target)) {
			e.preventDefault();
			globalQueryInputEl?.focus();
			globalQueryInputEl?.select();
			return;
		}

		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			searchOpen = !searchOpen;
		}
	}

	function resolveSystemTheme(): 'dark' | 'light' {
		return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
	}

	function applyTheme(nextTheme: ThemeMode) {
		theme = nextTheme;
		const effectiveTheme = nextTheme === 'system' ? resolveSystemTheme() : nextTheme;
		document.documentElement.dataset.theme = effectiveTheme;
		localStorage.setItem(THEME_KEY, nextTheme);
	}

	function toggleTheme() {
		if (theme === 'system') {
			applyTheme('light');
		} else if (theme === 'light') {
			applyTheme('dark');
		} else {
			applyTheme('system');
		}
	}

	function runGlobalQuery() {
		const q = globalQueryText.trim();
		if (page.url.pathname.startsWith('/datasets')) {
			if (q) {
				goto(`/datasets?q=${encodeURIComponent(q)}`);
			} else {
				goto('/datasets');
			}
			return;
		}
		if (q) {
			goto(`/query?q=${encodeURIComponent(q)}`);
		} else {
			goto('/query');
		}
	}

	function isSectionActive(href: string): boolean {
		if (href === '/traces') return page.url.pathname === '/traces' || page.url.pathname.startsWith('/traces/') || page.url.pathname === '/spans';
		if (href === '/settings') return page.url.pathname === '/settings' || page.url.pathname.startsWith('/settings/');
		if (href === '/datasets') return page.url.pathname === '/datasets' || page.url.pathname.startsWith('/datasets/');
		if (href === '/approvals') return page.url.pathname === '/approvals';
		if (href === '/analytics') return page.url.pathname === '/analytics' || page.url.pathname.startsWith('/analytics/');
		if (href === '/query') return page.url.pathname === '/query';
		return page.url.pathname === href;
	}

	const topNavTabs = [
		{ href: '/', label: 'Home' },
		{ href: '/traces', label: 'Traces' },
		{ href: '/review', label: 'Review' },
		{ href: '/approvals', label: 'Approvals' },
		{ href: '/analytics', label: 'Analytics' },
		{ href: '/query', label: 'Search' },
		{ href: '/datasets', label: 'Datasets' },
		{ href: '/settings', label: 'Settings' }
	] as const;

	const workspaceLabel = $derived.by(() => currentProject?.name || 'default');

	const commandPlaceholder = $derived.by(() => {
		if (page.url.pathname.startsWith('/traces/')) return 'Search this trace context... status:failed model:gpt-4o';
		if (page.url.pathname.startsWith('/datasets')) return 'Search datasets... name:eval description:qa';
		if (page.url.pathname.startsWith('/settings')) return 'Jump into query... status:failed provider:openai';
		return 'Search traces... model:gpt-4o status:failed';
	});

	const commandActionLabel = $derived.by(() => (page.url.pathname.startsWith('/datasets') ? 'Search' : 'Query'));

	const railWidthClass = $derived.by(() =>
		page.url.pathname.startsWith('/traces/') ? 'w-[min(1120px,calc(100vw-1.25rem))]' : 'w-[min(980px,calc(100vw-1.25rem))]'
	);

	const themeLabel = $derived.by(() => (theme === 'system' ? 'Auto' : theme === 'light' ? 'Light' : 'Dark'));

	// Auth pages don't need sidebar or auth check
	const authPages = ['/login', '/signup', '/accept-invite', '/forgot-password', '/reset-password'];
	const isAuthPage = $derived(authPages.includes(page.url.pathname));
	const isCloudMode = $derived(authConfig.mode === 'cloud');
	const isAuthenticated = $derived(authMe !== null);
	const needsLogin = $derived(authChecked && isCloudMode && !isAuthPage && !isAuthenticated && !apiUnreachable);
	const apiTarget = (import.meta.env.VITE_API_URL as string | undefined) || '/api';

	$effect(() => {
		if (needsLogin) {
			goto('/login');
		}
	});

	onMount(() => {
		document.addEventListener('keydown', handleGlobalKeydown);

		const savedTheme = localStorage.getItem(THEME_KEY);
		if (savedTheme === 'light' || savedTheme === 'dark' || savedTheme === 'system') {
			applyTheme(savedTheme);
		} else {
			applyTheme('system');
		}

		const media = window.matchMedia('(prefers-color-scheme: light)');
		const onSystemThemeChange = () => {
			if (theme === 'system') {
				document.documentElement.dataset.theme = resolveSystemTheme();
			}
		};
		media.addEventListener('change', onSystemThemeChange);

		// Load auth config first, then check auth for non-auth pages
		getAuthConfig()
			.then(async (c) => {
				authConfig = c;

				if (!isAuthPage) {
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
				// Fail closed: never fall through into an unauthenticated app shell.
				apiUnreachable = true;
				authChecked = true;
			});

		getStats().then((s) => (stats = s)).catch(() => {});
		getAllQueueItems('pending').then((r) => (pendingReviewCount = r.items.length)).catch(() => {});

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
			document.removeEventListener('keydown', handleGlobalKeydown);
			media.removeEventListener('change', onSystemThemeChange);
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
				Unable to connect to the Traceway backend at <code class="text-accent">{apiTarget}</code>.
				For local dev, run <code class="text-accent">cd backend/app && encore run</code>.
			</div>
			<button onclick={() => location.reload()} class="mt-2 px-4 py-1.5 text-sm bg-accent/10 text-accent rounded hover:bg-accent/20 transition-colors">
				Retry
			</button>
		</div>
	</div>
{:else if needsLogin}
	<div class="min-h-screen flex items-center justify-center bg-bg">
		<div class="text-text-muted text-sm">Redirecting to login...</div>
	</div>
{:else}
	<div class="min-h-screen relative">
		<!-- Main content -->
		<main class="min-h-screen min-w-0 overflow-auto">
			<div class="p-3.5 lg:p-5 space-y-3.5">
				<div class="sticky top-3 z-30">
					<div class="topnav-shell app-shell-wide rounded-xl px-2 py-1.5 flex items-center gap-1.5">
						<div class="flex items-center gap-1 min-w-0 relative">
							<a href="/" class="inline-flex items-center px-2 h-8 rounded-lg border border-border/65 bg-bg-tertiary/45 hover:border-border transition-colors">
								<TracewayWordmark className="h-4.5 w-auto text-text" />
							</a>
							{#if isCloudMode && projects.length > 0}
								<button
									onclick={() => (projectDropdownOpen = !projectDropdownOpen)}
									class="inline-flex items-center gap-2 px-2.5 h-8 rounded-lg border border-border/65 bg-bg-tertiary/45 text-[12px] text-text-secondary hover:text-text hover:border-border transition-colors"
									aria-label="Project switcher"
								>
									<span class="w-4 h-4 rounded-md bg-accent/15 border border-accent/35 text-accent inline-flex items-center justify-center text-[10px]">◆</span>
									<span class="truncate max-w-44">{currentProject?.name ?? workspaceLabel}</span>
									<span class="px-1.5 py-0.5 rounded bg-bg/55 border border-border/60 text-[11px] text-text-muted">Owner</span>
									<svg class="w-3 h-3 text-text-muted transition-transform {projectDropdownOpen ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m19.5 8.25-7.5 7.5-7.5-7.5" /></svg>
								</button>

								{#if projectDropdownOpen}
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div class="fixed inset-0 z-40" onclick={() => (projectDropdownOpen = false)} onkeydown={(e) => e.key === 'Escape' && (projectDropdownOpen = false)}></div>
									<div class="absolute left-0 top-full mt-2 z-50 w-[360px] bg-bg-secondary border border-border rounded-xl shadow-xl overflow-hidden">
										<div class="p-3 border-b border-border/70">
											<div class="text-[11px] text-text-muted uppercase tracking-[0.12em] mb-2">Teams</div>
											<div class="space-y-1">
												{#each projects as project}
													<button
														onclick={() => handleSwitchProject(project.id)}
														class="w-full text-left px-3 py-2 rounded-lg border transition-colors {project.id === currentProject?.id ? 'bg-accent text-bg border-accent/70' : 'border-border/55 text-text-secondary hover:text-text hover:bg-bg-tertiary/45'}"
													>
														<div class="font-medium text-sm">{project.name}</div>
														<div class="text-xs opacity-80 truncate">{project.id}</div>
													</button>
												{/each}
											</div>
										</div>
										<div class="p-3 border-b border-border/70 space-y-1">
											<a href="/settings/team" class="block px-2 py-1.5 rounded-md text-sm text-text-secondary hover:text-text hover:bg-bg-tertiary/45 transition-colors">Team members...</a>
											<button onclick={openNewProjectModal} class="w-full text-left px-2 py-1.5 rounded-md text-sm text-text-secondary hover:text-text hover:bg-bg-tertiary/45 transition-colors">Create team...</button>
										</div>
										<div class="p-3 flex items-center justify-between text-xs text-text-muted">
											<span>User ID: {authMe?.user_id ?? 'unknown'}</span>
										</div>
										<div class="p-3 border-t border-border/70">
											<button onclick={handleLogout} class="text-sm text-danger hover:text-danger/80 transition-colors">Log out</button>
										</div>
									</div>
								{/if}
							{/if}

							{#if page.url.pathname !== '/'}
							<div class="hidden lg:block text-[11px] text-text-muted truncate">{page.url.pathname}</div>
							{/if}
						</div>

						<div class="flex-1"></div>

						<button
							onclick={toggleTheme}
							class="hidden sm:inline-flex items-center gap-1.5 h-8 px-2.5 rounded-lg border border-border/65 bg-bg-tertiary/35 text-[12px] text-text-secondary hover:text-text hover:border-border transition-colors"
							title="Toggle theme"
						>
							<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
								{#if theme === 'light'}
									<path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m0 13.5V21m8.25-9H18m-13.5 0H3m15.114 6.364-1.591-1.591M7.477 7.477 5.886 5.886m12.228 0-1.591 1.591M7.477 16.523l-1.591 1.591M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
								{:else if theme === 'dark'}
									<path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0 1 12 21a9 9 0 0 1 0-18c.338 0 .671.019 1 .056a7.5 7.5 0 0 0 8.752 11.946Z" />
								{:else}
									<path stroke-linecap="round" stroke-linejoin="round" d="M9 17.25V15m0 0V9m0 6h6m-6 0H5.25m13.5 0H15m0 0V9m0 6v2.25M3.75 7.5h16.5M3.75 16.5h16.5" />
								{/if}
							</svg>
							<span>{themeLabel}</span>
						</button>

						<div class="flex items-center gap-0.5 rounded-lg border border-border/65 bg-bg-tertiary/35 p-0.5 overflow-x-auto max-w-[64vw] sm:max-w-none">
							{#each topNavTabs as tab}
								<a href={tab.href} class="topnav-tab {isSectionActive(tab.href) ? 'topnav-tab-active' : ''}">
									{tab.label}
									{#if tab.href === '/review' && pendingReviewCount > 0}
										<span class="ml-1 px-1 py-0.5 rounded bg-warning/18 text-warning text-[9px] leading-none">{pendingReviewCount}</span>
									{/if}
								</a>
							{/each}
						</div>
					</div>
				</div>
				<div class="app-page-shell app-shell-wide rounded-2xl p-4 lg:p-5">
					{@render children()}
				</div>
			</div>
		</main>

		{#if authChecked && !isAuthPage && !apiUnreachable && isAuthenticated && page.url.pathname !== '/query'}
			<div class="fixed left-1/2 bottom-3 -translate-x-1/2 z-40 {railWidthClass} command-rail-anim">
				<div class="query-command-shell rounded-xl p-2">
					<div class="flex items-center gap-1.5 flex-wrap">
						<button class="query-chip {isSectionActive('/query') ? 'query-chip-active' : ''}" onclick={() => goto('/query')}>Query</button>
						<button class="query-chip {isSectionActive('/traces') ? 'query-chip-active' : ''}" onclick={() => goto('/traces')}>Traces</button>
						<button class="query-chip hidden sm:inline-flex {isSectionActive('/review') ? 'query-chip-active' : ''}" onclick={() => goto('/review')}>Review</button>
						<button class="query-chip hidden sm:inline-flex {isSectionActive('/approvals') ? 'query-chip-active' : ''}" onclick={() => goto('/approvals')}>Approvals</button>
						<button class="query-chip hidden sm:inline-flex {isSectionActive('/analytics') ? 'query-chip-active' : ''}" onclick={() => goto('/analytics')}>Analytics</button>
						<button class="query-chip hidden sm:inline-flex {isSectionActive('/datasets') ? 'query-chip-active' : ''}" onclick={() => goto('/datasets')}>Datasets</button>
						<button class="query-chip hidden md:inline-flex" onclick={openNewProjectModal}>New project</button>
						<button class="query-chip hidden md:inline-flex" onclick={() => (searchOpen = true)}>Search</button>
						<button class="query-chip hidden md:inline-flex" onclick={() => (showCommandHelp = true)}>Shortcuts</button>

						<div class="flex items-center flex-1 rounded-lg border border-border/45 bg-bg/30 min-w-[220px] focus-within:border-accent/55 transition-colors">
							<div class="pl-3 pr-2 text-text-muted/80">
								<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
									<path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" />
								</svg>
							</div>
							<input
								type="text"
								bind:value={globalQueryText}
								bind:this={globalQueryInputEl}
								maxlength={320}
								onkeydown={(e) => e.key === 'Enter' && runGlobalQuery()}
								placeholder={commandPlaceholder}
								class="w-full min-w-0 overflow-hidden text-ellipsis whitespace-nowrap bg-transparent py-1.5 text-[13px] text-text placeholder:text-text-muted/45 focus:outline-none"
							/>
						</div>

						<button onclick={runGlobalQuery} class="px-3.5 py-1.5 bg-accent text-bg rounded-lg text-xs font-semibold tracking-wide hover:bg-accent/90 transition-colors">
							{commandActionLabel}
						</button>
						<div class="hidden lg:block text-[10px] text-text-muted/80 pl-0.5"><span class="query-kbd">/</span></div>
					</div>
				</div>
			</div>
		{/if}
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

	{#if showCommandHelp}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="fixed inset-0 z-[100] flex items-center justify-center bg-black/45"
			onclick={(e) => { if (e.target === e.currentTarget) showCommandHelp = false; }}
			onkeydown={(e) => { if (e.key === 'Escape') showCommandHelp = false; }}
		>
			<div class="query-command-shell rounded-xl w-full max-w-md mx-4 p-4 space-y-3">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-text">Shortcuts</h3>
					<button class="text-xs text-text-muted hover:text-text" onclick={() => (showCommandHelp = false)}>Close</button>
				</div>
				<div class="space-y-2 text-xs">
					<div class="flex items-center justify-between"><span class="text-text-secondary">Open search</span><kbd class="query-kbd">Cmd K</kbd></div>
					<div class="flex items-center justify-between"><span class="text-text-secondary">Go to query</span><kbd class="query-kbd">/query</kbd></div>
					<div class="flex items-center justify-between"><span class="text-text-secondary">Go to traces</span><kbd class="query-kbd">/traces</kbd></div>
				</div>
			</div>
		</div>
	{/if}

{/if}

<SearchModal bind:open={searchOpen} />
