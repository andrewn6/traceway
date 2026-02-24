<script>
	import InView from '$lib/InView.svelte';
	import ArchDiagram from '$lib/ArchDiagram.svelte';

	let scrollY = $state(0);

	// Animated trace spans for hero background
	const heroSpans = [
		{ y: 12, x: 5, w: 88, h: 2.5, color: 'rgba(110,231,183,0.07)', delay: 0, dur: 2.5 },
		{ y: 18, x: 8, w: 22, h: 2, color: 'rgba(167,139,250,0.09)', delay: 0.3, dur: 1.8 },
		{ y: 18, x: 34, w: 14, h: 2, color: 'rgba(34,211,238,0.08)', delay: 0.8, dur: 1.4 },
		{ y: 18, x: 52, w: 38, h: 2, color: 'rgba(110,231,183,0.09)', delay: 1.2, dur: 2.0 },
		{ y: 24, x: 55, w: 18, h: 1.5, color: 'rgba(251,191,36,0.07)', delay: 1.5, dur: 1.6 },
		{ y: 24, x: 76, w: 12, h: 1.5, color: 'rgba(110,231,183,0.06)', delay: 1.9, dur: 1.2 },
		// Second trace group
		{ y: 36, x: 2, w: 92, h: 2.5, color: 'rgba(110,231,183,0.05)', delay: 0.5, dur: 2.8 },
		{ y: 42, x: 5, w: 30, h: 2, color: 'rgba(167,139,250,0.07)', delay: 0.8, dur: 1.6 },
		{ y: 42, x: 40, w: 50, h: 2, color: 'rgba(110,231,183,0.07)', delay: 1.4, dur: 2.2 },
		{ y: 48, x: 42, w: 20, h: 1.5, color: 'rgba(34,211,238,0.06)', delay: 1.8, dur: 1.3 },
		// Third trace group
		{ y: 60, x: 10, w: 78, h: 2.5, color: 'rgba(110,231,183,0.04)', delay: 1.0, dur: 2.6 },
		{ y: 66, x: 14, w: 16, h: 2, color: 'rgba(251,191,36,0.06)', delay: 1.3, dur: 1.5 },
		{ y: 66, x: 34, w: 48, h: 2, color: 'rgba(167,139,250,0.05)', delay: 1.7, dur: 2.0 },
		{ y: 72, x: 36, w: 24, h: 1.5, color: 'rgba(110,231,183,0.05)', delay: 2.0, dur: 1.4 },
		{ y: 72, x: 64, w: 16, h: 1.5, color: 'rgba(34,211,238,0.05)', delay: 2.3, dur: 1.1 },
		// Ghost traces at edges
		{ y: 82, x: 0, w: 95, h: 2, color: 'rgba(110,231,183,0.03)', delay: 0.2, dur: 3.0 },
		{ y: 88, x: 3, w: 40, h: 1.5, color: 'rgba(167,139,250,0.03)', delay: 0.6, dur: 2.0 },
	];
</script>

<svelte:window bind:scrollY={scrollY} />

<style>
	@keyframes span-appear {
		0% { transform: scaleX(0); opacity: 0; }
		20% { opacity: 1; }
		100% { transform: scaleX(1); opacity: 1; }
	}
	@keyframes span-pulse {
		0%, 100% { opacity: 0.6; }
		50% { opacity: 1; }
	}
	@keyframes scan-line {
		0% { left: -2px; opacity: 0; }
		10% { opacity: 1; }
		90% { opacity: 1; }
		100% { left: 100%; opacity: 0; }
	}
	.hero-span {
		transform-origin: left center;
		animation: span-appear var(--dur) cubic-bezier(0.16, 1, 0.3, 1) var(--delay) both,
		           span-pulse 4s ease-in-out calc(var(--delay) + var(--dur)) infinite;
	}
	.scan-line {
		animation: scan-line 6s linear infinite;
	}
	@keyframes float-up {
		0% { transform: translateY(0); opacity: 0.5; }
		100% { transform: translateY(-100vh); opacity: 0; }
	}
	.particle {
		animation: float-up var(--dur) linear var(--delay) infinite;
	}
