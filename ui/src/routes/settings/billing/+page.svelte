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
				try {
					org = await getOrg();
				} catch {}
			}, 2000);
		}
	});
</script>

<div class="w-full space-y-5">
	<div>
		<h1 class="text-lg font-semibold tracking-tight">Billing</h1>
		<p class="text-text-muted text-[12px] mt-0.5">Manage your plan and usage.</p>
	</div>

	{#if error}
		<div class="alert-danger">
			{error}
		</div>
	{/if}

	{#if loading}
		<div class="text-text-muted text-sm py-8 text-center">Loading...</div>
	{:else if org}
		<div class="grid grid-cols-1 lg:grid-cols-[160px_minmax(0,1fr)] gap-3.5 items-start">
			<aside class="hidden lg:block">
				<div class="app-toolbar-shell rounded-xl p-1.5 space-y-0.5 sticky top-18">
					<div class="px-2 py-1 text-[11px] rounded-md border border-border/70 bg-bg-tertiary/60 text-text">Plan &amp; Billing</div>
				</div>
			</aside>

			<div class="space-y-3.5">
				<div class="table-float p-3.5">
					<div class="flex items-center justify-between gap-4">
						<div>
							<div class="label-micro uppercase tracking-wide">Current plan</div>
							<div class="text-[1.65rem] leading-none font-semibold text-text capitalize mt-0.5">{org.plan}</div>
						</div>
						<div class="grid grid-cols-3 gap-4 text-right">
							<div>
								<div class="text-[11px] text-text-muted">Capacity</div>
								<div class="text-[11px] text-text-secondary mt-0.5">{formatNumber(org.plan_limits.spans_per_month)} spans/mo</div>
							</div>
							<div>
								<div class="text-[11px] text-text-muted">Retention</div>
								<div class="text-[11px] text-text-secondary mt-0.5">{org.plan_limits.retention_days} days</div>
							</div>
							<div>
								<div class="text-[11px] text-text-muted">Team</div>
								<div class="text-[11px] text-text-secondary mt-0.5">{org.plan_limits.max_team_members} members</div>
							</div>
						</div>
					</div>
				</div>

				<div class="grid grid-cols-1 xl:grid-cols-3 gap-2.5">
					{#each plans as plan}
						{@const isCurrent = org.plan === plan.id}
						<section class="table-float p-3.5 space-y-2.5 border {isCurrent ? 'border-accent/65' : ''}">
							<div class="flex items-start justify-between gap-2">
								<div>
									<div class="text-[15px] font-semibold text-text">{plan.name}</div>
									<div class="mt-1">
										<span class="text-[2rem] leading-none font-semibold text-text">{plan.price}</span>
										<span class="text-[13px] text-text-muted">{plan.period}</span>
									</div>
								</div>
								{#if !isCurrent && plan.id !== 'free'}
									<span class="text-[10px] text-text-muted border border-border/70 rounded-full px-2 py-0.5">Upgrade</span>
								{/if}
							</div>

							<ul class="space-y-1.5 min-h-[132px]">
								{#each plan.features as feature}
									<li class="flex items-start gap-2 text-[14px] text-text-secondary">
										<svg class="w-4 h-4 shrink-0 mt-0.5 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
											<path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
										</svg>
										{feature}
									</li>
								{/each}
							</ul>

							{#if isCurrent}
								<div class="text-center text-[11px] text-text py-1.5 border border-border/70 rounded-lg bg-bg-tertiary/50">Current plan</div>
							{:else if plan.id === 'free'}
								<div class="h-8"></div>
							{:else}
								<button
									onclick={() => startCheckout(plan.id)}
									disabled={!!checkoutLoading}
									class="btn-primary w-full disabled:cursor-not-allowed"
								>
									{checkoutLoading === plan.id ? 'Redirecting...' : `Upgrade to ${plan.name}`}
								</button>
							{/if}
						</section>
					{/each}
				</div>

				<div class="table-float p-4 grid grid-cols-1 md:grid-cols-[1fr_1fr] gap-4 items-center">
					<div>
						<div class="text-lg font-semibold text-text">Enterprise</div>
						<div class="text-sm text-text-muted mt-1">Unlimited spans, 365-day retention, SSO, dedicated support, custom legal terms.</div>
					</div>
					<div class="flex md:justify-end">
						<a href="mailto:andrew@traceway.ai" class="btn-secondary border-accent/70 text-accent hover:bg-accent/10">Contact us</a>
					</div>
				</div>

				<p class="text-xs text-text-muted">Plans are billed monthly via <a href="https://polar.sh" target="_blank" rel="noopener noreferrer" class="text-accent hover:underline">Polar</a>.</p>
			</div>
		</div>
	{/if}
</div>
