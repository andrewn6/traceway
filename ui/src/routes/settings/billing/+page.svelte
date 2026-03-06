<script lang="ts">
	import { createBillingCheckout, getOrg, type OrgInfo } from '$lib/api';
	import { onMount } from 'svelte';

	let org: OrgInfo | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let checkoutLoading = $state('');

	// Plan display info
	const plans = [
		{
			id: 'free',
			name: 'Free',
			price: '$0',
			period: '/mo',
			features: ['10K spans/month', '7-day retention', '1 team member', '1 API key', 'Community support'],
		},
		{
			id: 'pro',
			name: 'Pro',
			price: '$20',
			period: '/user/mo',
			features: ['1M spans/month', '30-day retention', '5 team members', '5 API keys', 'Email support'],
		},
		{
			id: 'team',
			name: 'Team',
			price: '$100',
			period: '/user/mo',
			features: ['10M spans/month', '90-day retention', '50 team members', 'Unlimited API keys', 'Priority support'],
		},
	];

	function formatNumber(n: number): string {
		if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(0)}B`;
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
		return n.toString();
	}

	async function startCheckout(plan: string) {
		if (plan !== 'pro' && plan !== 'team') return;
		checkoutLoading = plan;
		error = '';
		try {
			const data = await createBillingCheckout(plan);
			if (data.url) {
				window.location.href = data.url;
			} else {
				throw new Error('No checkout URL returned');
			}
		} catch (e: any) {
			error = e.message || 'Failed to start checkout';
		} finally {
			checkoutLoading = '';
		}
	}

	onMount(async () => {
		try {
			org = await getOrg();
		} catch {
			error = 'Failed to load organization info';
		}
		loading = false;

		// Show success message if returning from checkout
		const params = new URLSearchParams(window.location.search);
		if (params.get('checkout') === 'success') {
			// Refresh org data to get updated plan
			setTimeout(async () => {
				try { org = await getOrg(); } catch {}
			}, 2000);
		}
	});
</script>

<div class="max-w-4xl space-y-6">
	<div>
		<h1 class="text-xl font-bold">Billing</h1>
		<p class="text-text-muted text-sm mt-1">Manage your plan and usage.</p>
	</div>

	{#if error}
		<div class="bg-danger/10 border border-danger/30 rounded px-3 py-2 text-danger text-sm">
			{error}
		</div>
	{/if}

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if org}
		<!-- Current plan summary -->
		<div class="bg-bg-secondary border border-border rounded p-4 space-y-2">
			<div class="flex items-center justify-between">
				<div>
					<div class="text-sm text-text-secondary">Current Plan</div>
					<div class="text-lg font-bold text-text capitalize">{org.plan}</div>
				</div>
				<div class="text-right space-y-0.5">
					<div class="text-xs text-text-muted">
						{formatNumber(org.plan_limits.spans_per_month)} spans/mo
					</div>
					<div class="text-xs text-text-muted">
						{org.plan_limits.retention_days}-day retention
					</div>
					<div class="text-xs text-text-muted">
						{org.plan_limits.max_team_members} team member{org.plan_limits.max_team_members !== 1 ? 's' : ''}
					</div>
				</div>
			</div>
		</div>

		<!-- Plan cards -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			{#each plans as plan}
				{@const isCurrent = org.plan === plan.id}
				<div
					class="bg-bg-secondary border rounded p-5 space-y-4 flex flex-col
						{isCurrent ? 'border-accent' : 'border-border'}"
				>
					<div>
						<div class="text-sm font-semibold text-text">{plan.name}</div>
						<div class="mt-1">
							<span class="text-2xl font-bold text-text">{plan.price}</span>
							<span class="text-sm text-text-muted">{plan.period}</span>
						</div>
					</div>

					<ul class="flex-1 space-y-1.5">
						{#each plan.features as feature}
							<li class="flex items-start gap-2 text-sm text-text-secondary">
								<svg class="w-4 h-4 shrink-0 mt-0.5 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
									<path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
								</svg>
								{feature}
							</li>
						{/each}
					</ul>

					{#if isCurrent}
						<div class="text-center text-xs text-accent font-semibold py-2 border border-accent/30 rounded bg-accent/5">
							Current plan
						</div>
					{:else if plan.id === 'free'}
						<div class="text-center text-xs text-text-muted py-2">
							<!-- No action for free if they're on a paid plan -->
						</div>
					{:else}
						<button
							onclick={() => startCheckout(plan.id)}
							disabled={!!checkoutLoading}
							class="block w-full text-center px-4 py-2 text-sm bg-accent text-bg font-semibold rounded hover:bg-accent/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
						>
							{checkoutLoading === plan.id ? 'Redirecting...' : `Upgrade to ${plan.name}`}
						</button>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Enterprise -->
		<div class="bg-bg-secondary border border-border rounded p-4 flex items-center justify-between">
			<div>
				<div class="text-sm font-semibold text-text">Enterprise</div>
				<div class="text-xs text-text-muted mt-0.5">Unlimited spans, 365-day retention, SSO, dedicated support</div>
			</div>
			<a
				href="mailto:andrew@traceway.ai"
				class="px-4 py-2 text-sm border border-accent text-accent font-semibold rounded hover:bg-accent/10 transition-colors"
			>
				Contact us
			</a>
		</div>

		<p class="text-xs text-text-muted">
			Plans are billed monthly via <a href="https://polar.sh" target="_blank" rel="noopener noreferrer" class="text-accent hover:underline">Polar</a>.
		</p>
	{/if}
</div>
