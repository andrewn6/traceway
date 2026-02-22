<script>
	import InView from '$lib/InView.svelte';

	let scrollY = $state(0);
	let navOpaque = $derived(scrollY > 40);
</script>

<svelte:window bind:scrollY={scrollY} />

<!-- NAV -->
<nav
	class="fixed top-0 left-0 right-0 z-50 transition-all duration-300 {navOpaque
		? 'bg-bg/80 backdrop-blur-xl border-b border-border'
		: 'bg-transparent'}"
>
	<div class="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
		<a href="/" class="flex items-center gap-2.5">
			<div class="flex h-7 w-7 items-center justify-center rounded-md bg-accent/10 border border-accent/20">
				<span class="text-accent font-mono text-sm font-bold">T</span>
			</div>
			<span class="text-text font-medium text-[15px] tracking-tight">traceway</span>
		</a>
		<div class="hidden items-center gap-8 md:flex">
			<a href="#features" class="text-text-secondary hover:text-text text-sm transition-colors">Features</a>
			<a href="#how-it-works" class="text-text-secondary hover:text-text text-sm transition-colors">How it works</a>
			<a href="#pricing" class="text-text-secondary hover:text-text text-sm transition-colors">Pricing</a>
			<a href="https://github.com/traceway" target="_blank" rel="noopener" class="text-text-secondary hover:text-text text-sm transition-colors">GitHub</a>
		</div>
		<div class="flex items-center gap-3">
			<a
				href="https://docs.traceway.dev"
				class="text-text-secondary hover:text-text hidden text-sm transition-colors sm:block"
			>
				Docs
			</a>
			<a
				href="https://platform.traceway.dev"
				class="rounded-lg bg-accent/10 px-4 py-1.5 text-sm font-medium text-accent border border-accent/20 transition-all hover:bg-accent/20"
			>
				Open app
			</a>
		</div>
	</div>
</nav>

<!-- HERO -->
<section class="relative flex min-h-screen items-center overflow-hidden pt-20">
	<!-- Subtle grid background -->
	<div class="absolute inset-0 opacity-[0.03]" style="background-image: url(&quot;data:image/svg+xml,%3Csvg width='60' height='60' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M60 0H0v60' fill='none' stroke='%23fff' stroke-width='0.5'/%3E%3C/svg%3E&quot;);"></div>
	<!-- Radial glow -->
	<div class="absolute top-1/4 left-1/2 -translate-x-1/2 -translate-y-1/2 h-[600px] w-[800px] rounded-full bg-accent/[0.04] blur-[120px]"></div>

	<div class="relative mx-auto max-w-6xl px-6">
		<InView>
			<div class="flex flex-col items-center text-center">
				<div class="mb-6 inline-flex items-center gap-2 rounded-full border border-border bg-bg-secondary px-4 py-1.5">
					<span class="h-1.5 w-1.5 rounded-full bg-accent animate-pulse"></span>
					<span class="text-text-secondary text-xs font-mono">open source</span>
				</div>

				<h1 class="max-w-3xl text-4xl font-semibold tracking-tight text-text sm:text-5xl md:text-6xl lg:text-7xl leading-[1.1]">
					Observability for
					<span class="text-accent">LLM apps</span>
				</h1>

				<p class="mt-6 max-w-xl text-base text-text-secondary leading-relaxed sm:text-lg">
					Trace every call. Measure latency and cost. Debug prompts in real time.
					Run locally or deploy to the cloud.
				</p>

				<div class="mt-10 flex flex-col items-center gap-4 sm:flex-row">
					<a
						href="https://platform.traceway.dev"
						class="group flex items-center gap-2 rounded-lg bg-accent px-6 py-2.5 text-sm font-medium text-bg transition-all hover:brightness-110 hover:shadow-[0_0_24px_rgba(110,231,183,0.25)]"
					>
						Get started
						<svg class="h-4 w-4 transition-transform group-hover:translate-x-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path d="M5 12h14M12 5l7 7-7 7" />
						</svg>
					</a>
					<a
						href="https://github.com/traceway"
						target="_blank"
						rel="noopener"
						class="flex items-center gap-2 rounded-lg border border-border bg-bg-secondary px-6 py-2.5 text-sm font-medium text-text-secondary transition-all hover:border-border hover:text-text hover:bg-bg-tertiary"
					>
						<svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
						View on GitHub
					</a>
				</div>

				<!-- Terminal preview / code snippet -->
				<div class="mt-16 w-full max-w-2xl">
					<div class="rounded-xl border border-border bg-bg-secondary overflow-hidden shadow-2xl shadow-black/20">
						<div class="flex items-center gap-2 border-b border-border px-4 py-3">
							<div class="h-2.5 w-2.5 rounded-full bg-text-muted/40"></div>
							<div class="h-2.5 w-2.5 rounded-full bg-text-muted/40"></div>
							<div class="h-2.5 w-2.5 rounded-full bg-text-muted/40"></div>
							<span class="ml-3 text-xs text-text-muted font-mono">terminal</span>
						</div>
						<div class="p-5 font-mono text-[13px] leading-relaxed">
							<div class="text-text-muted">$ pip install traceway</div>
							<div class="mt-3 text-text-muted">$ traceway init</div>
							<div class="mt-1 text-accent-dim">&#10003; Connected to traceway daemon</div>
							<div class="mt-3 text-text-muted">$ python app.py</div>
							<div class="mt-1 text-text-secondary">Tracing OpenAI calls...</div>
							<div class="mt-1 text-accent-dim">&#10003; 3 traces captured (247ms avg, $0.0012 total)</div>
							<div class="mt-1 text-text-secondary">Dashboard: <span class="text-accent underline">http://localhost:3000</span></div>
						</div>
					</div>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- FEATURES -->
