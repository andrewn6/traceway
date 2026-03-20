<script lang="ts">
	import { page } from '$app/state';
	let { children } = $props();

	const tabs = [
		{ href: '/settings', label: 'General' },
		{ href: '/settings/providers', label: 'Providers' },
		{ href: '/settings/api-keys', label: 'API Keys' },
		{ href: '/settings/team', label: 'Team' },
		{ href: '/settings/usage', label: 'Usage' },
		{ href: '/settings/billing', label: 'Billing' }
	];

	function isActive(href: string): boolean {
		if (href === '/settings') return page.url.pathname === '/settings';
		return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
	}
</script>

<div class="app-shell-wide space-y-4 motion-rise-in">
	<nav class="flex items-center gap-1 border-b border-border/40 pb-2" aria-label="Settings sections">
		{#each tabs as tab}
			<a
				href={tab.href}
				class="query-chip text-[12px] {isActive(tab.href) ? 'query-chip-active' : ''}"
			>
				{tab.label}
			</a>
		{/each}
	</nav>

	<div>
		{@render children()}
	</div>
</div>
