<script>
	import InView from '$lib/InView.svelte';
	import ArchDiagram from '$lib/ArchDiagram.svelte';

	let scrollY = $state(0);
	let navOpaque = $derived(scrollY > 60);
</script>

<svelte:window bind:scrollY={scrollY} />

<!-- NAV — full width, spread, monospace, Pierre-inspired but not copy -->
<nav
	class="fixed top-0 left-0 right-0 z-50 mix-blend-difference transition-all duration-500"
>
	<div class="flex items-baseline justify-between px-6 py-5 md:px-10">
		<div class="flex items-baseline gap-6">
			<a href="/" class="font-mono text-sm uppercase tracking-tight text-white/80 hover:text-white transition-colors">
				traceway
			</a>
			<span class="font-mono text-[11px] text-white/30 hidden sm:inline">2025</span>
		</div>
		<div class="flex items-baseline gap-6 md:gap-8">
			<a href="#features" class="font-mono text-[11px] uppercase text-white/50 hover:text-white transition-colors hidden md:inline">Features</a>
			<a href="#architecture" class="font-mono text-[11px] uppercase text-white/50 hover:text-white transition-colors hidden md:inline">Architecture</a>
			<a href="https://github.com/traceway" target="_blank" rel="noopener" class="font-mono text-[11px] uppercase text-white/50 hover:text-white transition-colors">GitHub</a>
			<a href="https://docs.traceway.dev" class="font-mono text-[11px] uppercase text-white/50 hover:text-white transition-colors hidden sm:inline">Docs</a>
			<a
				href="https://platform.traceway.dev"
				class="font-mono text-[11px] uppercase text-white/80 border border-white/20 rounded-full px-4 py-1 hover:bg-white/10 transition-all"
			>
				Open app
			</a>
		</div>
	</div>
</nav>

<!-- HERO — editorial, left-aligned, fills the screen -->
<section class="relative min-h-screen flex flex-col justify-center overflow-hidden">
	<!-- Background grid -->
	<div class="absolute inset-0 opacity-[0.025]" style="background-image: url(&quot;data:image/svg+xml,%3Csvg width='40' height='40' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M40 0H0v40' fill='none' stroke='%23fff' stroke-width='0.3'/%3E%3C/svg%3E&quot;);"></div>
	<!-- Glow -->
	<div class="absolute top-1/3 right-0 w-[600px] h-[600px] bg-accent/[0.03] blur-[150px] rounded-full"></div>

	<div class="relative px-6 md:px-10 max-w-[1400px]">
		<InView>
			<div class="flex flex-col gap-6 pt-32 md:pt-0">
				<!-- Eyebrow -->
				<div class="flex items-center gap-3">
					<span class="h-px w-8 bg-accent/40"></span>
					<span class="font-mono text-[11px] uppercase tracking-widest text-accent/70">Open source LLM observability</span>
				</div>

				<!-- Main headline — large, uppercase, monospace, not centered -->
				<h1 class="font-mono text-[clamp(2.5rem,8vw,7rem)] font-semibold leading-[0.95] tracking-tighter text-text uppercase">
					Trace every<br />
					<span class="text-accent">LLM call</span>
				</h1>

				<!-- Subtext — left aligned, narrow measure -->
				<p class="max-w-md text-[15px] leading-relaxed text-text-secondary mt-2">
					Capture inputs, outputs, latency, tokens, and cost automatically.
					Debug prompts in real time. Run on your laptop or deploy to the cloud.
				</p>

				<!-- CTAs — minimal -->
				<div class="flex items-center gap-5 mt-6">
					<a
						href="https://platform.traceway.dev"
						class="group flex items-center gap-2 bg-accent text-bg font-mono text-sm uppercase tracking-wide px-6 py-2.5 rounded transition-all hover:brightness-110 hover:shadow-[0_0_30px_rgba(110,231,183,0.2)]"
					>
						Get started
						<svg class="h-3.5 w-3.5 transition-transform group-hover:translate-x-1" viewBox="0 0 8 8" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M0.826 7.536L0.112 6.822L5.768 1.18H2.03V0.2H7.462V5.66H6.426V1.936L0.826 7.536Z" fill="currentColor"/></svg>
					</a>
					<a
						href="https://github.com/traceway"
						target="_blank"
						rel="noopener"
						class="font-mono text-[12px] uppercase text-text-secondary hover:text-text transition-colors border-b border-text-muted hover:border-text-secondary pb-0.5"
					>
						View source
					</a>
				</div>
			</div>
		</InView>

		<!-- Terminal — right-aligned on desktop, below on mobile -->
		<InView delay={200}>
			<div class="mt-16 md:mt-0 md:absolute md:right-10 md:bottom-[15vh] md:w-[480px]">
				<div class="rounded border border-border bg-bg-secondary/80 backdrop-blur overflow-hidden shadow-2xl shadow-black/30">
					<div class="flex items-center gap-2 border-b border-border/60 px-4 py-2.5">
						<div class="h-2 w-2 rounded-full bg-text-muted/30"></div>
						<div class="h-2 w-2 rounded-full bg-text-muted/30"></div>
						<div class="h-2 w-2 rounded-full bg-text-muted/30"></div>
						<span class="ml-3 text-[10px] text-text-muted font-mono uppercase">terminal</span>
					</div>
					<div class="p-4 font-mono text-[12px] leading-relaxed">
						<div class="text-text-muted">$ cargo install --path crates/daemon</div>
						<div class="mt-2 text-text-muted">$ traceway --foreground --target-url https://api.openai.com</div>
						<div class="mt-0.5 text-accent-dim">&#10003; API server listening on 127.0.0.1:3000</div>
						<div class="mt-0.5 text-accent-dim">&#10003; Proxy listening on 127.0.0.1:3001</div>
						<div class="mt-0.5 text-text-secondary">  target: https://api.openai.com</div>
						<div class="mt-2 text-text-muted">$ OPENAI_BASE_URL=http://localhost:3001 python app.py</div>
						<div class="mt-0.5 text-text-secondary">Tracing calls through proxy...</div>
						<div class="mt-0.5 text-accent-dim">&#10003; 3 traces captured (247ms avg, $0.0012 total)</div>
						<div class="mt-0.5 text-text-secondary">Dashboard: <span class="text-accent">http://localhost:3000</span></div>
					</div>
				</div>
			</div>
		</InView>
	</div>

	<!-- Bottom of hero — scroll hint -->
	<div class="absolute bottom-8 left-6 md:left-10 font-mono text-[10px] uppercase tracking-widest text-text-muted animate-pulse">
		scroll
	</div>