<section id="features" class="py-32 relative">
	<div class="mx-auto max-w-6xl px-6">
		<InView>
			<div class="text-center mb-16">
				<p class="text-accent font-mono text-sm mb-3">Features</p>
				<h2 class="text-3xl font-semibold tracking-tight text-text sm:text-4xl">
					Everything you need to debug LLM apps
				</h2>
				<p class="mt-4 text-text-secondary max-w-lg mx-auto">
					From local development to production monitoring.
					One tool for the entire lifecycle.
				</p>
			</div>
		</InView>

		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each [
				{
					title: 'Full trace capture',
					desc: 'Every LLM call logged automatically — inputs, outputs, latency, tokens, cost. Nested spans for complex chains.',
					icon: '&#9776;',
					delay: 0
				},
				{
					title: 'Real-time dashboard',
					desc: 'Live analytics with interactive charts. Filter by model, time range, status. See trends at a glance.',
					icon: '&#9632;',
					delay: 80
				},
				{
					title: 'Local-first',
					desc: 'Runs as a lightweight daemon on your machine. No cloud account needed. Your data stays on disk.',
					icon: '&#8962;',
					delay: 160
				},
				{
					title: 'Cloud ready',
					desc: 'When you need scale, deploy to the cloud with Postgres + Turbopuffer. Same API, same dashboard.',
					icon: '&#9729;',
					delay: 0
				},
				{
					title: 'OpenAI compatible',
					desc: 'Drop-in proxy that intercepts OpenAI API calls. No code changes required. Works with any OpenAI-compatible provider.',
					icon: '&#9889;',
					delay: 80
				},
				{
					title: 'Open source',
					desc: 'MIT licensed. Self-host, fork, contribute. Built in Rust for speed and reliability.',
					icon: '&#9734;',
					delay: 160
				}
			] as card}
				<InView delay={card.delay}>
					<div class="group rounded-xl border border-border bg-bg-card p-6 transition-all duration-300 hover:border-accent/20 hover:bg-bg-secondary">
						<div class="mb-4 flex h-10 w-10 items-center justify-center rounded-lg bg-accent/[0.07] text-accent text-lg border border-accent/10">
							{@html card.icon}
						</div>
						<h3 class="text-[15px] font-medium text-text mb-2">{card.title}</h3>
						<p class="text-sm text-text-secondary leading-relaxed">{card.desc}</p>
					</div>
				</InView>
			{/each}
		</div>
	</div>