</style>

<!-- NAV -->
<nav class="fixed top-0 left-0 right-0 z-50 transition-all duration-300 {scrollY > 60 ? 'bg-bg/80 backdrop-blur-lg border-b border-border/50' : ''}">
	<div class="max-w-6xl mx-auto px-6 md:px-10 flex items-center justify-between h-14">
		<a href="/" class="font-mono text-sm uppercase tracking-tight text-text/80 hover:text-text transition-colors">
			traceway
		</a>
		<div class="flex items-center gap-6 md:gap-8">
			<a href="#features" class="text-[13px] text-text-secondary hover:text-text transition-colors hidden md:inline">Features</a>
			<a href="#how-it-works" class="text-[13px] text-text-secondary hover:text-text transition-colors hidden md:inline">How it works</a>
			<a href="#pricing" class="text-[13px] text-text-secondary hover:text-text transition-colors hidden md:inline">Pricing</a>
			<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-[13px] text-text-secondary hover:text-text transition-colors hidden sm:inline">GitHub</a>
			<a href="https://platform.traceway.ai/login" class="text-[13px] text-text-secondary hover:text-text transition-colors">Log in</a>
			<a
				href="https://platform.traceway.ai/signup"
				class="text-[13px] font-medium text-bg bg-text rounded-md px-4 py-1.5 hover:bg-text/90 transition-all"
			>
				Get started
			</a>
		</div>
	</div>
</nav>

