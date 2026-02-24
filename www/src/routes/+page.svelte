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

<!-- BEFORE / AFTER — the "aha" moment -->
<section id="features" class="py-28 md:py-36 relative">
	<div class="max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-20">
				<h2 class="text-3xl md:text-[2.75rem] font-semibold tracking-tight text-text leading-tight">
					Your AI returned a wrong answer.<br class="hidden md:inline" />
					<span class="text-text-secondary">Now what?</span>
				</h2>
			</div>
		</InView>

		<!-- Side by side: Without vs With -->
		<div class="grid md:grid-cols-2 gap-6 md:gap-0">
			<!-- WITHOUT -->
			<InView>
				<div class="relative md:border-r border-border/30 md:pr-10">
					<div class="text-[11px] uppercase tracking-widest text-red-400/70 font-mono mb-6">Without Traceway</div>
					<div class="space-y-4">
						{#each [
							{ time: '14:32', line: 'POST /api/chat — 200 OK — 2.3s', dim: false },
							{ time: '14:32', line: 'user_id=usr_8f3 session=sess_29x', dim: true },
							{ time: '', line: '', dim: true },
							{ time: '???', line: 'Can\'t reproduce. Works fine for me.', dim: false },
							{ time: '???', line: 'Adding print() statements...', dim: true },
							{ time: '???', line: 'Redeploying to staging...', dim: true },
							{ time: '???', line: 'Waiting for it to happen again...', dim: true },
						] as log}
							{#if log.line}
								<div class="flex items-start gap-3 font-mono text-[12px] {log.dim ? 'text-text-muted/40' : 'text-text-muted/70'}">
									<span class="shrink-0 w-10 text-right text-text-muted/30">{log.time}</span>
									<span>{log.line}</span>
								</div>
							{:else}
								<div class="h-4"></div>
							{/if}
						{/each}
						<div class="mt-6 pt-6 border-t border-border/20 font-mono text-[13px] text-red-400/50">
							3 hours later... found it. Maybe.
						</div>
					</div>
				</div>
			</InView>

			<!-- WITH -->
			<InView delay={200}>
				<div class="md:pl-10">
					<div class="text-[11px] uppercase tracking-widest text-accent/70 font-mono mb-6">With Traceway</div>
					<!-- Mini trace visualization -->
					<div class="rounded-lg border border-accent/20 bg-accent/[0.02] overflow-hidden">
						<div class="px-4 py-2.5 border-b border-accent/10 flex items-center justify-between">
							<span class="font-mono text-[12px] text-accent/80">customer-support-bot</span>
							<span class="font-mono text-[10px] text-text-muted">2.3s &middot; $0.043</span>
						</div>
						<div class="p-4 space-y-2">
							{#each [
								{ name: 'rewrite-query', w: '15%', color: 'bg-purple-400/30', label: '180ms' },
								{ name: 'search-kb', w: '8%', color: 'bg-cyan-400/30', label: '92ms', bad: true },
								{ name: 'build-context', w: '4%', color: 'bg-text-muted/20', label: '45ms' },
								{ name: 'generate-answer', w: '65%', color: 'bg-accent/25', label: '1.8s' },
							] as s}
								<div class="flex items-center gap-3">
									<span class="font-mono text-[10px] text-text-muted w-28 shrink-0 text-right">{s.name}</span>
									<div class="flex-1 h-5 relative">
										<div class="h-full rounded-sm {s.color} {s.bad ? 'ring-1 ring-red-400/40' : ''}" style="width: {s.w};">
											{#if s.bad}
												<span class="absolute -top-5 left-0 text-[9px] font-mono text-red-400/70">wrong doc returned</span>
											{/if}
										</div>
									</div>
									<span class="font-mono text-[10px] text-text-muted/50 w-12 shrink-0">{s.label}</span>
								</div>
							{/each}
						</div>
					</div>
					<div class="mt-6 pt-6 border-t border-accent/10 font-mono text-[13px] text-accent/60">
						5 minutes. Root cause found. 23 affected users identified.
					</div>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- HOW IT WORKS — interactive terminal-style -->
<section id="how-it-works" class="py-28 md:py-36 relative overflow-hidden">
	<!-- Full-width accent line -->
	<div class="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-accent/20 to-transparent"></div>

	<div class="max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-20">
				<h2 class="text-3xl md:text-[2.75rem] font-semibold tracking-tight text-text leading-tight">
					Three commands. Full observability.
				</h2>
			</div>
		</InView>

		<!-- Steps as connected terminals -->
		<div class="max-w-3xl mx-auto space-y-0">
			{#each [
				{
					step: '01',
					title: 'Install',
					cmd: '$ cargo install --git https://github.com/blastgits/traceway traceway',
					output: ['Compiling traceway v0.1.0', 'Installed to ~/.cargo/bin/traceway'],
					accent: false
				},
				{
					step: '02',
					title: 'Run',
					cmd: '$ traceway --target-url https://api.openai.com',
					output: ['API server on 127.0.0.1:3000', 'Proxy on 127.0.0.1:3001 -> api.openai.com'],
					accent: true
				},
				{
					step: '03',
					title: 'Trace',
					cmd: '$ python app.py  # point base_url at :3001',
					output: ['3 traces captured (247ms avg, $0.0012 total)', 'Dashboard: http://localhost:3000'],
					accent: true
				},
			] as step, i}
				<InView delay={i * 120}>
					<div class="relative">
						<!-- Connector line -->
						{#if i > 0}
							<div class="absolute -top-6 left-8 w-px h-6 bg-border/40"></div>
						{/if}
						<div class="flex gap-6 items-start">
							<!-- Step indicator -->
							<div class="shrink-0 w-16 pt-4">
								<div class="text-accent font-mono text-[11px] font-semibold">{step.step}</div>
								<div class="text-[13px] text-text-secondary mt-0.5">{step.title}</div>
							</div>
							<!-- Terminal block -->
							<div class="flex-1 rounded-lg border border-border/60 bg-bg-secondary/80 overflow-hidden mb-6">
								<div class="px-4 py-2 border-b border-border/30 flex items-center gap-1.5">
									<div class="h-2 w-2 rounded-full bg-text-muted/15"></div>
									<div class="h-2 w-2 rounded-full bg-text-muted/15"></div>
									<div class="h-2 w-2 rounded-full bg-text-muted/15"></div>
								</div>
								<div class="p-4 font-mono text-[12px] leading-relaxed">
									<div class="text-text/80">{step.cmd}</div>
									{#each step.output as line}
										<div class="text-{step.accent ? 'accent' : 'text-muted'}/60 mt-0.5">{step.accent ? '✓ ' : '  '}{line}</div>
									{/each}
								</div>
							</div>
						</div>
					</div>
				</InView>
			{/each}
		</div>
	</div>
</section>

<!-- FEATURES — bento grid -->
<section class="py-28 md:py-36 relative overflow-hidden">
	<div class="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-border/40 to-transparent"></div>

	<div class="max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-16">
				<h2 class="text-3xl md:text-[2.75rem] font-semibold tracking-tight text-text">
					Built for real AI workloads
				</h2>
			</div>
		</InView>

		<!-- Bento grid — asymmetric, not boring -->
		<div class="grid grid-cols-1 md:grid-cols-6 gap-4">
			<!-- Big card: trace capture (spans 4 cols) -->
			<InView class="md:col-span-4">
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl p-8 relative overflow-hidden group hover:border-border/70 transition-all h-full">
					<div class="absolute top-0 right-0 w-64 h-64 bg-accent/[0.03] blur-[100px] rounded-full group-hover:bg-accent/[0.05] transition-all"></div>
					<div class="relative">
						<div class="text-accent font-mono text-[11px] mb-3">Full trace capture</div>
						<h3 class="text-xl font-semibold text-text mb-2">Every call. Every span. Every token.</h3>
						<p class="text-sm text-text-secondary max-w-md leading-relaxed">Nested spans for complex chains — RAG pipelines, agent loops, tool calls. See inputs, outputs, latency, and cost for every step.</p>
						<!-- Mini waterfall inside -->
						<div class="mt-6 space-y-1.5 max-w-sm">
							{#each [
								{ name: 'agent-loop', w: '100%', c: 'bg-accent/15' },
								{ name: '  llm-call', w: '40%', c: 'bg-accent/20' },
								{ name: '  tool:search', w: '25%', c: 'bg-purple-400/20', ml: '42%' },
								{ name: '  llm-call', w: '30%', c: 'bg-accent/20', ml: '68%' },
							] as bar}
								<div class="flex items-center gap-2">
									<span class="font-mono text-[9px] text-text-muted/40 w-20 shrink-0 text-right">{bar.name}</span>
									<div class="flex-1 h-3 relative">
										<div class="absolute top-0 h-full rounded-sm {bar.c}" style="width: {bar.w}; margin-left: {bar.ml || '0'}"></div>
									</div>
								</div>
							{/each}
						</div>
					</div>
				</div>
			</InView>

			<!-- Side cards (spans 2 cols, stacked) -->
			<InView delay={80} class="md:col-span-2 flex flex-col gap-4">
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl p-6 hover:border-border/70 transition-all flex-1">
					<div class="text-purple-400 font-mono text-[11px] mb-3">Any provider</div>
					<h3 class="text-[15px] font-medium text-text mb-2">OpenAI, Anthropic, Ollama</h3>
					<p class="text-[13px] text-text-secondary leading-relaxed">Transparent proxy. Zero code changes. Auto-detects provider.</p>
				</div>
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl p-6 hover:border-border/70 transition-all flex-1">
					<div class="text-cyan-400 font-mono text-[11px] mb-3">Built in Rust</div>
					<h3 class="text-[15px] font-medium text-text mb-2">Fast. Really fast.</h3>
					<p class="text-[13px] text-text-secondary leading-relaxed">Sub-millisecond overhead. Single binary, no containers, no dependencies.</p>
				</div>
			</InView>

			<!-- Bottom row: 3 equal cards -->
			<InView delay={100} class="md:col-span-2">
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl p-6 hover:border-border/70 transition-all h-full">
					<div class="text-amber-400 font-mono text-[11px] mb-3">Real-time dashboard</div>
					<h3 class="text-[15px] font-medium text-text mb-2">Live analytics</h3>
					<p class="text-[13px] text-text-secondary leading-relaxed">Charts, filters, trends. See what's happening right now.</p>
				</div>
			</InView>
			<InView delay={180} class="md:col-span-2">
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl p-6 hover:border-border/70 transition-all h-full">
					<div class="text-accent font-mono text-[11px] mb-3">Local-first</div>
					<h3 class="text-[15px] font-medium text-text mb-2">Your data stays yours</h3>
					<p class="text-[13px] text-text-secondary leading-relaxed">SQLite on your machine. No cloud account needed. Upgrade to cloud when ready.</p>
				</div>
			</InView>
			<InView delay={260} class="md:col-span-2">
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl p-6 hover:border-border/70 transition-all h-full">
					<div class="text-text-muted font-mono text-[11px] mb-3">Open source</div>
					<h3 class="text-[15px] font-medium text-text mb-2">MIT licensed</h3>
					<p class="text-[13px] text-text-secondary leading-relaxed">Self-host, fork, contribute. No vendor lock-in, ever.</p>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- PRICING -->
<section id="pricing" class="py-28 md:py-36 relative overflow-hidden">
	<div class="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-accent/20 to-transparent"></div>
	<div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[1000px] h-[600px] bg-accent/[0.02] blur-[200px] rounded-full"></div>

	<div class="relative max-w-6xl mx-auto px-6 md:px-10">
		<InView>
			<div class="text-center mb-20">
				<h2 class="text-3xl md:text-[2.75rem] font-semibold tracking-tight text-text">
					Start free. Scale when you're ready.
				</h2>
				<p class="text-text-secondary mt-4 max-w-lg mx-auto leading-relaxed">
					Self-host for free forever, or let us handle the infrastructure.
				</p>
			</div>
		</InView>

		<div class="grid md:grid-cols-3 gap-5 max-w-4xl mx-auto">
			{#each [
				{
					name: 'Free', price: '$0', period: '/mo', highlight: false,
					desc: 'For getting started and local development.',
					features: ['10K spans/month', '7-day retention', '1 team member', '1 API key', 'Community support'],
					cta: 'Get started', ctaStyle: 'border'
				},
				{
					name: 'Pro', price: '$20', period: '/mo', highlight: true,
					desc: 'For teams shipping AI to production.',
					features: ['1M spans/month', '30-day retention', '5 team members', '5 API keys', 'Email support'],
					cta: 'Start free trial', ctaStyle: 'accent'
				},
				{
					name: 'Team', price: '$100', period: '/mo', highlight: false,
					desc: 'For growing teams that need full observability.',
					features: ['10M spans/month', '90-day retention', '50 team members', 'Unlimited API keys', 'Priority support'],
					cta: 'Get started', ctaStyle: 'border'
				},
			] as plan, i}
				<InView delay={i * 80}>
					<div class="relative {plan.highlight ? 'bg-bg-secondary border-2 border-accent/40 shadow-[0_0_80px_rgba(110,231,183,0.06)]' : 'bg-bg-secondary/50 border border-border/40'} rounded-2xl p-7 flex flex-col h-full">
						{#if plan.highlight}
							<div class="absolute -top-3.5 left-1/2 -translate-x-1/2">
								<span class="bg-accent text-bg text-[10px] font-bold uppercase tracking-wider px-4 py-1.5 rounded-full">Most popular</span>
							</div>
						{/if}
						<div class="mb-6">
							<div class="text-[12px] {plan.highlight ? 'text-accent' : 'text-text-muted'} uppercase tracking-wider mb-3 font-mono">{plan.name}</div>
							<div class="flex items-baseline gap-1">
								<span class="text-4xl font-bold text-text">{plan.price}</span>
								<span class="text-sm text-text-muted">{plan.period}</span>
							</div>
							<p class="text-[13px] text-text-secondary mt-2">{plan.desc}</p>
						</div>
						<ul class="space-y-2.5 text-[13px] flex-1 mb-7">
							{#each plan.features as feature}
								<li class="flex items-center gap-2.5 text-text-secondary">
									<svg class="w-3.5 h-3.5 shrink-0 {plan.highlight ? 'text-accent' : 'text-text-muted/60'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
									{feature}
								</li>
							{/each}
						</ul>
						<a
							href="https://platform.traceway.ai/signup"
							class="block text-center text-sm rounded-lg px-4 py-3 transition-all {plan.ctaStyle === 'accent' ? 'font-semibold bg-accent text-bg hover:brightness-110' : 'text-text-secondary border border-border hover:bg-bg-tertiary hover:text-text'}"
						>
							{plan.cta}
						</a>
					</div>
				</InView>
			{/each}
		</div>

		<!-- Enterprise + Self-hosted -->
		<div class="grid md:grid-cols-2 gap-5 max-w-4xl mx-auto mt-5">
			<InView delay={100}>
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl px-7 py-6 flex items-center justify-between gap-4">
					<div>
						<div class="text-sm font-medium text-text">Enterprise</div>
						<p class="text-[13px] text-text-secondary mt-0.5">Unlimited everything. SSO, SAML, dedicated support.</p>
					</div>
					<a href="mailto:support@traceway.ai" class="shrink-0 text-sm text-text-secondary border border-border rounded-lg px-5 py-2 hover:bg-bg-tertiary hover:text-text transition-all">Contact us</a>
				</div>
			</InView>
			<InView delay={150}>
				<div class="bg-bg-secondary/50 border border-border/40 rounded-2xl px-7 py-6 flex items-center justify-between gap-4">
					<div>
						<div class="text-sm font-medium text-text">Self-hosted</div>
						<p class="text-[13px] text-text-secondary mt-0.5">Free forever. Unlimited traces. No account needed.</p>
					</div>
					<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="shrink-0 text-sm text-text-secondary border border-border rounded-lg px-5 py-2 hover:bg-bg-tertiary hover:text-text transition-all">GitHub</a>
				</div>
			</InView>
		</div>

		<p class="text-center text-[12px] text-text-muted mt-8">
			Billed monthly via <a href="https://polar.sh" target="_blank" rel="noopener" class="text-accent/70 hover:text-accent transition-colors">Polar</a>. Cancel anytime.
		</p>
	</div>
</section>

<!-- BOTTOM CTA — big, dramatic -->
<section class="py-32 md:py-44 relative overflow-hidden">
	<div class="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-accent/20 to-transparent"></div>
	<!-- Full background glow -->
	<div class="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(110,231,183,0.04)_0%,transparent_60%)]"></div>
	<div class="relative max-w-6xl mx-auto px-6 md:px-10 text-center">
		<InView>
			<h2 class="text-4xl md:text-[3.5rem] font-bold tracking-tight text-text leading-[1]">
				Stop flying blind.
			</h2>
			<p class="mt-6 text-lg text-text-secondary max-w-lg mx-auto leading-relaxed">
				One binary. No config. See your first traces in under a minute.
			</p>
			<div class="flex items-center justify-center gap-4 mt-12">
				<a
					href="https://platform.traceway.ai/signup"
					class="group relative inline-flex items-center gap-2 bg-accent text-bg font-semibold text-[15px] px-10 py-4 rounded-xl transition-all hover:shadow-[0_0_80px_rgba(110,231,183,0.25)]"
				>
					<span class="absolute inset-0 rounded-xl bg-gradient-to-r from-white/0 via-white/10 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity"></span>
					<span class="relative">Get started free</span>
				</a>
			</div>
			<div class="mt-6">
				<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-sm text-text-muted hover:text-text-secondary transition-colors">
					or star us on GitHub
				</a>
			</div>
		</InView>
	</div>
</section>

<!-- FOOTER -->
<footer class="border-t border-border/30 py-8">
	<div class="max-w-6xl mx-auto px-6 md:px-10">
		<div class="flex flex-col sm:flex-row items-center justify-between gap-4">
			<span class="font-mono text-[11px] uppercase text-text-muted/60 tracking-tight">traceway</span>
			<div class="flex items-center gap-6">
				<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-[12px] text-text-muted/50 hover:text-text-secondary transition-colors">GitHub</a>
				<a href="https://platform.traceway.ai/login" class="text-[12px] text-text-muted/50 hover:text-text-secondary transition-colors">Log in</a>
				<a href="https://platform.traceway.ai/signup" class="text-[12px] text-text-muted/50 hover:text-text-secondary transition-colors">Sign up</a>
			</div>
			<span class="text-[11px] text-text-muted/30">MIT Licensed</span>
		</div>
	</div>
</footer>
