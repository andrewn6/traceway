<script lang="ts">
	import InView from '$lib/InView.svelte';

	let scrollY = $state(0);
</script>

<svelte:window bind:scrollY />

<style>
	/* Grid background pattern */
	.grid-bg {
		background-image:
			linear-gradient(rgba(0, 0, 0, 0.03) 1px, transparent 1px),
			linear-gradient(90deg, rgba(0, 0, 0, 0.03) 1px, transparent 1px);
		background-size: 60px 60px;
	}

	/* Animated dot grid for hero */
	.dot-grid {
		background-image: radial-gradient(circle, #0a0a0a 1px, transparent 1px);
		background-size: 24px 24px;
		opacity: 0.08;
	}

	/* Dark feature card */
	.dark-card {
		background: #0f172a;
		border-radius: 16px;
		padding: 24px;
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	.dark-card-visual {
		flex: 1;
		min-height: 180px;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 20px;
	}
</style>

<!-- NAV -->
<nav class="fixed top-0 left-0 right-0 z-50 bg-white/80 backdrop-blur-xl border-b border-border/50">
	<div class="max-w-[1200px] mx-auto flex items-center justify-between px-6 py-3">
		<a href="/" class="flex items-center gap-2 shrink-0" aria-label="Traceway home">
			<svg class="h-5 w-auto" viewBox="0 0 164 30" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Traceway" role="img">
				<path d="M27.375 27.375H18.0938V24.2814H24.2814V18.0938H27.375V27.375Z" fill="currentColor"/>
				<path fill-rule="evenodd" clip-rule="evenodd" d="M11.7656 0.375C18.0565 0.375 23.1562 5.47476 23.1562 11.7656C23.1562 18.0565 18.0565 23.1562 11.7656 23.1562C5.47476 23.1562 0.375 18.0565 0.375 11.7656C0.375 5.47476 5.47476 0.375 11.7656 0.375ZM11.8711 3.75C7.38594 3.75 3.75 7.38594 3.75 11.8711C3.75 16.3562 7.38594 19.9922 11.8711 19.9922C16.3562 19.9922 19.9922 16.3562 19.9922 11.8711C19.9922 7.38594 16.3562 3.75 11.8711 3.75Z" fill="currentColor"/>
				<path d="M37.967 20.007V9.735H35.375V7.271H37.967V2.375H40.815V7.271H44.623V9.735H40.815V19.559C40.815 21.319 41.679 21.639 43.151 21.639C43.823 21.639 44.303 21.575 44.975 21.447V23.911C44.239 24.071 43.407 24.167 42.447 24.167C39.599 24.167 37.967 23.207 37.967 20.007Z" fill="currentColor"/>
				<path d="M57.066 7.207V10.119C56.586 10.023 56.202 9.991 55.658 9.991C53.226 9.991 51.274 11.911 51.274 14.855V24.007H48.394V7.271H51.274V10.119C52.01 8.423 53.642 7.143 55.914 7.143C56.362 7.143 56.778 7.175 57.066 7.207Z" fill="currentColor"/>
				<path d="M64.086 24.295C61.046 24.295 58.582 22.503 58.582 19.495C58.582 16.167 61.174 14.791 64.918 14.023L68.982 13.191V12.487C68.982 10.567 67.926 9.479 65.686 9.479C63.574 9.479 62.326 10.471 61.814 12.327L59.094 11.623C59.894 8.903 62.358 6.951 65.782 6.951C69.526 6.951 71.83 8.775 71.83 12.359V20.519C71.83 21.607 72.502 21.959 73.622 21.703V24.007C71.062 24.327 69.59 23.719 69.238 22.119C68.214 23.431 66.358 24.295 64.086 24.295ZM68.982 18.311V15.431L65.718 16.135C63.158 16.647 61.43 17.351 61.43 19.367C61.43 20.999 62.614 21.959 64.438 21.959C66.902 21.959 68.982 20.455 68.982 18.311Z" fill="currentColor"/>
				<path d="M78.4658 15.655C78.4658 19.463 80.5137 21.831 83.4897 21.831C85.7938 21.831 87.1698 20.359 87.6497 18.279L90.2098 19.463C89.4098 22.247 86.9777 24.359 83.4897 24.359C78.8177 24.359 75.5858 20.807 75.5858 15.655C75.5858 10.471 78.8177 6.951 83.4897 6.951C86.9777 6.951 89.3458 8.967 90.1458 11.751L87.6497 12.999C87.1698 10.951 85.7938 9.447 83.4897 9.447C80.5137 9.447 78.4658 11.815 78.4658 15.655Z" fill="currentColor"/>
				<path d="M100.051 24.359C95.4108 24.359 92.2108 20.807 92.2108 15.655C92.2108 10.663 95.3788 6.951 99.9228 6.951C104.595 6.951 107.059 10.503 107.059 15.143V16.103H94.9627C95.1227 19.623 97.1388 21.863 100.051 21.863C102.291 21.863 103.891 20.647 104.403 18.663L106.931 19.559C105.907 22.535 103.379 24.359 100.051 24.359ZM99.8907 9.415C97.4587 9.415 95.6348 11.047 95.1227 13.895H104.147C104.083 11.527 102.803 9.415 99.8907 9.415Z" fill="currentColor"/>
				<path d="M118.456 7.271H120.888L124.344 19.751L127.832 7.271H130.712L125.624 24.007H123.096L119.608 11.591L116.12 24.007H113.592L108.504 7.271H111.48L115 19.719L118.456 7.271Z" fill="currentColor"/>
				<path d="M137.367 24.295C134.327 24.295 131.863 22.503 131.863 19.495C131.863 16.167 134.455 14.791 138.199 14.023L142.263 13.191V12.487C142.263 10.567 141.207 9.479 138.967 9.479C136.855 9.479 135.607 10.471 135.095 12.327L132.375 11.623C133.175 8.903 135.639 6.951 139.063 6.951C142.807 6.951 145.111 8.775 145.111 12.359V20.519C145.111 21.607 145.783 21.959 146.903 21.703V24.007C144.343 24.327 142.871 23.719 142.519 22.119C141.495 23.431 139.639 24.295 137.367 24.295ZM142.263 18.311V15.431L138.999 16.135C136.439 16.647 134.711 17.351 134.711 19.367C134.711 20.999 135.895 21.959 137.719 21.959C140.183 21.959 142.263 20.455 142.263 18.311Z" fill="currentColor"/>
				<path d="M156.039 25.831C155.079 28.359 153.768 29.959 150.855 29.959C150.151 29.959 149.799 29.927 149.287 29.831V27.335C149.831 27.463 150.183 27.495 150.663 27.495C151.911 27.495 152.584 27.047 153.256 25.383L154.056 23.399L147.848 7.271H150.887L155.591 20.071L160.231 7.271H163.24L156.039 25.831Z" fill="currentColor"/>
			</svg>
		</a>

		<div class="hidden md:flex items-center gap-8">
			<a href="#features" class="text-[14px] text-text-secondary hover:text-text transition-colors">Features</a>
			<a href="#how-it-works" class="text-[14px] text-text-secondary hover:text-text transition-colors">How it works</a>
			<a href="#pricing" class="text-[14px] text-text-secondary hover:text-text transition-colors">Pricing</a>
			<a href="/blog" class="text-[14px] text-text-secondary hover:text-text transition-colors">Blog</a>
			<a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="text-[14px] text-text-secondary hover:text-text transition-colors">Docs</a>
		</div>

		<div class="flex items-center gap-3">
			<a href="https://platform.traceway.ai/login" class="hidden sm:inline text-[14px] text-text-secondary hover:text-text transition-colors">Log in</a>
			<a
				href="https://platform.traceway.ai/signup"
				class="text-[13px] font-semibold bg-accent text-white rounded-lg px-4 py-2 hover:bg-accent-dim transition-all"
			>
				Sign Up
			</a>
		</div>
	</div>
</nav>

<!-- HERO -->
<section class="relative pt-32 md:pt-44 pb-20 md:pb-32 overflow-hidden">
	<!-- Dot grid background -->
	<div class="absolute inset-0 dot-grid pointer-events-none"></div>

	<div class="relative max-w-[800px] mx-auto px-6 text-center">
		<!-- Announcement pill -->
		<InView>
			<a
				href="https://github.com/blastgits/traceway"
				target="_blank"
				rel="noopener"
				class="inline-flex items-center gap-2 rounded-full border border-border bg-white pl-1.5 pr-4 py-1 mb-10 hover:border-accent/50 transition-colors group shadow-sm"
			>
				<span class="text-[11px] font-semibold bg-accent text-white rounded-full px-2.5 py-0.5">Open Source</span>
				<span class="text-[13px] text-text-secondary">LLM observability for every team</span>
				<svg class="w-3.5 h-3.5 text-text-muted group-hover:translate-x-0.5 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" /></svg>
			</a>
		</InView>

		<!-- Headline -->
		<InView>
			<h1 class="text-[clamp(2.5rem,6vw,4.5rem)] font-bold leading-[1.08] tracking-tight text-text">
				See inside every AI call your app makes
			</h1>
		</InView>

		<!-- Subtitle -->
		<InView delay={80}>
			<p class="text-[17px] md:text-lg text-text-secondary leading-relaxed mt-6 max-w-xl mx-auto">
				Turn production traces into debugging insights. Compare prompts, models, and costs. Improve quality with every release.
			</p>
		</InView>

		<!-- CTAs -->
		<InView delay={160}>
			<div class="flex items-center justify-center gap-3 mt-10">
				<a
					href="https://platform.traceway.ai/signup"
					class="inline-flex items-center gap-2 bg-accent text-white font-semibold text-[15px] px-7 py-3 rounded-lg hover:bg-accent-dim transition-all shadow-sm"
				>
					Start building
				</a>
				<a
					href="mailto:support@traceway.ai"
					class="inline-flex items-center gap-2 text-[15px] text-text border border-border rounded-lg px-7 py-3 hover:bg-bg-secondary transition-all"
				>
					Contact sales
				</a>
			</div>
		</InView>
	</div>

	<!-- Animated scatter dots (Braintrust-style) -->
	<div class="relative max-w-[1100px] mx-auto mt-16 h-[160px] overflow-hidden hidden md:block">
		<svg class="w-full h-full" viewBox="0 0 1100 160" fill="none" xmlns="http://www.w3.org/2000/svg">
			<!-- Vertical bars pattern like Braintrust -->
			{#each Array(80) as _, i}
				{@const x = 10 + (i % 40) * 27}
				{@const row = Math.floor(i / 40)}
				{@const y = 10 + row * 80}
				{@const h = 15 + Math.random() * 50}
				{@const isAccent = Math.random() > 0.88}
				<rect
					x={x}
					y={y}
					width="3"
					height={h}
					rx="1.5"
					fill={isAccent ? '#10b981' : '#0a0a0a'}
					opacity={isAccent ? 0.6 : 0.08 + Math.random() * 0.12}
				/>
			{/each}
			<!-- Accent dots -->
			{#each Array(8) as _, i}
				{@const cx = 80 + i * 130 + Math.random() * 60}
				{@const cy = 30 + Math.random() * 100}
				<circle cx={cx} cy={cy} r="5" fill="#10b981" opacity="0.5" />
			{/each}
			<!-- Plus signs -->
			{#each Array(3) as _, i}
				{@const px = 200 + i * 350}
				{@const py = 50 + Math.random() * 60}
				<line x1={px - 6} y1={py} x2={px + 6} y2={py} stroke="#10b981" stroke-width="2" opacity="0.4" />
				<line x1={px} y1={py - 6} x2={px} y2={py + 6} stroke="#10b981" stroke-width="2" opacity="0.4" />
			{/each}
		</svg>
	</div>
</section>

<!-- ═══════════════════════════════════════════════════ -->
<!-- 01 TRACE — Braintrust-style numbered section        -->
<!-- ═══════════════════════════════════════════════════ -->
<section id="features" class="py-20 md:py-32 bg-white">
	<div class="max-w-[1200px] mx-auto px-6">
		<!-- Section header: left-aligned -->
		<InView>
			<div class="mb-4 text-[14px] text-text-muted">01 Trace</div>
			<h2 class="text-[clamp(2rem,4.5vw,3.2rem)] font-bold tracking-tight leading-[1.12] max-w-xl">
				Know exactly what your agents did.
			</h2>
			<p class="text-text-secondary text-[16px] leading-relaxed mt-4 max-w-2xl">
				Every prompt, tool call, and response &mdash; captured with rich context from real production traffic.
			</p>
		</InView>

		<!-- 3 dark cards -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-5 mt-12">
			<InView>
				<div class="dark-card">
					<div class="dark-card-visual">
						<!-- Trace waterfall illustration -->
						<svg class="w-full max-w-[340px]" viewBox="0 0 340 160" fill="none" xmlns="http://www.w3.org/2000/svg">
							<!-- Row backgrounds -->
							<rect x="20" y="10" width="300" height="28" rx="4" fill="rgba(255,255,255,0.04)"/>
							<rect x="20" y="46" width="300" height="28" rx="4" fill="rgba(255,255,255,0.04)"/>
							<rect x="20" y="82" width="300" height="28" rx="4" fill="rgba(255,255,255,0.04)"/>
							<rect x="20" y="118" width="300" height="28" rx="4" fill="rgba(255,255,255,0.04)"/>

							<!-- Span labels -->
							<text x="30" y="29" font-size="9" fill="rgba(255,255,255,0.35)" font-family="monospace">main</text>
							<text x="30" y="65" font-size="9" fill="rgba(255,255,255,0.35)" font-family="monospace">rewrite</text>
							<text x="30" y="101" font-size="9" fill="rgba(255,255,255,0.35)" font-family="monospace">search</text>
							<text x="30" y="137" font-size="9" fill="rgba(255,255,255,0.35)" font-family="monospace">generate</text>

							<!-- Span bars -->
							<rect x="85" y="16" width="220" height="16" rx="3" fill="rgba(255,255,255,0.08)"/>
							<circle cx="88" cy="24" r="3" fill="#10b981"/>

							<rect x="95" y="52" width="60" height="16" rx="3" fill="rgba(99,102,241,0.3)"/>

							<rect x="165" y="88" width="45" height="16" rx="3" fill="rgba(34,211,238,0.3)"/>

							<rect x="220" y="124" width="85" height="16" rx="3" fill="rgba(16,185,129,0.3)"/>
						</svg>
					</div>
					<h3 class="text-[16px] font-semibold text-white mb-2">End-to-end execution paths</h3>
					<p class="text-[13px] text-white/50 leading-relaxed">See every step from input to output with the context needed to debug fast. Search, filter, and sort traces by content, latency, cost, quality, tags, and custom metadata.</p>
				</div>
			</InView>

			<InView delay={80}>
				<div class="dark-card">
					<div class="dark-card-visual">
						<!-- Inspect / replay illustration -->
						<svg class="w-full max-w-[340px]" viewBox="0 0 340 160" fill="none" xmlns="http://www.w3.org/2000/svg">
							<!-- Top bar -->
							<rect x="60" y="15" width="170" height="22" rx="4" fill="rgba(255,255,255,0.06)" stroke="rgba(99,102,241,0.4)" stroke-width="1"/>
							<text x="145" y="30" text-anchor="middle" font-size="8" fill="rgba(255,255,255,0.4)" font-family="monospace">input / output</text>

							<!-- Flow line -->
							<circle cx="40" cy="80" r="6" stroke="rgba(255,255,255,0.2)" stroke-width="1.5" fill="none"/>
							<line x1="46" y1="80" x2="100" y2="80" stroke="rgba(255,255,255,0.15)" stroke-width="1.5"/>
							<rect x="100" y="68" width="60" height="24" rx="4" fill="rgba(255,255,255,0.06)"/>
							<line x1="160" y1="80" x2="195" y2="80" stroke="rgba(255,255,255,0.15)" stroke-width="1.5"/>
							<rect x="195" y="68" width="60" height="24" rx="4" fill="rgba(255,255,255,0.06)"/>
							<line x1="255" y1="80" x2="290" y2="80" stroke="rgba(255,255,255,0.15)" stroke-width="1.5"/>
							<circle cx="300" cy="80" r="6" fill="rgba(99,102,241,0.5)" stroke="rgba(99,102,241,0.6)" stroke-width="1.5"/>

							<!-- Bottom detail rows -->
							<rect x="80" y="115" width="80" height="16" rx="3" fill="rgba(255,255,255,0.04)"/>
							<rect x="170" y="115" width="80" height="16" rx="3" fill="rgba(255,255,255,0.04)"/>
						</svg>
					</div>
					<h3 class="text-[16px] font-semibold text-white mb-2">Reproduce and inspect real sessions</h3>
					<p class="text-[13px] text-white/50 leading-relaxed">Open any production trace to replay behavior, test fixes, and debug failures in full context. Inspect inputs, outputs, and metadata at every step.</p>
				</div>
			</InView>

			<InView delay={160}>
				<div class="dark-card">
					<div class="dark-card-visual">
						<!-- Dataset / pipeline illustration -->
						<svg class="w-full max-w-[340px]" viewBox="0 0 340 160" fill="none" xmlns="http://www.w3.org/2000/svg">
							<!-- Vertical pipeline -->
							<circle cx="170" cy="20" r="5" fill="#10b981"/>
							<line x1="170" y1="25" x2="170" y2="50" stroke="rgba(16,185,129,0.3)" stroke-width="1.5"/>

							<!-- Branch nodes -->
							<line x1="170" y1="50" x2="100" y2="75" stroke="rgba(255,255,255,0.12)" stroke-width="1.5"/>
							<line x1="170" y1="50" x2="170" y2="75" stroke="rgba(255,255,255,0.12)" stroke-width="1.5"/>
							<line x1="170" y1="50" x2="240" y2="75" stroke="rgba(255,255,255,0.12)" stroke-width="1.5"/>

							<!-- Node boxes -->
							<rect x="75" y="75" width="50" height="24" rx="4" fill="rgba(255,255,255,0.06)"/>
							<rect x="145" y="75" width="50" height="24" rx="4" fill="rgba(99,102,241,0.15)" stroke="rgba(99,102,241,0.4)" stroke-width="1"/>
							<rect x="215" y="75" width="50" height="24" rx="4" fill="rgba(255,255,255,0.06)"/>

							<!-- Merge down -->
							<line x1="100" y1="99" x2="170" y2="125" stroke="rgba(255,255,255,0.12)" stroke-width="1.5"/>
							<line x1="170" y1="99" x2="170" y2="125" stroke="rgba(255,255,255,0.12)" stroke-width="1.5"/>
							<line x1="240" y1="99" x2="170" y2="125" stroke="rgba(255,255,255,0.12)" stroke-width="1.5"/>

							<!-- Bottom result -->
							<rect x="140" y="125" width="60" height="22" rx="4" fill="rgba(16,185,129,0.12)" stroke="rgba(16,185,129,0.3)" stroke-width="1"/>
							<text x="170" y="140" text-anchor="middle" font-size="8" fill="rgba(16,185,129,0.6)" font-family="monospace">dataset</text>
						</svg>
					</div>
					<h3 class="text-[16px] font-semibold text-white mb-2">Turn production traces into action</h3>
					<p class="text-[13px] text-white/50 leading-relaxed">Assign runs for review or evaluation &mdash; or promote them into datasets to improve prompts, routing, and models.</p>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- ═══════════════════════════════════════════════════ -->
<!-- 02 MONITOR — left-aligned product section           -->
<!-- ═══════════════════════════════════════════════════ -->
<section id="how-it-works" class="py-20 md:py-32 border-t border-border">
	<div class="max-w-[1200px] mx-auto px-6">
		<InView>
			<div class="mb-4 text-[14px] text-text-muted">02 Monitor</div>
			<h2 class="text-[clamp(2rem,4.5vw,3.2rem)] font-bold tracking-tight leading-[1.12] max-w-xl">
				Understand cost, latency, and quality at a glance.
			</h2>
			<p class="text-text-secondary text-[16px] leading-relaxed mt-4 max-w-2xl">
				Traceway automatically captures tokens, cost, and timing for every LLM call. No manual instrumentation, no surprise bills.
			</p>
		</InView>

		<div class="grid grid-cols-1 md:grid-cols-3 gap-5 mt-12">
			<InView>
				<div class="rounded-xl border border-border bg-white p-7 h-full">
					<div class="w-10 h-10 rounded-lg bg-emerald-50 flex items-center justify-center mb-5">
						<svg class="w-5 h-5 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v12m-3-2.818.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" /></svg>
					</div>
					<h3 class="text-[17px] font-semibold text-text mb-2">Cost attribution</h3>
					<p class="text-[14px] text-text-secondary leading-relaxed">Per-model, per-span cost breakdowns. Know exactly which part of your pipeline is spending the most.</p>
				</div>
			</InView>
			<InView delay={80}>
				<div class="rounded-xl border border-border bg-white p-7 h-full">
					<div class="w-10 h-10 rounded-lg bg-blue-50 flex items-center justify-center mb-5">
						<svg class="w-5 h-5 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 3v11.25A2.25 2.25 0 0 0 6 16.5h2.25M3.75 3h-1.5m1.5 0h16.5m0 0h1.5m-1.5 0v11.25A2.25 2.25 0 0 1 18 16.5h-2.25m-7.5 0h7.5m-7.5 0-1 3m8.5-3 1 3m0 0 .5 1.5m-.5-1.5h-9.5m0 0-.5 1.5m.75-9 3-3 2.148 2.148A12.061 12.061 0 0 1 16.5 7.605" /></svg>
					</div>
					<h3 class="text-[17px] font-semibold text-text mb-2">Latency insights</h3>
					<p class="text-[14px] text-text-secondary leading-relaxed">See which spans are slow and why. P50, P95, and P99 latency breakdowns across your entire pipeline.</p>
				</div>
			</InView>
			<InView delay={160}>
				<div class="rounded-xl border border-border bg-white p-7 h-full">
					<div class="w-10 h-10 rounded-lg bg-violet-50 flex items-center justify-center mb-5">
						<svg class="w-5 h-5 text-violet-500" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09ZM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 0 0-2.455 2.456ZM16.894 20.567 16.5 21.75l-.394-1.183a2.25 2.25 0 0 0-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 0 0 1.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 0 0 1.423 1.423l1.183.394-1.183.394a2.25 2.25 0 0 0-1.423 1.423Z" /></svg>
					</div>
					<h3 class="text-[17px] font-semibold text-text mb-2">Quality signals</h3>
					<p class="text-[14px] text-text-secondary leading-relaxed">Track accuracy scores, user feedback, and custom evaluation metrics. Spot regressions before users do.</p>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- ═══════════════════════════════════════════════════ -->
<!-- 03 INTEGRATE — left-aligned, how-it-works style     -->
<!-- ═══════════════════════════════════════════════════ -->
<section class="py-20 md:py-32 border-t border-border bg-bg-secondary">
	<div class="max-w-[1200px] mx-auto px-6">
		<div class="grid grid-cols-1 md:grid-cols-2 gap-12 items-start">
			<InView>
				<div class="mb-4 text-[14px] text-text-muted">03 Integrate</div>
				<h2 class="text-[clamp(2rem,4.5vw,3.2rem)] font-bold tracking-tight leading-[1.12]">
					Works with your stack. No changes needed.
				</h2>
			</InView>
			<InView delay={80}>
				<p class="text-text-secondary text-[16px] leading-relaxed md:pt-12">
					Traceway sits between your app and your LLM provider as a transparent proxy. <span class="font-semibold text-text">Point your base URL at Traceway</span> and every call is traced automatically. Works with OpenAI, Anthropic, Ollama, and any OpenAI-compatible API.
				</p>
			</InView>
		</div>

		<!-- Integration cards -->
		<div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-4 mt-12">
			{#each [
				{ name: 'OpenAI', desc: 'GPT-4o, o1, and all models' },
				{ name: 'Anthropic', desc: 'Claude 3.5 Sonnet and family' },
				{ name: 'Ollama', desc: 'Local models, zero config' },
				{ name: 'Any provider', desc: 'OpenAI-compatible APIs' },
			] as item, i}
				<InView delay={i * 60}>
					<div class="rounded-xl border border-border bg-white p-6 text-center h-full hover:shadow-sm transition-all">
						<div class="text-[15px] font-semibold text-text mb-1">{item.name}</div>
						<p class="text-[13px] text-text-muted">{item.desc}</p>
					</div>
				</InView>
			{/each}
		</div>
	</div>
</section>

<!-- PRICING -->
<section id="pricing" class="py-20 md:py-32">
	<div class="max-w-[1000px] mx-auto px-6">
		<InView>
			<div class="text-center mb-14">
				<h2 class="text-[clamp(2rem,4.5vw,3rem)] font-bold tracking-tight text-text">
					Start free. Scale when ready.
				</h2>
				<p class="text-text-secondary mt-4 max-w-md mx-auto">
					Self-host for free forever, or let us handle the infrastructure.
				</p>
			</div>
		</InView>

		<div class="grid md:grid-cols-3 gap-4 max-w-3xl mx-auto">
			{#each [
				{ name: 'Free', price: '$0', period: '/mo', pop: false, desc: 'Local dev and getting started.', features: ['10K spans/month', '7-day retention', '1 team member', 'Community support'], cta: 'Get started' },
				{ name: 'Pro', price: '$20', period: '/user/mo', pop: true, desc: 'Teams shipping AI to production.', features: ['1M spans/month', '30-day retention', '5 team members', 'Email support'], cta: 'Start free trial' },
				{ name: 'Team', price: '$100', period: '/user/mo', pop: false, desc: 'Full observability at scale.', features: ['10M spans/month', '90-day retention', '50 team members', 'Priority support'], cta: 'Get started' },
			] as plan, i}
				<InView delay={i * 60}>
					<div class="relative {plan.pop ? 'bg-white border-2 border-accent shadow-lg' : 'bg-white border border-border'} rounded-2xl p-6 flex flex-col h-full transition-all duration-300 hover:translate-y-[-2px] hover:shadow-md">
						{#if plan.pop}
							<div class="absolute -top-2.5 left-1/2 -translate-x-1/2">
								<span class="bg-accent text-white text-[9px] font-bold uppercase tracking-wider px-3 py-0.5 rounded-full">Popular</span>
							</div>
						{/if}
						<div class="mb-5">
							<div class="text-[11px] {plan.pop ? 'text-accent' : 'text-text-muted'} uppercase tracking-wider mb-2 font-mono font-bold">{plan.name}</div>
							<div class="flex items-baseline gap-1">
								<span class="text-3xl font-bold text-text">{plan.price}</span>
								<span class="text-sm text-text-muted">{plan.period}</span>
							</div>
							<p class="text-[13px] text-text-secondary mt-1.5">{plan.desc}</p>
						</div>
						<ul class="space-y-2.5 text-[13px] flex-1 mb-6">
							{#each plan.features as f}
								<li class="flex items-center gap-2 text-text-secondary">
									<svg class="w-3.5 h-3.5 shrink-0 {plan.pop ? 'text-accent' : 'text-text-muted'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
									{f}
								</li>
							{/each}
						</ul>
						<a
							href="https://platform.traceway.ai/signup"
							class="block text-center text-[14px] rounded-lg px-4 py-2.5 transition-all {plan.pop ? 'font-semibold bg-accent text-white hover:bg-accent-dim' : 'text-text border border-border hover:bg-bg-secondary'}"
						>
							{plan.cta}
						</a>
					</div>
				</InView>
			{/each}
		</div>

		<!-- Enterprise + Self-hosted -->
		<div class="grid md:grid-cols-2 gap-4 max-w-3xl mx-auto mt-4">
			{#each [
				{ name: 'Enterprise', desc: 'Unlimited everything. SSO, SAML, dedicated support.', cta: 'Contact', href: 'mailto:support@traceway.ai' },
				{ name: 'Self-hosted', desc: 'Free forever. Unlimited traces. No account needed.', cta: 'GitHub', href: 'https://github.com/blastgits/traceway' },
			] as row}
				<InView delay={80}>
					<div class="bg-white border border-border rounded-2xl px-6 py-4 flex items-center justify-between gap-4 hover:shadow-sm transition-all">
						<div>
							<div class="text-[14px] font-semibold text-text">{row.name}</div>
							<p class="text-[13px] text-text-secondary mt-0.5">{row.desc}</p>
						</div>
						<a href={row.href} class="shrink-0 text-[13px] text-text border border-border rounded-lg px-4 py-1.5 hover:bg-bg-secondary transition-all">{row.cta}</a>
					</div>
				</InView>
			{/each}
		</div>

		<p class="text-center text-[12px] text-text-muted mt-6">
			Billed monthly via <a href="https://polar.sh" target="_blank" rel="noopener" class="text-accent hover:text-accent-dim transition-colors">Polar</a>. Cancel anytime.
		</p>
	</div>
</section>

<!-- CTA SECTION (Browserbase-style: "What will you build?") -->
<section class="py-16 md:py-24 border-t border-border grid-bg">
	<div class="max-w-[1200px] mx-auto px-6">
		<div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-8">
			<InView>
				<h2 class="text-[clamp(2rem,4.5vw,3.5rem)] font-bold tracking-tight leading-tight">
					What will you build?
				</h2>
			</InView>
			<InView delay={80}>
				<div class="flex flex-col sm:flex-row gap-3">
					<a
						href="mailto:support@traceway.ai"
						class="inline-flex items-center justify-center text-[15px] text-text border-2 border-text rounded-lg px-8 py-3 hover:bg-text hover:text-white transition-all font-medium"
					>
						Get a Demo
					</a>
					<a
						href="https://platform.traceway.ai/signup"
						class="inline-flex items-center justify-center text-[15px] text-white bg-accent rounded-lg px-8 py-3 hover:bg-accent-dim transition-all font-semibold"
					>
						Get Started
					</a>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- FOOTER (Browserbase-style: brand color background) -->
<footer class="bg-[#0f172a] text-white">
	<!-- Links section -->
	<div class="max-w-[1200px] mx-auto px-6 py-12">
		<div class="grid grid-cols-2 md:grid-cols-4 gap-8">
			<!-- Product -->
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Product</h4>
				<ul class="space-y-2.5">
					<li><a href="#features" class="text-[13px] text-white/60 hover:text-white transition-colors">Features</a></li>
					<li><a href="#pricing" class="text-[13px] text-white/60 hover:text-white transition-colors">Pricing</a></li>
					<li><a href="#how-it-works" class="text-[13px] text-white/60 hover:text-white transition-colors">How it works</a></li>
					<li><a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">Docs</a></li>
				</ul>
			</div>

			<!-- Company -->
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Company</h4>
				<ul class="space-y-2.5">
					<li><a href="/blog" class="text-[13px] text-white/60 hover:text-white transition-colors">Blog</a></li>
					<li><a href="mailto:support@traceway.ai" class="text-[13px] text-white/60 hover:text-white transition-colors">Contact</a></li>
					<li><a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">About</a></li>
				</ul>
			</div>

			<!-- Developers -->
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Developers</h4>
				<ul class="space-y-2.5">
					<li><a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">GitHub</a></li>
					<li><a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">API Docs</a></li>
					<li><a href="https://github.com/blastgits/traceway/blob/main/CHANGELOG.md" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">Changelog</a></li>
				</ul>
			</div>

			<!-- Legal -->
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Legal</h4>
				<ul class="space-y-2.5">
					<li><a href="/privacy" class="text-[13px] text-white/60 hover:text-white transition-colors">Privacy Policy</a></li>
					<li><a href="/terms" class="text-[13px] text-white/60 hover:text-white transition-colors">Terms of Service</a></li>
				</ul>
			</div>
		</div>

		<!-- Divider + bottom row -->
		<div class="mt-10 pt-6 border-t border-white/10 flex flex-col md:flex-row items-center justify-between gap-4">
			<div class="flex items-center gap-4">
				<span class="text-[12px] text-white/40">MIT Licensed</span>
				<span class="text-[12px] text-white/40">Open source LLM observability</span>
			</div>
			<div class="flex items-center gap-4">
				<!-- GitHub -->
				<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-white/40 hover:text-white transition-colors" aria-label="GitHub">
					<svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
				</a>
				<!-- X/Twitter -->
				<a href="https://x.com/traceway" target="_blank" rel="noopener" class="text-white/40 hover:text-white transition-colors" aria-label="X (Twitter)">
					<svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24"><path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/></svg>
				</a>
			</div>
		</div>
	</div>

	<!-- Large brand name -->
	<div class="border-t border-white/5 py-8 overflow-hidden">
		<div class="max-w-[1200px] mx-auto px-6">
			<svg class="w-full max-w-[600px] opacity-20" viewBox="0 0 164 30" fill="none" xmlns="http://www.w3.org/2000/svg">
				<path d="M27.375 27.375H18.0938V24.2814H24.2814V18.0938H27.375V27.375Z" fill="white"/>
				<path fill-rule="evenodd" clip-rule="evenodd" d="M11.7656 0.375C18.0565 0.375 23.1562 5.47476 23.1562 11.7656C23.1562 18.0565 18.0565 23.1562 11.7656 23.1562C5.47476 23.1562 0.375 18.0565 0.375 11.7656C0.375 5.47476 5.47476 0.375 11.7656 0.375ZM11.8711 3.75C7.38594 3.75 3.75 7.38594 3.75 11.8711C3.75 16.3562 7.38594 19.9922 11.8711 19.9922C16.3562 19.9922 19.9922 16.3562 19.9922 11.8711C19.9922 7.38594 16.3562 3.75 11.8711 3.75Z" fill="white"/>
				<path d="M37.967 20.007V9.735H35.375V7.271H37.967V2.375H40.815V7.271H44.623V9.735H40.815V19.559C40.815 21.319 41.679 21.639 43.151 21.639C43.823 21.639 44.303 21.575 44.975 21.447V23.911C44.239 24.071 43.407 24.167 42.447 24.167C39.599 24.167 37.967 23.207 37.967 20.007Z" fill="white"/>
				<path d="M57.066 7.207V10.119C56.586 10.023 56.202 9.991 55.658 9.991C53.226 9.991 51.274 11.911 51.274 14.855V24.007H48.394V7.271H51.274V10.119C52.01 8.423 53.642 7.143 55.914 7.143C56.362 7.143 56.778 7.175 57.066 7.207Z" fill="white"/>
				<path d="M64.086 24.295C61.046 24.295 58.582 22.503 58.582 19.495C58.582 16.167 61.174 14.791 64.918 14.023L68.982 13.191V12.487C68.982 10.567 67.926 9.479 65.686 9.479C63.574 9.479 62.326 10.471 61.814 12.327L59.094 11.623C59.894 8.903 62.358 6.951 65.782 6.951C69.526 6.951 71.83 8.775 71.83 12.359V20.519C71.83 21.607 72.502 21.959 73.622 21.703V24.007C71.062 24.327 69.59 23.719 69.238 22.119C68.214 23.431 66.358 24.295 64.086 24.295ZM68.982 18.311V15.431L65.718 16.135C63.158 16.647 61.43 17.351 61.43 19.367C61.43 20.999 62.614 21.959 64.438 21.959C66.902 21.959 68.982 20.455 68.982 18.311Z" fill="white"/>
				<path d="M78.4658 15.655C78.4658 19.463 80.5137 21.831 83.4897 21.831C85.7938 21.831 87.1698 20.359 87.6497 18.279L90.2098 19.463C89.4098 22.247 86.9777 24.359 83.4897 24.359C78.8177 24.359 75.5858 20.807 75.5858 15.655C75.5858 10.471 78.8177 6.951 83.4897 6.951C86.9777 6.951 89.3458 8.967 90.1458 11.751L87.6497 12.999C87.1698 10.951 85.7938 9.447 83.4897 9.447C80.5137 9.447 78.4658 11.815 78.4658 15.655Z" fill="white"/>
				<path d="M100.051 24.359C95.4108 24.359 92.2108 20.807 92.2108 15.655C92.2108 10.663 95.3788 6.951 99.9228 6.951C104.595 6.951 107.059 10.503 107.059 15.143V16.103H94.9627C95.1227 19.623 97.1388 21.863 100.051 21.863C102.291 21.863 103.891 20.647 104.403 18.663L106.931 19.559C105.907 22.535 103.379 24.359 100.051 24.359ZM99.8907 9.415C97.4587 9.415 95.6348 11.047 95.1227 13.895H104.147C104.083 11.527 102.803 9.415 99.8907 9.415Z" fill="white"/>
				<path d="M118.456 7.271H120.888L124.344 19.751L127.832 7.271H130.712L125.624 24.007H123.096L119.608 11.591L116.12 24.007H113.592L108.504 7.271H111.48L115 19.719L118.456 7.271Z" fill="white"/>
				<path d="M137.367 24.295C134.327 24.295 131.863 22.503 131.863 19.495C131.863 16.167 134.455 14.791 138.199 14.023L142.263 13.191V12.487C142.263 10.567 141.207 9.479 138.967 9.479C136.855 9.479 135.607 10.471 135.095 12.327L132.375 11.623C133.175 8.903 135.639 6.951 139.063 6.951C142.807 6.951 145.111 8.775 145.111 12.359V20.519C145.111 21.607 145.783 21.959 146.903 21.703V24.007C144.343 24.327 142.871 23.719 142.519 22.119C141.495 23.431 139.639 24.295 137.367 24.295ZM142.263 18.311V15.431L138.999 16.135C136.439 16.647 134.711 17.351 134.711 19.367C134.711 20.999 135.895 21.959 137.719 21.959C140.183 21.959 142.263 20.455 142.263 18.311Z" fill="white"/>
				<path d="M156.039 25.831C155.079 28.359 153.768 29.959 150.855 29.959C150.151 29.959 149.799 29.927 149.287 29.831V27.335C149.831 27.463 150.183 27.495 150.663 27.495C151.911 27.495 152.584 27.047 153.256 25.383L154.056 23.399L147.848 7.271H150.887L155.591 20.071L160.231 7.271H163.24L156.039 25.831Z" fill="white"/>
			</svg>
		</div>
	</div>
</footer>