</section>

<!-- FEATURES — asymmetric grid, not cookie-cutter centered -->
<section id="features" class="py-28 relative">
	<div class="px-6 md:px-10 max-w-[1400px]">
		<InView>
			<div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4 mb-16">
				<div>
					<span class="font-mono text-[11px] uppercase tracking-widest text-accent/70">01 / Features</span>
					<h2 class="text-3xl font-semibold tracking-tight text-text mt-3 sm:text-4xl">
						Everything you need to<br class="hidden sm:block" /> debug LLM applications
					</h2>
				</div>
				<p class="text-sm text-text-secondary max-w-sm leading-relaxed">
					From local development to production monitoring.
					One tool for the entire lifecycle.
				</p>
			</div>
		</InView>

		<div class="grid gap-px bg-border sm:grid-cols-2 lg:grid-cols-3 border border-border">
			{#each [
				{
					title: 'Full trace capture',
					desc: 'Every LLM call logged automatically. Inputs, outputs, latency, tokens, cost. Nested spans for complex chains.',
					num: '01',
					delay: 0
				},
				{
					title: 'Real-time dashboard',
					desc: 'Live analytics with interactive charts. Filter by model, time range, status. See trends at a glance.',
					num: '02',
					delay: 60
				},
				{
					title: 'Local-first',
					desc: 'Runs as a lightweight daemon on your machine. No cloud account needed. Your data stays on disk.',
					num: '03',
					delay: 120
				},
				{
					title: 'Cloud ready',
					desc: 'When you need scale, deploy to the cloud with Postgres + Turbopuffer. Same API, same dashboard.',
					num: '04',
					delay: 0
				},
				{
					title: 'OpenAI compatible',
					desc: 'Drop-in proxy that intercepts API calls. No code changes required. Works with any OpenAI-compatible provider.',
					num: '05',
					delay: 60
				},
				{
					title: 'Open source',
					desc: 'MIT licensed. Self-host, fork, contribute. Built in Rust for speed and reliability.',
					num: '06',
					delay: 120
				}
			] as card}
				<InView delay={card.delay}>
					<div class="bg-bg p-6 md:p-8 transition-colors duration-300 hover:bg-bg-secondary group h-full">
						<div class="flex items-start justify-between mb-6">
							<span class="font-mono text-[10px] text-text-muted uppercase">{card.num}</span>
						</div>
						<h3 class="text-[15px] font-medium text-text mb-3">{card.title}</h3>
						<p class="text-sm text-text-secondary leading-relaxed">{card.desc}</p>
					</div>
				</InView>
			{/each}
		</div>
	</div>