</section>

<!-- HOW IT WORKS -->
<section id="how-it-works" class="py-32 relative border-t border-border/50">
	<div class="mx-auto max-w-6xl px-6">
		<InView>
			<div class="text-center mb-16">
				<p class="text-accent font-mono text-sm mb-3">Architecture</p>
				<h2 class="text-3xl font-semibold tracking-tight text-text sm:text-4xl">
					How Traceway works
				</h2>
				<p class="mt-4 text-text-secondary max-w-lg mx-auto">
					A lightweight proxy sits between your app and the LLM provider,
					capturing every request.
				</p>
			</div>
		</InView>

		<InView delay={100}>
			<div class="mx-auto max-w-3xl">
				<div class="rounded-xl border border-border bg-bg-secondary overflow-hidden">
					<div class="flex items-center gap-2 border-b border-border px-4 py-3">
						<span class="text-xs text-text-muted font-mono">architecture</span>
					</div>
					{@html `<pre class="p-6 font-mono text-xs sm:text-sm leading-relaxed text-text-secondary overflow-x-auto"><code>  <span class="text-text">Your App</span>            <span class="text-text">Traceway Daemon</span>         <span class="text-text">LLM Provider</span>
  <span class="text-accent/60">┌─────────┐</span>          <span class="text-accent/60">┌──────────────┐</span>       <span class="text-accent/60">┌────────────┐</span>
  <span class="text-accent/60">│</span>         <span class="text-accent/60">│</span>  request  <span class="text-accent/60">│</span>   <span class="text-accent">Proxy</span>      <span class="text-accent/60">│</span>       <span class="text-accent/60">│</span>            <span class="text-accent/60">│</span>
  <span class="text-accent/60">│</span>  LLM    <span class="text-accent/60">│</span>─────────▶<span class="text-accent/60">│</span>   ┃          <span class="text-accent/60">│</span>──────▶<span class="text-accent/60">│</span>  OpenAI   <span class="text-accent/60">│</span>
  <span class="text-accent/60">│</span>  calls  <span class="text-accent/60">│</span>◁─────────<span class="text-accent/60">│</span>   ┃ <span class="text-accent">Trace</span>    <span class="text-accent/60">│</span>◁──────<span class="text-accent/60">│</span>  Anthropic<span class="text-accent/60">│</span>
  <span class="text-accent/60">│</span>         <span class="text-accent/60">│</span>  response <span class="text-accent/60">│</span>   ▼          <span class="text-accent/60">│</span>       <span class="text-accent/60">│</span>  etc.     <span class="text-accent/60">│</span>
  <span class="text-accent/60">└─────────┘</span>          <span class="text-accent/60">│</span> <span class="text-accent/60">┌──────────┐</span> <span class="text-accent/60">│</span>       <span class="text-accent/60">└────────────┘</span>
                       <span class="text-accent/60">│</span> <span class="text-accent/60">│</span> <span class="text-accent">Storage</span>  <span class="text-accent/60">│</span> <span class="text-accent/60">│</span>
                       <span class="text-accent/60">│</span> <span class="text-accent/60">│</span> <span class="text-text-muted">SQLite /</span> <span class="text-accent/60">│</span> <span class="text-accent/60">│</span>
  <span class="text-accent/60">┌─────────┐</span>          <span class="text-accent/60">│</span> <span class="text-accent/60">│</span> <span class="text-text-muted">Postgres</span> <span class="text-accent/60">│</span> <span class="text-accent/60">│</span>
  <span class="text-accent/60">│</span>         <span class="text-accent/60">│</span>   API    <span class="text-accent/60">│</span> <span class="text-accent/60">└──────────┘</span> <span class="text-accent/60">│</span>
  <span class="text-accent/60">│</span>  <span class="text-accent">Web UI</span> <span class="text-accent/60">│</span>◁────────▶<span class="text-accent/60">│</span>   <span class="text-accent">REST API</span>   <span class="text-accent/60">│</span>
  <span class="text-accent/60">│</span>         <span class="text-accent/60">│</span>          <span class="text-accent/60">│</span>              <span class="text-accent/60">│</span>
  <span class="text-accent/60">└─────────┘</span>          <span class="text-accent/60">└──────────────┘</span></code></pre>`}
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- LOCAL vs CLOUD -->
<section id="pricing" class="py-32 relative border-t border-border/50">
	<div class="mx-auto max-w-6xl px-6">
		<InView>
			<div class="text-center mb-16">
				<p class="text-accent font-mono text-sm mb-3">Deployment</p>
				<h2 class="text-3xl font-semibold tracking-tight text-text sm:text-4xl">
					Local or cloud. Your call.
				</h2>
				<p class="mt-4 text-text-secondary max-w-lg mx-auto">
					Start on your laptop in seconds. Scale to production when ready.
				</p>
			</div>
		</InView>

		<div class="grid gap-6 md:grid-cols-2 max-w-4xl mx-auto">
			<InView delay={0}>
				<div class="rounded-xl border border-border bg-bg-card p-8 h-full">
					<div class="mb-4 font-mono text-xs text-accent/70 uppercase tracking-wider">Local</div>
					<h3 class="text-xl font-semibold text-text mb-3">Free, forever</h3>
					<p class="text-sm text-text-secondary leading-relaxed mb-6">
						Run the daemon on your machine. SQLite storage. Zero dependencies beyond the binary.
					</p>
					<ul class="space-y-3 text-sm">
						{#each [
							'Single binary, no containers',
							'SQLite storage (zero config)',
							'Full dashboard & API',
							'OpenAI proxy built in',
							'Unlimited traces'
						] as item}
							<li class="flex items-center gap-3 text-text-secondary">
								<span class="text-accent text-xs">&#10003;</span>
								{item}
							</li>
						{/each}
					</ul>
					<div class="mt-8">
						<code class="text-xs font-mono text-text-muted bg-bg-tertiary px-3 py-1.5 rounded-md border border-border">
							curl -sSf https://traceway.dev/install | sh
						</code>
					</div>
				</div>
			</InView>

			<InView delay={100}>
				<div class="rounded-xl border border-accent/20 bg-bg-card p-8 relative overflow-hidden h-full">
					<div class="absolute top-0 right-0 w-32 h-32 bg-accent/[0.03] blur-3xl rounded-full"></div>
					<div class="relative">
						<div class="mb-4 font-mono text-xs text-accent uppercase tracking-wider">Cloud</div>
						<h3 class="text-xl font-semibold text-text mb-3">Production scale</h3>
						<p class="text-sm text-text-secondary leading-relaxed mb-6">
							Same daemon, cloud storage. Postgres for traces, Turbopuffer for vector search.
						</p>
						<ul class="space-y-3 text-sm">
							{#each [
								'Postgres + Turbopuffer backend',
								'Team access & auth',
								'High availability',
								'Same API & dashboard',
								'Managed or self-hosted'
							] as item}
								<li class="flex items-center gap-3 text-text-secondary">
									<span class="text-accent text-xs">&#10003;</span>
									{item}
								</li>
							{/each}
						</ul>
						<div class="mt-8">
							<a
								href="https://platform.traceway.dev"
								class="inline-flex items-center gap-2 rounded-lg bg-accent/10 px-4 py-2 text-sm font-medium text-accent border border-accent/20 transition-all hover:bg-accent/20"
							>
								Get started
							</a>
						</div>
					</div>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- CODE SNIPPET -->
