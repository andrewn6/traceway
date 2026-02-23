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
			<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="font-mono text-[11px] uppercase text-white/50 hover:text-white transition-colors">GitHub</a>
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

	<div class="relative mx-auto max-w-[1400px] px-6 md:px-16 lg:px-24">
		<div class="flex flex-col lg:flex-row lg:items-center lg:justify-between lg:gap-16">
			<!-- Left: text content -->
			<InView>
				<div class="flex flex-col gap-6 pt-32 lg:pt-0 lg:max-w-xl lg:shrink-0">
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
							href="https://github.com/blastgits/traceway"
							target="_blank"
							rel="noopener"
							class="font-mono text-[12px] uppercase text-text-secondary hover:text-text transition-colors border-b border-text-muted hover:border-text-secondary pb-0.5"
						>
							View source
						</a>
					</div>
				</div>
			</InView>

			<!-- Right: terminal -->
			<InView delay={200}>
				<div class="mt-16 lg:mt-0 w-full lg:max-w-[520px]">
					<div class="rounded border border-border bg-bg-secondary/80 backdrop-blur overflow-hidden shadow-2xl shadow-black/30">
						<div class="flex items-center gap-2 border-b border-border/60 px-4 py-2.5">
							<div class="h-2 w-2 rounded-full bg-text-muted/30"></div>
							<div class="h-2 w-2 rounded-full bg-text-muted/30"></div>
							<div class="h-2 w-2 rounded-full bg-text-muted/30"></div>
							<span class="ml-3 text-[10px] text-text-muted font-mono uppercase">terminal</span>
						</div>
						<div class="p-4 font-mono text-[12px] leading-relaxed">
							<div class="text-text-muted">$ cargo install --path crates/daemon</div>
							<div class="mt-3 text-text-muted"># works with any provider — OpenAI, Anthropic, Ollama, etc.</div>
							<div class="mt-0.5 text-text-muted">$ traceway --foreground --target-url https://api.openai.com</div>
							<div class="mt-0.5 text-accent-dim">&#10003; API server on 127.0.0.1:3000</div>
							<div class="mt-0.5 text-accent-dim">&#10003; Proxy on 127.0.0.1:3001 &#8594; https://api.openai.com</div>
							<div class="mt-3 text-text-muted">$ python app.py  <span class="text-text-muted/50"># point your client at :3001</span></div>
							<div class="mt-0.5 text-accent-dim">&#10003; 3 traces captured (247ms avg, $0.0012 total)</div>
							<div class="mt-0.5 text-text-secondary">Dashboard: <span class="text-accent">http://localhost:3000</span></div>
						</div>
					</div>
				</div>
			</InView>
		</div>
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
					title: 'Any LLM provider',
					desc: 'Transparent proxy that works with OpenAI, Anthropic, Ollama, or any OpenAI-compatible API. Auto-detects provider for token extraction.',
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
							'Transparent LLM proxy built in',
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
					Change your base_url to the Traceway proxy. Works with any provider. No SDK, no wrapper, no other code changes.
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
<div class="text-text-muted"># Just change base_url to point at the Traceway proxy</span></div>
<div class="text-text-muted"># Works with OpenAI, Anthropic, Ollama — any provider</div>
<div><span class="text-text">client</span> <span class="text-text-muted">=</span> <span class="text-text">openai.OpenAI(</span></div>
<div class="text-accent">    base_url=<span class="text-accent-dim">"http://localhost:3001/v1"</span>  <span class="text-text-muted"># Traceway proxy</span></div>
<div><span class="text-text">)</span></div>
<div class="mt-3"></div>
<div class="text-text-muted"># Every call is traced automatically — no other changes needed</div>
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
				<span class="font-mono text-[11px] uppercase tracking-widest text-accent/70">05 / Trace hierarchy</span>
				<h2 class="text-3xl font-semibold tracking-tight text-text mt-3 sm:text-4xl">
					Every call, every span, every token
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
					<span class="ml-3 text-[10px] text-text-muted font-mono uppercase">Trace detail</span>
					<div class="ml-auto text-[10px] text-text-muted font-mono">1.24s total</div>
				</div>

				<!-- Trace header -->
				<div class="border-b border-border/40 px-5 py-3 flex items-center justify-between">
					<div class="flex items-center gap-3">
						<span class="text-accent font-mono text-sm font-medium">chat-with-rag</span>
						<span class="text-[10px] font-mono text-text-muted bg-bg-tertiary px-2 py-0.5 rounded">trace</span>
						<span class="text-[10px] font-mono text-accent/60 bg-accent/5 px-2 py-0.5 rounded border border-accent/10">ok</span>
					</div>
					<div class="flex items-center gap-4 text-[10px] font-mono text-text-muted">
						<span>7 spans</span>
						<span>$0.0034</span>
					</div>
				</div>

				<!-- Trace waterfall -->
				<div class="p-5 space-y-1">
					<!-- Time axis -->
					<div class="flex items-center mb-3 pl-[220px] sm:pl-[280px]">
						<div class="flex-1 flex justify-between text-[9px] font-mono text-text-muted/50">
							<span>0ms</span>
							<span>250ms</span>
							<span>500ms</span>
							<span>750ms</span>
							<span>1000ms</span>
							<span>1240ms</span>
						</div>
					</div>

					{#each [
						{ name: 'chat-with-rag', kind: 'trace', depth: 0, start: 0, width: 100, dur: '1240ms', color: 'bg-accent/25', border: 'border-accent/40' },
						{ name: 'embed-query', kind: 'llm_call', depth: 1, start: 2, width: 12, dur: '148ms', color: 'bg-purple-400/25', border: 'border-purple-400/40', model: 'text-embedding-3-small' },
						{ name: 'vector-search', kind: 'custom', depth: 1, start: 14, width: 8, dur: '95ms', color: 'bg-cyan-400/25', border: 'border-cyan-400/40' },
						{ name: 'build-context', kind: 'custom', depth: 1, start: 22, width: 4, dur: '45ms', color: 'bg-text-muted/15', border: 'border-text-muted/30' },
						{ name: 'chat-completion', kind: 'llm_call', depth: 1, start: 26, width: 70, dur: '872ms', color: 'bg-accent/25', border: 'border-accent/40', model: 'gpt-4o' },
						{ name: 'tool: search_docs', kind: 'custom', depth: 2, start: 32, width: 18, dur: '224ms', color: 'bg-amber-400/20', border: 'border-amber-400/30' },
						{ name: 'chat-completion', kind: 'llm_call', depth: 2, start: 52, width: 42, dur: '520ms', color: 'bg-accent/25', border: 'border-accent/40', model: 'gpt-4o' },
					] as span}
						<div class="flex items-center group hover:bg-bg-tertiary/30 rounded transition-colors py-1 px-1 -mx-1">
							<!-- Span name column -->
							<div class="w-[210px] sm:w-[270px] shrink-0 flex items-center gap-1.5" style="padding-left: {span.depth * 20}px;">
								{#if span.depth > 0}
									<span class="text-border text-[10px]">└</span>
								{/if}
								<span class="font-mono text-[11px] text-text truncate">{span.name}</span>
								{#if span.model}
									<span class="text-[9px] font-mono text-text-muted/60 hidden sm:inline">{span.model}</span>
								{/if}
							</div>
							<!-- Waterfall bar -->
							<div class="flex-1 relative h-5">
								<div
									class="absolute top-0.5 h-4 rounded-sm {span.color} border {span.border} transition-all group-hover:brightness-125 flex items-center"
									style="left: {span.start}%; width: {span.width}%;"
								>
									<span class="text-[9px] font-mono text-text-secondary px-1.5 truncate">{span.dur}</span>
								</div>
							</div>
						</div>
					{/each}

					<!-- Span detail hint -->
					<div class="pt-3 mt-2 border-t border-border/30 flex items-center gap-4 text-[10px] font-mono text-text-muted">
						<span class="flex items-center gap-1.5"><span class="w-2 h-2 rounded-sm bg-accent/25 border border-accent/40"></span> llm_call</span>
						<span class="flex items-center gap-1.5"><span class="w-2 h-2 rounded-sm bg-purple-400/25 border border-purple-400/40"></span> embedding</span>
						<span class="flex items-center gap-1.5"><span class="w-2 h-2 rounded-sm bg-amber-400/20 border border-amber-400/30"></span> tool_call</span>
						<span class="flex items-center gap-1.5"><span class="w-2 h-2 rounded-sm bg-text-muted/15 border border-text-muted/30"></span> custom</span>
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
						href="https://github.com/blastgits/traceway"
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
				<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="font-mono text-[11px] uppercase text-text-muted hover:text-text-secondary transition-colors">GitHub</a>
				<a href="https://platform.traceway.dev" class="font-mono text-[11px] uppercase text-text-muted hover:text-text-secondary transition-colors">Platform</a>
			</div>
			<span class="font-mono text-[10px] text-text-muted/50">MIT Licensed</span>
		</div>
	</div>
</footer>