</section>

<!-- ARCHITECTURE — clean diagram with straight lines -->
<section id="architecture" class="py-28 relative">
	<div class="px-6 md:px-10 max-w-[1400px]">
		<InView>
			<div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4 mb-16">
				<div>
					<span class="font-mono text-[11px] uppercase tracking-widest text-accent/70">02 / Architecture</span>
					<h2 class="text-3xl font-semibold tracking-tight text-text mt-3 sm:text-4xl">
						How Traceway works
					</h2>
				</div>
				<p class="text-sm text-text-secondary max-w-sm leading-relaxed">
					A lightweight proxy sits between your app and the LLM provider, capturing every request.
				</p>
			</div>
		</InView>

		<InView delay={100}>
			<div class="max-w-4xl">
				<div class="rounded border border-border bg-bg-secondary overflow-hidden">
					<div class="border-b border-border/60 px-4 py-2.5">
						<span class="text-[10px] text-text-muted font-mono uppercase">Architecture</span>
					</div>
					<div class="p-6 overflow-x-auto">
						<ArchDiagram />
					</div>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- LOCAL vs CLOUD -->
<section id="pricing" class="py-28 relative">
	<div class="px-6 md:px-10 max-w-[1400px]">
		<InView>
			<div class="mb-16">
				<span class="font-mono text-[11px] uppercase tracking-widest text-accent/70">03 / Deployment</span>
				<h2 class="text-3xl font-semibold tracking-tight text-text mt-3 sm:text-4xl">
					Local or cloud. Your call.
				</h2>
			</div>
		</InView>

		<div class="grid gap-px bg-border md:grid-cols-2 border border-border">
			<InView delay={0}>
				<div class="bg-bg p-8 md:p-10 h-full">
					<div class="font-mono text-[10px] text-text-muted uppercase tracking-wider mb-6">Local</div>
					<h3 class="text-2xl font-semibold text-text mb-4">Free, forever</h3>
					<p class="text-sm text-text-secondary leading-relaxed mb-8">
						Run the daemon on your machine. SQLite storage. Zero dependencies beyond the binary.
					</p>
					<ul class="space-y-3 text-sm mb-8">
						{#each [
							'Single binary, no containers',
							'SQLite storage (zero config)',
							'Full dashboard & API',
							'OpenAI proxy built in',
							'Unlimited traces'
						] as item}
							<li class="flex items-center gap-3 text-text-secondary">
								<span class="text-accent text-[10px] font-mono">+</span>
								{item}
							</li>
						{/each}
					</ul>
					<code class="text-[11px] font-mono text-text-muted bg-bg-tertiary px-3 py-1.5 rounded border border-border">
						cargo install --path crates/daemon
					</code>
				</div>
			</InView>

			<InView delay={80}>
				<div class="bg-bg p-8 md:p-10 relative h-full">
					<div class="absolute top-0 right-0 w-48 h-48 bg-accent/[0.03] blur-[80px] rounded-full"></div>
					<div class="relative">
						<div class="font-mono text-[10px] text-accent uppercase tracking-wider mb-6">Cloud</div>
						<h3 class="text-2xl font-semibold text-text mb-4">Production scale</h3>
						<p class="text-sm text-text-secondary leading-relaxed mb-8">
							Same daemon, cloud storage. Postgres for traces, Turbopuffer for vector search.
						</p>
						<ul class="space-y-3 text-sm mb-8">
							{#each [
								'Postgres + Turbopuffer backend',
								'Team access & auth',
								'High availability',
								'Same API & dashboard',
								'Managed or self-hosted'
							] as item}
								<li class="flex items-center gap-3 text-text-secondary">
									<span class="text-accent text-[10px] font-mono">+</span>
									{item}
								</li>
							{/each}
						</ul>
						<a
							href="https://platform.traceway.dev"
							class="inline-flex items-center gap-2 font-mono text-[11px] uppercase text-accent border border-accent/30 rounded-full px-5 py-2 transition-all hover:bg-accent/10"
						>
							Get started
						</a>
					</div>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- CODE SNIPPET -->