<!-- HERO — full-bleed with live trace visualization -->
<section class="relative min-h-screen flex items-center justify-center overflow-hidden">
	<!-- Animated trace waterfall background -->
	<div class="absolute inset-0" style="transform: translateY({scrollY * -0.15}px);">
		<!-- Vertical connection lines -->
		<div class="absolute left-[8%] top-[14%] w-px h-[76%] bg-gradient-to-b from-transparent via-accent/[0.06] to-transparent"></div>
		<div class="absolute left-[35%] top-[10%] w-px h-[82%] bg-gradient-to-b from-transparent via-purple-400/[0.04] to-transparent"></div>
		<div class="absolute left-[53%] top-[16%] w-px h-[70%] bg-gradient-to-b from-transparent via-accent/[0.05] to-transparent"></div>

		<!-- Animated span bars -->
		{#each heroSpans as span}
			<div
				class="hero-span absolute rounded-sm"
				style="
					top: {span.y}%;
					left: {span.x}%;
					width: {span.w}%;
					height: {span.h}%;
					background: {span.color};
					--delay: {span.delay}s;
					--dur: {span.dur}s;
				"
			></div>
		{/each}

		<!-- Scanning line -->
		<div class="scan-line absolute top-0 w-px h-full bg-gradient-to-b from-transparent via-accent/30 to-transparent" style="filter: blur(1px);"></div>

		<!-- Floating particles -->
		{#each Array(8) as _, i}
			<div
				class="particle absolute w-px rounded-full"
				style="
					left: {10 + i * 12}%;
					bottom: 0;
					height: 3px;
					background: rgba(110,231,183,{0.15 + (i % 3) * 0.1});
					--delay: {i * 1.2}s;
					--dur: {6 + (i % 4) * 2}s;
				"
			></div>
		{/each}
	</div>

	<!-- Glow orbs -->
	<div class="absolute top-1/4 left-1/4 w-[500px] h-[500px] bg-accent/[0.04] blur-[180px] rounded-full"></div>
	<div class="absolute bottom-1/4 right-1/4 w-[400px] h-[400px] bg-purple-400/[0.03] blur-[150px] rounded-full"></div>

	<!-- Vignette / fade to center -->
	<div class="absolute inset-0 bg-[radial-gradient(ellipse_at_center,transparent_30%,var(--color-bg)_75%)]"></div>

	<!-- Content -->
	<div class="relative z-10 max-w-6xl mx-auto px-6 md:px-10 text-center py-32">
		<InView>
			<div class="inline-flex items-center gap-2.5 border border-accent/20 bg-accent/[0.04] rounded-full px-5 py-2 mb-10 backdrop-blur-sm">
				<span class="h-1.5 w-1.5 rounded-full bg-accent animate-pulse"></span>
				<span class="text-[12px] text-accent/80 font-medium tracking-wide">Open source LLM observability</span>
			</div>

			<h1 class="text-[clamp(3rem,7vw,5.5rem)] font-bold leading-[0.95] tracking-tight text-text max-w-4xl mx-auto">
				See inside every
				<span class="relative inline-block">
					<span class="relative z-10 text-transparent bg-clip-text bg-gradient-to-r from-accent via-emerald-300 to-accent">AI call</span>
					<span class="absolute -inset-x-2 -inset-y-1 bg-accent/[0.08] rounded-lg blur-sm"></span>
				</span>
			</h1>

			<p class="max-w-2xl mx-auto text-[17px] md:text-[19px] leading-relaxed text-text-secondary mt-8">
				Every prompt. Every token. Every dollar.
				<span class="text-text/70">Debug AI in minutes, not days.</span>
			</p>

			<div class="flex items-center justify-center gap-4 mt-12">
				<a
					href="https://platform.traceway.ai/signup"
					class="group relative inline-flex items-center gap-2 bg-accent text-bg font-semibold text-sm px-8 py-3.5 rounded-lg transition-all hover:shadow-[0_0_50px_rgba(110,231,183,0.25)]"
				>
					<span class="absolute inset-0 rounded-lg bg-gradient-to-r from-white/0 via-white/10 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity"></span>
					<span class="relative">Get started free</span>
				</a>
				<a
					href="https://github.com/blastgits/traceway"
					target="_blank"
					rel="noopener"
					class="inline-flex items-center gap-2 text-sm text-text-secondary border border-border/60 bg-bg/50 backdrop-blur-sm rounded-lg px-8 py-3.5 hover:bg-bg-secondary hover:border-border hover:text-text transition-all"
				>
					View on GitHub
				</a>
			</div>


		</InView>
	</div>

	<!-- Bottom fade -->
	<div class="absolute bottom-0 left-0 right-0 h-32 bg-gradient-to-t from-bg to-transparent"></div>
</section>

<!-- SOCIAL PROOF -->
<section class="py-12 border-y border-border/40">
	<div class="max-w-6xl mx-auto px-6 md:px-10 text-center">
		<p class="text-[12px] uppercase tracking-widest text-text-muted mb-6">Works with every provider</p>
		<div class="flex items-center justify-center gap-10 md:gap-16 flex-wrap">
			{#each ['OpenAI', 'Anthropic', 'Ollama', 'Mistral', 'Any OpenAI-compatible API'] as provider}
				<span class="font-mono text-[13px] text-text-muted/50">{provider}</span>
			{/each}
		</div>
	</div>
</section>

<!-- VALUE PROP — the "why" with visual punch -->
<section class="py-24 md:py-32 relative overflow-hidden">
	<div class="absolute inset-0 bg-gradient-to-b from-transparent via-accent/[0.015] to-transparent"></div>
	<div class="relative max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-16 md:mb-20">
				<h2 class="text-3xl md:text-[2.75rem] font-semibold tracking-tight text-text leading-tight">
					Your AI app returned a wrong answer.<br class="hidden md:inline" />
					<span class="text-text-secondary">Now what?</span>
				</h2>
				<p class="text-text-secondary mt-5 max-w-xl mx-auto leading-relaxed">
					Without observability, debugging AI is guesswork. With Traceway, you see every step the model took and find the root cause in minutes.
				</p>
			</div>
		</InView>

		<div class="grid md:grid-cols-3 gap-6">
			{#each [
				{
					title: 'See every step',
					desc: 'Full trace of every LLM call, retrieval, and tool use. Inputs, outputs, latency, and cost at a glance.',
					num: '01',
					accent: true
				},
				{
					title: 'Find the problem',
					desc: 'Wrong documents retrieved? Bad prompt? Model hallucination? See exactly which step in the chain broke.',
					num: '02',
					accent: false
				},
				{
					title: 'Prove the fix',
					desc: 'Verify fixes with new traces. Search across all requests to find every affected user.',
					num: '03',
					accent: false
				}
			] as item, i}
				<InView delay={i * 80}>
					<div class="bg-bg-secondary/60 border border-border/50 rounded-xl p-7 hover:border-border transition-all group h-full">
						<div class="flex items-center gap-3 mb-4">
							<span class="h-8 w-8 rounded-lg bg-accent/10 border border-accent/20 flex items-center justify-center text-accent font-mono text-[11px] font-medium">{item.num}</span>
						</div>
						<h3 class="text-[16px] font-medium text-text mb-2">{item.title}</h3>
						<p class="text-sm text-text-secondary leading-relaxed">{item.desc}</p>
					</div>
				</InView>
			{/each}
		</div>
	</div>
</section>

<!-- FEATURES -->
<section id="features" class="py-24 md:py-32 border-t border-border/40">
	<div class="max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-16 md:mb-20">
				<p class="text-[12px] uppercase tracking-widest text-accent/70 mb-4">Features</p>
				<h2 class="text-3xl md:text-4xl font-semibold tracking-tight text-text">
					Everything you need to debug LLM applications
				</h2>
				<p class="text-text-secondary mt-4 max-w-lg mx-auto leading-relaxed">
					From local development to production monitoring. One tool for the entire lifecycle.
				</p>
			</div>
		</InView>

		<div class="grid sm:grid-cols-2 lg:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
			{#each [
				{
					title: 'Full trace capture',
					desc: 'Every LLM call logged with inputs, outputs, latency, tokens, and cost. Nested spans for complex chains.',
				},
				{
					title: 'Real-time dashboard',
					desc: 'Live analytics with interactive charts. Filter by model, time range, status. See trends at a glance.',
				},
				{
					title: 'Local-first',
					desc: 'Runs as a lightweight daemon on your machine. No cloud account needed. Your data stays on disk.',
				},
				{
					title: 'Cloud ready',
					desc: 'When you need scale, deploy to the cloud with Postgres + Turbopuffer. Same API, same dashboard.',
				},
				{
					title: 'Any LLM provider',
					desc: 'Transparent proxy works with OpenAI, Anthropic, Ollama, or any OpenAI-compatible API.',
				},
				{
					title: 'Open source',
					desc: 'MIT licensed. Self-host, fork, contribute. Built in Rust for speed and reliability.',
				}
			] as card, i}
				<InView delay={i * 50}>
					<div class="bg-bg p-8 hover:bg-bg-secondary transition-colors h-full">
						<h3 class="text-[15px] font-medium text-text mb-3">{card.title}</h3>
						<p class="text-sm text-text-secondary leading-relaxed">{card.desc}</p>
					</div>
				</InView>
			{/each}
		</div>
	</div>
</section>

<!-- HOW IT WORKS — code + architecture -->
<section id="how-it-works" class="py-24 md:py-32 border-t border-border/40">
	<div class="max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-16 md:mb-20">
				<p class="text-[12px] uppercase tracking-widest text-accent/70 mb-4">How it works</p>
				<h2 class="text-3xl md:text-4xl font-semibold tracking-tight text-text">
					One line to start tracing
				</h2>
				<p class="text-text-secondary mt-4 max-w-lg mx-auto leading-relaxed">
					Point your LLM client at the Traceway proxy. Every call is captured automatically.
				</p>
			</div>
		</InView>

		<InView delay={80}>
			<div class="max-w-2xl mx-auto">
				<div class="rounded-lg border border-border bg-bg-secondary overflow-hidden">
					<div class="flex items-center justify-between border-b border-border/60 px-4 py-2.5">
						<span class="text-[11px] text-text-muted font-mono">app.py</span>
						<span class="text-[11px] text-text-muted font-mono">python</span>
					</div>
					{@html `<div class="p-5 font-mono text-[12px] leading-relaxed overflow-x-auto">
<div><span class="text-text-muted">import</span> <span class="text-text">openai</span></div>
<div class="mt-3"></div>
<div class="text-text-muted"># Point at the Traceway proxy — that's it</div>
<div><span class="text-text">client</span> <span class="text-text-muted">=</span> <span class="text-text">openai.OpenAI(</span></div>
<div class="text-accent">    base_url=<span class="text-accent-dim">"http://localhost:3001/v1"</span></div>
<div><span class="text-text">)</span></div>
<div class="mt-3"></div>
<div class="text-text-muted"># Every call is traced automatically</div>
<div><span class="text-text">response</span> <span class="text-text-muted">=</span> <span class="text-text">client.chat.completions.create(</span></div>
<div>    <span class="text-text">model=</span><span class="text-accent-dim">"gpt-4o"</span><span class="text-text-muted">,</span></div>
<div>    <span class="text-text">messages=[{</span><span class="text-accent-dim">"role"</span><span class="text-text">: </span><span class="text-accent-dim">"user"</span><span class="text-text">, </span><span class="text-accent-dim">"content"</span><span class="text-text">: </span><span class="text-accent-dim">"Hello!"</span><span class="text-text">}]</span></div>
<div><span class="text-text">)</span></div>
</div>`}
				</div>
			</div>
		</InView>

		<!-- Architecture -->
		<InView delay={150}>
			<div class="mt-16 max-w-3xl mx-auto">
				<div class="rounded-lg border border-border bg-bg-secondary overflow-hidden">
					<div class="border-b border-border/60 px-4 py-2.5">
						<span class="text-[11px] text-text-muted font-mono">Architecture</span>
					</div>
					<div class="p-6 overflow-x-auto">
						<ArchDiagram />
					</div>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- PRICING -->
<section id="pricing" class="py-24 md:py-32 border-t border-border/40 relative overflow-hidden">
	<div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[1000px] h-[600px] bg-accent/[0.02] blur-[200px] rounded-full"></div>
	<div class="relative max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-16 md:mb-20">
				<p class="text-[12px] uppercase tracking-widest text-accent/70 mb-4">Pricing</p>
				<h2 class="text-3xl md:text-4xl font-semibold tracking-tight text-text">
					Start free. Scale when you're ready.
				</h2>
				<p class="text-text-secondary mt-4 max-w-lg mx-auto leading-relaxed">
					Self-host for free forever, or let us handle the infrastructure.
				</p>
			</div>
		</InView>

		<div class="grid md:grid-cols-2 lg:grid-cols-4 gap-5 max-w-5xl mx-auto">
			<!-- Free -->
			<InView delay={0}>
				<div class="bg-bg-secondary border border-border/60 rounded-xl p-6 flex flex-col h-full">
					<div class="mb-6">
						<div class="text-[12px] text-text-muted uppercase tracking-wider mb-3">Free</div>
						<div class="flex items-baseline gap-1">
							<span class="text-3xl font-semibold text-text">$0</span>
							<span class="text-sm text-text-muted">/mo</span>
						</div>
						<p class="text-[13px] text-text-secondary mt-2">For getting started and local development.</p>
					</div>
					<ul class="space-y-2.5 text-[13px] flex-1 mb-6">
						{#each [
							'10K spans/month',
							'7-day retention',
							'1 team member',
							'1 API key',
							'Community support'
						] as item}
							<li class="flex items-center gap-2.5 text-text-secondary">
								<svg class="w-3.5 h-3.5 shrink-0 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
								{item}
							</li>
						{/each}
					</ul>
					<a
						href="https://platform.traceway.ai/signup"
						class="block text-center text-sm text-text-secondary border border-border rounded-lg px-4 py-2.5 hover:bg-bg-tertiary hover:text-text transition-all"
					>
						Get started
					</a>
				</div>
			</InView>

			<!-- Pro — highlighted -->
			<InView delay={60}>
				<div class="relative bg-bg-secondary border-2 border-accent/40 rounded-xl p-6 flex flex-col h-full shadow-[0_0_60px_rgba(110,231,183,0.06)]">
					<div class="absolute -top-3 left-1/2 -translate-x-1/2">
						<span class="bg-accent text-bg text-[10px] font-semibold uppercase tracking-wider px-3 py-1 rounded-full">Most popular</span>
					</div>
					<div class="mb-6">
						<div class="text-[12px] text-accent uppercase tracking-wider mb-3">Pro</div>
						<div class="flex items-baseline gap-1">
							<span class="text-3xl font-semibold text-text">$20</span>
							<span class="text-sm text-text-muted">/mo</span>
						</div>
						<p class="text-[13px] text-text-secondary mt-2">For teams shipping AI to production.</p>
					</div>
					<ul class="space-y-2.5 text-[13px] flex-1 mb-6">
						{#each [
							'1M spans/month',
							'30-day retention',
							'5 team members',
							'5 API keys',
							'Email support'
						] as item}
							<li class="flex items-center gap-2.5 text-text-secondary">
								<svg class="w-3.5 h-3.5 shrink-0 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
								{item}
							</li>
						{/each}
					</ul>
					<a
						href="https://platform.traceway.ai/signup"
						class="block text-center text-sm font-medium bg-accent text-bg rounded-lg px-4 py-2.5 hover:brightness-110 transition-all"
					>
						Start free trial
					</a>
				</div>
			</InView>

			<!-- Team -->
			<InView delay={120}>
				<div class="bg-bg-secondary border border-border/60 rounded-xl p-6 flex flex-col h-full">
					<div class="mb-6">
						<div class="text-[12px] text-text-muted uppercase tracking-wider mb-3">Team</div>
						<div class="flex items-baseline gap-1">
							<span class="text-3xl font-semibold text-text">$100</span>
							<span class="text-sm text-text-muted">/mo</span>
						</div>
						<p class="text-[13px] text-text-secondary mt-2">For growing teams that need full observability.</p>
					</div>
					<ul class="space-y-2.5 text-[13px] flex-1 mb-6">
						{#each [
							'10M spans/month',
							'90-day retention',
							'50 team members',
							'Unlimited API keys',
							'Priority support'
						] as item}
							<li class="flex items-center gap-2.5 text-text-secondary">
								<svg class="w-3.5 h-3.5 shrink-0 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
								{item}
							</li>
						{/each}
					</ul>
					<a
						href="https://platform.traceway.ai/signup"
						class="block text-center text-sm text-text-secondary border border-border rounded-lg px-4 py-2.5 hover:bg-bg-tertiary hover:text-text transition-all"
					>
						Get started
					</a>
				</div>
			</InView>

			<!-- Enterprise -->
			<InView delay={180}>
				<div class="bg-bg-secondary border border-border/60 rounded-xl p-6 flex flex-col h-full">
					<div class="mb-6">
						<div class="text-[12px] text-text-muted uppercase tracking-wider mb-3">Enterprise</div>
						<div class="flex items-baseline gap-1">
							<span class="text-2xl font-semibold text-text">Custom</span>
						</div>
						<p class="text-[13px] text-text-secondary mt-2">For organizations with advanced needs.</p>
					</div>
					<ul class="space-y-2.5 text-[13px] flex-1 mb-6">
						{#each [
							'Unlimited spans',
							'365-day retention',
							'Unlimited members',
							'SSO & SAML',
							'Dedicated support'
						] as item}
							<li class="flex items-center gap-2.5 text-text-secondary">
								<svg class="w-3.5 h-3.5 shrink-0 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
								{item}
							</li>
						{/each}
					</ul>
					<a
						href="mailto:support@traceway.ai"
						class="block text-center text-sm text-text-secondary border border-border rounded-lg px-4 py-2.5 hover:bg-bg-tertiary hover:text-text transition-all"
					>
						Contact us
					</a>
				</div>
			</InView>
		</div>

		<!-- Self-hosted callout -->
		<InView delay={200}>
			<div class="mt-10 max-w-5xl mx-auto">
				<div class="bg-bg-secondary/50 border border-border/40 rounded-xl px-6 py-5 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
					<div>
						<div class="text-sm font-medium text-text">Self-hosted is always free</div>
						<p class="text-[13px] text-text-secondary mt-1">Run on your own infrastructure with unlimited traces. No account required.</p>
					</div>
					<a
						href="https://github.com/blastgits/traceway"
						target="_blank"
						rel="noopener"
						class="shrink-0 inline-flex items-center gap-2 text-sm text-text-secondary border border-border rounded-lg px-4 py-2 hover:bg-bg-tertiary hover:text-text transition-all"
					>
						View on GitHub
					</a>
				</div>
			</div>
		</InView>

		<p class="text-center text-[12px] text-text-muted mt-8">
			All plans billed monthly via <a href="https://polar.sh" target="_blank" rel="noopener" class="text-accent/70 hover:text-accent transition-colors">Polar</a>. Cancel anytime.
		</p>
	</div>