<section class="py-32 relative border-t border-border/50">
	<div class="mx-auto max-w-6xl px-6">
		<InView>
			<div class="text-center mb-16">
				<p class="text-accent font-mono text-sm mb-3">Integration</p>
				<h2 class="text-3xl font-semibold tracking-tight text-text sm:text-4xl">
					Two lines to start tracing
				</h2>
				<p class="mt-4 text-text-secondary max-w-lg mx-auto">
					Point your OpenAI client at the Traceway proxy. That's it.
				</p>
			</div>
		</InView>

		<InView delay={80}>
			<div class="mx-auto max-w-2xl">
				<div class="rounded-xl border border-border bg-bg-secondary overflow-hidden">
					<div class="flex items-center justify-between border-b border-border px-4 py-3">
						<span class="text-xs text-text-muted font-mono">app.py</span>
						<span class="text-xs text-text-muted font-mono">python</span>
					</div>
					<div class="p-5 font-mono text-[13px] leading-relaxed overflow-x-auto">
						<div><span class="text-text-muted">import</span> <span class="text-text">openai</span></div>
						<div class="mt-2"></div>
						<div><span class="text-text">client</span> <span class="text-text-muted">=</span> <span class="text-text">openai.OpenAI(</span></div>
						<div class="text-accent">    base_url=<span class="text-accent-dim">"http://localhost:3000/proxy/v1"</span><span class="text-text-muted">,</span>  <span class="text-text-muted"># &#8592; Traceway proxy</span></div>
						<div><span class="text-text">)</span></div>
						<div class="mt-2"></div>
						<div class="text-text-muted"># Use the client normally — everything is traced</div>
						<div><span class="text-text">response</span> <span class="text-text-muted">=</span> <span class="text-text">client.chat.completions.create(</span></div>
						<div>    <span class="text-text">model=</span><span class="text-accent-dim">"gpt-4o"</span><span class="text-text-muted">,</span></div>
						<div>    <span class="text-text">messages=[{</span><span class="text-accent-dim">"role"</span><span class="text-text-muted">:</span> <span class="text-accent-dim">"user"</span><span class="text-text-muted">,</span> <span class="text-accent-dim">"content"</span><span class="text-text-muted">:</span> <span class="text-accent-dim">"Hello!"</span><span class="text-text">}]</span></div>
						<div><span class="text-text">)</span></div>
					</div>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- SCREENSHOT PLACEHOLDER -->
