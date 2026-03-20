<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getStats, getAuthConfig, getAuthMe, getProjects, createProject, deleteProject, switchProject, logout, subscribeEvents, getAllQueueItems, type Stats, type AuthConfig, type AuthMe, type Project } from '$lib/api';
	import SearchModal from '$lib/components/SearchModal.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
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
	let showCommandHelp = $state(false);
	type ThemeMode = 'dark' | 'light' | 'system';
	let theme: ThemeMode = $state('system');

	const THEME_KEY = 'traceway:theme';

	function handleGlobalKeydown(e: KeyboardEvent) {
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

		let unsub: (() => void) | null = null;
		let interval: ReturnType<typeof setInterval> | null = null;

		if (!isAuthPage) {
			getStats().then((s) => (stats = s)).catch(() => {});
			getAllQueueItems('pending').then((r) => (pendingReviewCount = r.items.length)).catch(() => {});

			unsub = subscribeEvents((event) => {
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

			interval = setInterval(() => {
				getStats().then((s) => (stats = s)).catch(() => {});
			}, 5000);
		}

		return () => {
			if (unsub) unsub();
			if (interval) clearInterval(interval);
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
			location.reload();
		} catch (e) {
			console.error('Failed to switch project', e);
		}
	}

	function openNewProjectModal() {
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
			<button onclick={() => location.reload()} class="btn-secondary mt-2">
				Retry
			</button>
		</div>
	</div>
{:else if needsLogin}
	<div class="min-h-screen flex items-center justify-center bg-bg">
		<div class="text-text-muted text-sm">Redirecting to login...</div>
	</div>
{:else}
	<div class="flex min-h-screen">
		<Sidebar
			{authMe}
			{projects}
			{currentProject}
			{pendingReviewCount}
			{isCloudMode}
			{theme}
			onToggleTheme={toggleTheme}
			onOpenSearch={() => (searchOpen = true)}
			onSwitchProject={handleSwitchProject}
			onCreateProject={openNewProjectModal}
			onLogout={handleLogout}
		/>
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
						<p class="text-xs text-danger">{newProjectError}</p>
					{/if}

					<div class="flex justify-end gap-2 pt-1">
						<button type="button" onclick={() => showNewProjectModal = false} class="btn-ghost h-7 text-xs" disabled={newProjectLoading}>
							Cancel
						</button>
						<button type="submit" class="btn-primary h-7 text-xs" disabled={newProjectLoading || !newProjectName.trim()}>
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
			<div class="bg-bg-secondary border border-border rounded-lg w-full max-w-md mx-4 p-4 space-y-3">
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