</section>

<!-- BOTTOM CTA -->
<section class="py-28 md:py-36 border-t border-border/40 relative overflow-hidden">
	<div class="absolute inset-0 bg-gradient-to-t from-accent/[0.03] via-transparent to-transparent"></div>
	<div class="absolute bottom-0 left-1/2 -translate-x-1/2 w-[800px] h-[400px] bg-accent/[0.04] blur-[200px] rounded-full"></div>
	<div class="relative max-w-6xl mx-auto px-6 md:px-10 text-center">
		<InView>
			<h2 class="text-3xl md:text-[2.75rem] font-semibold tracking-tight text-text leading-tight">
				Start tracing in under a minute
			</h2>
			<p class="mt-5 text-text-secondary max-w-md mx-auto leading-relaxed">
				One binary. No config. Free forever on your own infrastructure.
			</p>
			<div class="flex items-center justify-center gap-4 mt-10">
				<a
					href="https://platform.traceway.ai/signup"
					class="inline-flex items-center gap-2 bg-accent text-bg font-medium text-sm px-7 py-3 rounded-lg transition-all hover:brightness-110 hover:shadow-[0_0_60px_rgba(110,231,183,0.2)]"
				>
					Get started free
				</a>
				<a
					href="https://github.com/blastgits/traceway"
					target="_blank"
					rel="noopener"
					class="inline-flex items-center gap-2 text-sm text-text-secondary border border-border rounded-lg px-7 py-3 hover:bg-bg-secondary hover:text-text transition-all"
				>
					Star on GitHub
				</a>
			</div>
		</InView>
	</div>
</section>

<!-- FOOTER -->
<footer class="border-t border-border/40 py-8">
	<div class="max-w-6xl mx-auto px-6 md:px-10">
		<div class="flex flex-col sm:flex-row items-center justify-between gap-4">
			<span class="font-mono text-[11px] uppercase text-text-muted tracking-tight">traceway</span>
			<div class="flex items-center gap-6">
				<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-[12px] text-text-muted hover:text-text-secondary transition-colors">GitHub</a>
				<a href="https://platform.traceway.ai/login" class="text-[12px] text-text-muted hover:text-text-secondary transition-colors">Log in</a>
				<a href="https://platform.traceway.ai/signup" class="text-[12px] text-text-muted hover:text-text-secondary transition-colors">Sign up</a>
			</div>
			<span class="text-[11px] text-text-muted/50">MIT Licensed</span>
		</div>
	</div>
</footer>