<section class="py-32 relative border-t border-border/50">
	<div class="mx-auto max-w-6xl px-6">
		<InView>
			<div class="text-center mb-12">
				<h2 class="text-3xl font-semibold tracking-tight text-text sm:text-4xl">
					See your traces in real time
				</h2>
				<p class="mt-4 text-text-secondary max-w-lg mx-auto">
					Interactive dashboard with filtering, search, and analytics built in.
				</p>
			</div>
		</InView>

		<InView delay={80}>
			<div class="mx-auto max-w-4xl rounded-xl border border-border bg-bg-secondary overflow-hidden shadow-2xl shadow-black/30">
				<!-- Fake dashboard screenshot placeholder -->
				<div class="border-b border-border px-4 py-2.5 flex items-center gap-3">
					<div class="h-2.5 w-2.5 rounded-full bg-text-muted/30"></div>
					<div class="h-2.5 w-2.5 rounded-full bg-text-muted/30"></div>
					<div class="h-2.5 w-2.5 rounded-full bg-text-muted/30"></div>
					<div class="ml-6 flex-1 h-5 rounded bg-bg-tertiary max-w-xs"></div>
				</div>
				<div class="p-6 min-h-[320px] flex flex-col gap-4">
					<!-- Fake sidebar + content -->
					<div class="flex gap-4 h-full">
						<div class="hidden sm:flex flex-col gap-3 w-40 shrink-0">
							{#each ['Traces', 'Analytics', 'Models', 'Settings'] as item, i}
								<div class="h-8 rounded-md {i === 0 ? 'bg-accent/10 border border-accent/20' : 'bg-bg-tertiary'} flex items-center px-3">
									<span class="text-xs font-mono {i === 0 ? 'text-accent' : 'text-text-muted'}">{item}</span>
								</div>
							{/each}
						</div>
						<div class="flex-1 flex flex-col gap-3">
							<!-- Fake stat cards -->
							<div class="grid grid-cols-3 gap-3">
								{#each [
									{ label: 'Total traces', value: '12,847' },
									{ label: 'Avg latency', value: '234ms' },
									{ label: 'Total cost', value: '$4.82' }
								] as stat}
									<div class="rounded-lg bg-bg-tertiary p-3 border border-border/50">
										<div class="text-[10px] text-text-muted font-mono uppercase">{stat.label}</div>
										<div class="text-lg font-semibold text-text mt-1 font-mono">{stat.value}</div>
									</div>
								{/each}
							</div>
							<!-- Fake chart area -->
							<div class="flex-1 rounded-lg bg-bg-tertiary border border-border/50 min-h-[160px] flex items-end p-4 gap-1">
								{#each [35, 42, 55, 38, 62, 71, 45, 58, 80, 65, 72, 90, 68, 55, 78, 85, 70, 62, 88, 75] as h}
									<div class="flex-1 rounded-t bg-accent/20 transition-all" style="height: {h}%"></div>
								{/each}
							</div>
						</div>
					</div>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- CTA -->
<section class="py-32 relative border-t border-border/50">
	<div class="absolute inset-0 bg-gradient-to-b from-transparent via-accent/[0.02] to-transparent"></div>
	<div class="relative mx-auto max-w-6xl px-6 text-center">
		<InView>
			<h2 class="text-3xl font-semibold tracking-tight text-text sm:text-4xl">
				Start tracing in under a minute
			</h2>
			<p class="mt-4 text-text-secondary max-w-md mx-auto">
				One binary. No config needed. See your first traces immediately.
			</p>
			<div class="mt-10 flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
				<a
					href="https://platform.traceway.dev"
					class="group flex items-center gap-2 rounded-lg bg-accent px-6 py-2.5 text-sm font-medium text-bg transition-all hover:brightness-110 hover:shadow-[0_0_24px_rgba(110,231,183,0.25)]"
				>
					Get started free
					<svg class="h-4 w-4 transition-transform group-hover:translate-x-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path d="M5 12h14M12 5l7 7-7 7" />
					</svg>
				</a>
				<a
					href="https://github.com/traceway"
					target="_blank"
					rel="noopener"
					class="flex items-center gap-2 rounded-lg border border-border bg-bg-secondary px-6 py-2.5 text-sm font-medium text-text-secondary transition-all hover:text-text hover:bg-bg-tertiary"
				>
					<svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
					Star on GitHub
				</a>
			</div>
		</InView>
	</div>
</section>

<!-- FOOTER -->
<footer class="border-t border-border py-12">
	<div class="mx-auto max-w-6xl px-6">
		<div class="flex flex-col items-center justify-between gap-6 sm:flex-row">
			<div class="flex items-center gap-2.5">
				<div class="flex h-6 w-6 items-center justify-center rounded-md bg-accent/10 border border-accent/20">
					<span class="text-accent font-mono text-xs font-bold">T</span>
				</div>
				<span class="text-text-secondary text-sm">traceway</span>
			</div>
			<div class="flex items-center gap-6 text-sm text-text-muted">
				<a href="https://docs.traceway.dev" class="hover:text-text-secondary transition-colors">Docs</a>
				<a href="https://github.com/traceway" target="_blank" rel="noopener" class="hover:text-text-secondary transition-colors">GitHub</a>
				<a href="https://platform.traceway.dev" class="hover:text-text-secondary transition-colors">Platform</a>
			</div>
			<div class="text-xs text-text-muted font-mono">
				MIT Licensed
			</div>
		</div>
	</div>
</footer>