<section class="py-28 relative">
	<div class="px-6 md:px-10 max-w-[1400px]">
		<InView>
			<div class="flex flex-col md:flex-row md:items-end md:justify-between gap-4 mb-16">
				<div>
					<span class="font-mono text-[11px] uppercase tracking-widest text-accent/70">04 / Integration</span>
					<h2 class="text-3xl font-semibold tracking-tight text-text mt-3 sm:text-4xl">
						One line to start tracing
					</h2>
				</div>
				<p class="text-sm text-text-secondary max-w-sm leading-relaxed">
					Change your base_url to the Traceway proxy. No SDK, no wrapper, no other code changes.
				</p>
			</div>
		</InView>

		<InView delay={80}>
			<div class="max-w-2xl">
				<div class="rounded border border-border bg-bg-secondary overflow-hidden">
					<div class="flex items-center justify-between border-b border-border/60 px-4 py-2.5">
						<span class="text-[10px] text-text-muted font-mono uppercase">app.py</span>
						<span class="text-[10px] text-text-muted font-mono uppercase">python</span>
					</div>
					{@html `<div class="p-5 font-mono text-[12px] leading-relaxed overflow-x-auto">
<div><span class="text-text-muted">import</span> <span class="text-text">openai</span></div>
<div class="mt-3"></div>
<div class="text-text-muted"># Point at the Traceway proxy instead of OpenAI directly</div>
<div><span class="text-text">client</span> <span class="text-text-muted">=</span> <span class="text-text">openai.OpenAI(</span></div>
<div class="text-accent">    base_url=<span class="text-accent-dim">"http://localhost:3001/v1"</span><span class="text-text-muted">,</span>  <span class="text-text-muted"># proxy on :3001</span></div>
<div><span class="text-text">)</span></div>
<div class="mt-3"></div>
<div class="text-text-muted"># Use normally. Every call is traced automatically.</div>
<div><span class="text-text">response</span> <span class="text-text-muted">=</span> <span class="text-text">client.chat.completions.create(</span></div>
<div>    <span class="text-text">model=</span><span class="text-accent-dim">"gpt-4o"</span><span class="text-text-muted">,</span></div>
<div>    <span class="text-text">messages=[{"role": "user", "content": "Hello!"}]</span></div>
<div><span class="text-text">)</span></div>
</div>`}
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- DASHBOARD PREVIEW -->
<section class="py-28 relative">
	<div class="px-6 md:px-10 max-w-[1400px]">
		<InView>
			<div class="mb-12">
				<span class="font-mono text-[11px] uppercase tracking-widest text-accent/70">05 / Dashboard</span>
				<h2 class="text-3xl font-semibold tracking-tight text-text mt-3 sm:text-4xl">
					See your traces in real time
				</h2>
			</div>
		</InView>

		<InView delay={80}>
			<div class="max-w-5xl rounded border border-border bg-bg-secondary overflow-hidden shadow-2xl shadow-black/30">
				<!-- Window chrome -->
				<div class="border-b border-border/60 px-4 py-2.5 flex items-center gap-3">
					<div class="h-2 w-2 rounded-full bg-text-muted/25"></div>
					<div class="h-2 w-2 rounded-full bg-text-muted/25"></div>
					<div class="h-2 w-2 rounded-full bg-text-muted/25"></div>
					<div class="ml-6 flex-1 h-4 rounded bg-bg-tertiary max-w-xs"></div>
				</div>
				<div class="p-5 min-h-[340px] flex flex-col gap-4">
					<div class="flex gap-4 h-full">
						<!-- Sidebar -->
						<div class="hidden sm:flex flex-col gap-2 w-36 shrink-0">
							{#each ['Traces', 'Analytics', 'Models', 'Settings'] as item, i}
								<div class="h-7 rounded {i === 0 ? 'bg-accent/10 border border-accent/20' : 'bg-bg-tertiary/50'} flex items-center px-3">
									<span class="text-[10px] font-mono uppercase {i === 0 ? 'text-accent' : 'text-text-muted'}">{item}</span>
								</div>
							{/each}
						</div>
						<!-- Content -->
						<div class="flex-1 flex flex-col gap-3">
							<div class="grid grid-cols-3 gap-3">
								{#each [
									{ label: 'Total traces', value: '12,847' },
									{ label: 'Avg latency', value: '234ms' },
									{ label: 'Total cost', value: '$4.82' }
								] as stat}
									<div class="rounded bg-bg-tertiary/50 p-3 border border-border/30">
										<div class="text-[9px] text-text-muted font-mono uppercase">{stat.label}</div>
										<div class="text-lg font-semibold text-text mt-1 font-mono">{stat.value}</div>
									</div>
								{/each}
							</div>
							<div class="flex-1 rounded bg-bg-tertiary/50 border border-border/30 min-h-[180px] flex items-end p-4 gap-[3px]">
								{#each [35, 42, 55, 38, 62, 71, 45, 58, 80, 65, 72, 90, 68, 55, 78, 85, 70, 62, 88, 75, 60, 82, 69, 77] as h}
									<div class="flex-1 rounded-t bg-accent/15 transition-all" style="height: {h}%"></div>
								{/each}
							</div>
						</div>
					</div>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- CTA — minimal, bottom-of-page -->
<section class="py-32 relative">
	<div class="absolute inset-0 bg-gradient-to-b from-transparent via-accent/[0.015] to-transparent"></div>
	<div class="relative px-6 md:px-10 max-w-[1400px]">
		<InView>
			<div class="flex flex-col md:flex-row md:items-end md:justify-between gap-8">
				<div>
					<h2 class="font-mono text-[clamp(2rem,5vw,4rem)] font-semibold leading-[1] tracking-tighter text-text uppercase">
						Start tracing<br />in under a minute
					</h2>
					<p class="mt-4 text-text-secondary max-w-sm text-sm leading-relaxed">
						One binary. No config. See your first traces immediately.
					</p>
				</div>
				<div class="flex items-center gap-5">
					<a
						href="https://platform.traceway.dev"
						class="group flex items-center gap-2 bg-accent text-bg font-mono text-sm uppercase tracking-wide px-6 py-2.5 rounded transition-all hover:brightness-110 hover:shadow-[0_0_30px_rgba(110,231,183,0.2)]"
					>
						Get started free
						<svg class="h-3.5 w-3.5 transition-transform group-hover:translate-x-1" viewBox="0 0 8 8" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M0.826 7.536L0.112 6.822L5.768 1.18H2.03V0.2H7.462V5.66H6.426V1.936L0.826 7.536Z" fill="currentColor"/></svg>
					</a>
					<a
						href="https://github.com/traceway"
						target="_blank"
						rel="noopener"
						class="font-mono text-[12px] uppercase text-text-secondary hover:text-text transition-colors border-b border-text-muted hover:border-text-secondary pb-0.5"
					>
						Star on GitHub
					</a>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- FOOTER — minimal -->
<footer class="border-t border-border/50 py-8">
	<div class="px-6 md:px-10 max-w-[1400px]">
		<div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
			<span class="font-mono text-[11px] uppercase text-text-muted tracking-tight">traceway</span>
			<div class="flex items-center gap-6">
				<a href="https://docs.traceway.dev" class="font-mono text-[11px] uppercase text-text-muted hover:text-text-secondary transition-colors">Docs</a>
				<a href="https://github.com/traceway" target="_blank" rel="noopener" class="font-mono text-[11px] uppercase text-text-muted hover:text-text-secondary transition-colors">GitHub</a>
				<a href="https://platform.traceway.dev" class="font-mono text-[11px] uppercase text-text-muted hover:text-text-secondary transition-colors">Platform</a>
			</div>
			<span class="font-mono text-[10px] text-text-muted/50">MIT Licensed</span>
		</div>
	</div>
</footer>
