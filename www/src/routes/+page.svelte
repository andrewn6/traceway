<script lang="ts">
	import InView from '$lib/InView.svelte';

	let scrollY = $state(0);

	// Sticky scroll features
	const features = [
		{
			id: 'traces',
			title: 'Trace explorer',
			desc: 'Browse every LLM call in a split-pane explorer. Filter by time, model, cost, or content. Click any trace to see a full waterfall timeline with inputs, outputs, and metadata.',
			mockType: 'traces'
		},
		{
			id: 'analytics',
			title: 'Analytics',
			desc: 'Configurable dashboard with time-series charts, per-model cost breakdowns, token usage trends, and P50/P95/P99 latency percentiles.',
			mockType: 'analytics'
		},
		{
			id: 'evaluations',
			title: 'Evaluations',
			desc: 'Build datasets from production traces. Run evals with any model. Compare runs side-by-side to catch regressions before they reach users.',
			mockType: 'evaluations'
		},
		{
			id: 'search',
			title: 'Search',
			desc: 'Query language with autocomplete. Filter by model, cost, latency, status, or content. Scatter plots for performance analysis.',
			mockType: 'search'
		},
		{
			id: 'review',
			title: 'Human review',
			desc: 'Annotation queue with claim, review, and approve workflows. Review LLM outputs with full trace context and keyboard shortcuts.',
			mockType: 'review'
		},
		{
			id: 'datasets',
			title: 'Datasets',
			desc: 'Create evaluation datasets from production spans. Set capture rules to automatically collect data matching your criteria. Import JSON/JSONL.',
			mockType: 'datasets'
		},
		{
			id: 'sessions',
			title: 'Sessions',
			desc: 'Group traces by session ID for multi-turn conversation tracking. See aggregate tokens, cost, and span counts per session.',
			mockType: 'sessions'
		},
		{
			id: 'integrations',
			title: 'Integrations',
			desc: 'Transparent proxy for OpenAI, Anthropic, Ollama, and any OpenAI-compatible API. Zero code changes. Connect provider keys for evals.',
			mockType: 'integrations'
		}
	];

	let activeFeature = $state(0);

	// Track which feature items are in the viewport center
	let featureEls: HTMLElement[] = $state([]);

	$effect(() => {
		if (featureEls.length === 0) return;
		const handleScroll = () => {
			const viewportCenter = window.innerHeight * 0.45;
			let closest = 0;
			let closestDist = Infinity;
			featureEls.forEach((el, i) => {
				if (!el) return;
				const rect = el.getBoundingClientRect();
				const elCenter = rect.top + rect.height / 2;
				const dist = Math.abs(elCenter - viewportCenter);
				if (dist < closestDist) {
					closestDist = dist;
					closest = i;
				}
			});
			activeFeature = closest;
		};
		window.addEventListener('scroll', handleScroll, { passive: true });
		handleScroll();
		return () => window.removeEventListener('scroll', handleScroll);
	});
</script>

<svelte:window bind:scrollY />

<style>
	.dot-grid {
		background-image: radial-gradient(circle, #0a0a0a 1px, transparent 1px);
		background-size: 24px 24px;
		opacity: 0.04;
	}
	.grid-bg {
		background-image:
			linear-gradient(rgba(0, 0, 0, 0.03) 1px, transparent 1px),
			linear-gradient(90deg, rgba(0, 0, 0, 0.03) 1px, transparent 1px);
		background-size: 60px 60px;
	}

	/* Mock UI base */
	.mock {
		background: #fff;
		border: 1px solid #e5e5e5;
		border-radius: 6px;
		font-size: 11px;
		color: #525252;
	}
	.mock-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 7px 12px;
		border-bottom: 1px solid #f0f0f0;
	}
	.mock-row:last-child { border-bottom: none; }
	.mock-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}
	.mock-chip {
		font-size: 9px;
		padding: 1px 6px;
		border-radius: 3px;
		font-weight: 500;
		white-space: nowrap;
	}
	.mock-header {
		padding: 8px 12px;
		border-bottom: 1px solid #e5e5e5;
		font-size: 10px;
		font-weight: 600;
		color: #0a0a0a;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	/* Feature item in the right column */
	.feature-item {
		padding: 24px 0;
		border-bottom: 1px solid #e5e5e5;
		cursor: pointer;
		transition: opacity 0.3s ease;
	}
	.feature-item.is-active {
		opacity: 1;
	}
	.feature-item:not(.is-active) {
		opacity: 0.35;
	}

	/* Visual panel transitions */
	.visual-panel {
		transition: opacity 0.4s ease, transform 0.4s ease;
	}
	.visual-panel.is-visible {
		opacity: 1;
		transform: translateY(0);
	}
	.visual-panel:not(.is-visible) {
		opacity: 0;
		transform: translateY(8px);
		position: absolute;
		pointer-events: none;
	}
</style>

<!-- NAV -->
<nav class="fixed top-0 left-0 right-0 z-50 bg-white/80 backdrop-blur-xl border-b border-[#e5e5e5]">
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
			<a href="#pricing" class="text-[14px] text-text-secondary hover:text-text transition-colors">Pricing</a>
			<a href="/blog" class="text-[14px] text-text-secondary hover:text-text transition-colors">Blog</a>
			<a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="text-[14px] text-text-secondary hover:text-text transition-colors">Docs</a>
		</div>
		<div class="flex items-center gap-3">
			<a href="https://platform.traceway.ai/login" class="hidden sm:inline text-[14px] text-text-secondary hover:text-text transition-colors">Log in</a>
			<a href="https://platform.traceway.ai/signup" class="text-[13px] font-semibold bg-text text-white rounded-md px-4 py-2 hover:bg-text/85 transition-all">Sign Up</a>
		</div>
	</div>
</nav>

<!-- HERO — left-aligned with node graph on right -->
<section class="relative pt-36 md:pt-52 pb-28 md:pb-40 overflow-hidden">
	<div class="absolute inset-0 dot-grid pointer-events-none"></div>
	<div class="relative max-w-[1200px] mx-auto px-6">
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-12 lg:gap-16 items-center">
			<!-- Left: text content -->
			<div>
				<InView>
					<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="inline-flex items-center gap-2 rounded-full border border-border bg-white pl-1.5 pr-4 py-1 mb-8 hover:border-text/20 transition-colors group shadow-sm">
						<span class="text-[11px] font-semibold bg-text text-white rounded-full px-2.5 py-0.5">Open Source</span>
						<span class="text-[13px] text-text-secondary">LLM observability for every team</span>
						<svg class="w-3.5 h-3.5 text-text-muted group-hover:translate-x-0.5 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" /></svg>
					</a>
				</InView>
				<InView>
					<h1 class="text-[clamp(2.5rem,6vw,4.5rem)] font-bold leading-[1.08] tracking-tight text-text">
						See inside every AI call your app makes
					</h1>
				</InView>
				<InView delay={80}>
					<p class="text-[17px] md:text-lg text-text-secondary leading-relaxed mt-6 max-w-xl">
						Turn production traces into debugging insights. Compare prompts, models, and costs. Improve quality with every release.
					</p>
				</InView>
				<InView delay={160}>
					<div class="flex items-center gap-3 mt-10">
						<a href="https://platform.traceway.ai/signup" class="inline-flex items-center gap-2 bg-text text-white font-semibold text-[15px] px-7 py-3 rounded-md hover:bg-text/85 transition-all">Start building</a>
						<a href="mailto:support@traceway.ai" class="inline-flex items-center gap-2 text-[15px] text-text border border-border rounded-md px-7 py-3 hover:bg-bg-secondary transition-all">Contact sales</a>
					</div>
				</InView>
			</div>

			<!-- Right: Abstract node graph — large, colorful -->
			<div class="hidden lg:flex items-center justify-center lg:-mr-8 xl:-mr-16">
				<InView delay={200}>
					<div class="relative w-[560px] h-[480px]">
						<!-- Glow effects behind key nodes -->
						<div class="absolute w-[140px] h-[140px] rounded-full bg-emerald-400/10 blur-3xl" style="top:120px;left:200px"></div>
						<div class="absolute w-[120px] h-[120px] rounded-full bg-violet-400/10 blur-3xl" style="top:280px;left:210px"></div>

						<svg viewBox="0 0 560 480" fill="none" xmlns="http://www.w3.org/2000/svg" class="w-full h-full">
							<defs>
								<!-- Gradient for connection lines -->
								<linearGradient id="lineGradL" x1="0%" y1="0%" x2="100%" y2="0%">
									<stop offset="0%" stop-color="#a3a3a3" stop-opacity="0.3"/>
									<stop offset="100%" stop-color="#10b981" stop-opacity="0.5"/>
								</linearGradient>
								<linearGradient id="lineGradR" x1="0%" y1="0%" x2="100%" y2="0%">
									<stop offset="0%" stop-color="#10b981" stop-opacity="0.5"/>
									<stop offset="100%" stop-color="#6366f1" stop-opacity="0.4"/>
								</linearGradient>
								<!-- Glow filter -->
								<filter id="glow">
									<feGaussianBlur stdDeviation="3" result="coloredBlur"/>
									<feMerge><feMergeNode in="coloredBlur"/><feMergeNode in="SourceGraphic"/></feMerge>
								</filter>
							</defs>

							<!-- Connection lines — thicker, colored -->
							<path d="M170 95 Q250 130 268 170" stroke="url(#lineGradL)" stroke-width="2" fill="none"/>
							<path d="M170 240 Q250 220 268 195" stroke="url(#lineGradL)" stroke-width="2" fill="none"/>
							<path d="M170 385 Q250 370 268 340" stroke="url(#lineGradL)" stroke-width="2" fill="none"/>
							<path d="M295 170 Q330 130 390 95" stroke="url(#lineGradR)" stroke-width="2" fill="none"/>
							<path d="M295 195 Q340 230 390 240" stroke="url(#lineGradR)" stroke-width="2" fill="none"/>
							<path d="M295 320 Q330 280 390 240" stroke="url(#lineGradR)" stroke-width="2" fill="none"/>
							<path d="M295 340 Q340 370 390 385" stroke="url(#lineGradR)" stroke-width="2" fill="none"/>

							<!-- Animated signal dots — colored and glowing -->
							<circle r="4" fill="#10b981" filter="url(#glow)">
								<animateMotion dur="2.5s" repeatCount="indefinite" begin="0s" path="M170 95 Q250 130 268 170 Q290 150 295 170 Q330 130 390 95" />
							</circle>
							<circle r="4" fill="#6366f1" filter="url(#glow)">
								<animateMotion dur="3s" repeatCount="indefinite" begin="0.8s" path="M170 240 Q250 220 268 195 Q290 210 295 195 Q340 230 390 240" />
							</circle>
							<circle r="4" fill="#10b981" filter="url(#glow)">
								<animateMotion dur="3.5s" repeatCount="indefinite" begin="1.2s" path="M170 385 Q250 370 268 340 Q290 350 295 340 Q340 370 390 385" />
							</circle>
							<circle r="3" fill="#f59e0b" filter="url(#glow)" opacity="0.8">
								<animateMotion dur="2.8s" repeatCount="indefinite" begin="1.8s" path="M170 240 Q250 220 268 195 Q290 150 295 170 Q330 130 390 95" />
							</circle>
							<circle r="3" fill="#6366f1" filter="url(#glow)" opacity="0.8">
								<animateMotion dur="3.2s" repeatCount="indefinite" begin="2.2s" path="M170 385 Q250 370 268 340 Q290 310 295 320 Q330 280 390 240" />
							</circle>

							<!-- Left column: Source nodes — larger, with colored left border -->
							<g>
								<rect x="16" y="62" width="200" height="66" rx="10" fill="white" stroke="#e5e5e5" stroke-width="1"/>
								<rect x="16" y="62" width="4" height="66" rx="2" fill="#0a0a0a"/>
								<circle cx="40" cy="86" r="5" fill="#0a0a0a"/>
								<text x="56" y="83" font-size="14" font-weight="700" fill="#0a0a0a" font-family="Inter, sans-serif">chat-api</text>
								<text x="56" y="102" font-size="11" fill="#a3a3a3" font-family="Inter, sans-serif">POST /v1/chat/completions</text>
								<text x="180" y="83" font-size="10" fill="#10b981" font-weight="600" font-family="Inter, sans-serif" text-anchor="end">2.3s</text>
							</g>
							<g>
								<rect x="16" y="207" width="200" height="66" rx="10" fill="white" stroke="#e5e5e5" stroke-width="1"/>
								<rect x="16" y="207" width="4" height="66" rx="2" fill="#6366f1"/>
								<circle cx="40" cy="231" r="5" fill="#6366f1"/>
								<text x="56" y="228" font-size="14" font-weight="700" fill="#0a0a0a" font-family="Inter, sans-serif">review-agent</text>
								<text x="56" y="248" font-size="11" fill="#a3a3a3" font-family="Inter, sans-serif">7 spans &middot; 4.1s</text>
								<text x="180" y="228" font-size="10" fill="#6366f1" font-weight="600" font-family="Inter, sans-serif" text-anchor="end">$0.09</text>
							</g>
							<g>
								<rect x="16" y="352" width="200" height="66" rx="10" fill="white" stroke="#e5e5e5" stroke-width="1"/>
								<rect x="16" y="352" width="4" height="66" rx="2" fill="#f59e0b"/>
								<circle cx="40" cy="376" r="5" fill="#f59e0b"/>
								<text x="56" y="373" font-size="14" font-weight="700" fill="#0a0a0a" font-family="Inter, sans-serif">data-pipeline</text>
								<text x="56" y="393" font-size="11" fill="#a3a3a3" font-family="Inter, sans-serif">5 spans &middot; 3.2s</text>
								<text x="180" y="373" font-size="10" fill="#f59e0b" font-weight="600" font-family="Inter, sans-serif" text-anchor="end">$0.06</text>
							</g>

							<!-- Center column: Traceway hub — prominent -->
							<g>
								<rect x="234" y="150" width="96" height="72" rx="12" fill="#0a0a0a"/>
								<rect x="234" y="150" width="96" height="72" rx="12" stroke="#10b981" stroke-width="1" stroke-opacity="0.3"/>
								<text x="282" y="183" font-size="11" font-weight="800" fill="white" text-anchor="middle" font-family="Inter, sans-serif" letter-spacing="0.08em">TRACEWAY</text>
								<text x="282" y="201" font-size="9" fill="#10b981" text-anchor="middle" font-family="Inter, sans-serif">observe</text>
							</g>
							<g>
								<rect x="234" y="300" width="96" height="72" rx="12" fill="#0a0a0a"/>
								<rect x="234" y="300" width="96" height="72" rx="12" stroke="#10b981" stroke-width="1" stroke-opacity="0.3"/>
								<text x="282" y="333" font-size="11" font-weight="800" fill="white" text-anchor="middle" font-family="Inter, sans-serif" letter-spacing="0.08em">TRACEWAY</text>
								<text x="282" y="351" font-size="9" fill="#10b981" text-anchor="middle" font-family="Inter, sans-serif">observe</text>
							</g>

							<!-- Right column: Model nodes — with colored accents -->
							<g>
								<rect x="356" y="62" width="188" height="66" rx="10" fill="white" stroke="#e5e5e5" stroke-width="1"/>
								<circle cx="378" cy="86" r="5" fill="#10b981"/>
								<text x="394" y="83" font-size="14" font-weight="700" fill="#0a0a0a" font-family="Inter, sans-serif">gpt-4o</text>
								<text x="394" y="102" font-size="11" fill="#a3a3a3" font-family="Inter, sans-serif">340ms &middot; $0.043</text>
								<rect x="490" y="76" width="40" height="18" rx="4" fill="#dcfce7"/>
								<text x="510" y="89" font-size="9" font-weight="600" fill="#16a34a" text-anchor="middle" font-family="Inter, sans-serif">200</text>
							</g>
							<g>
								<rect x="356" y="207" width="188" height="66" rx="10" fill="white" stroke="#e5e5e5" stroke-width="1"/>
								<circle cx="378" cy="231" r="5" fill="#6366f1"/>
								<text x="394" y="228" font-size="14" font-weight="700" fill="#0a0a0a" font-family="Inter, sans-serif">claude-3.5</text>
								<text x="394" y="248" font-size="11" fill="#a3a3a3" font-family="Inter, sans-serif">520ms &middot; $0.082</text>
								<rect x="490" y="221" width="40" height="18" rx="4" fill="#dcfce7"/>
								<text x="510" y="234" font-size="9" font-weight="600" fill="#16a34a" text-anchor="middle" font-family="Inter, sans-serif">200</text>
							</g>
							<g>
								<rect x="356" y="352" width="188" height="66" rx="10" fill="white" stroke="#e5e5e5" stroke-width="1"/>
								<circle cx="378" cy="376" r="5" fill="#f59e0b"/>
								<text x="394" y="373" font-size="14" font-weight="700" fill="#0a0a0a" font-family="Inter, sans-serif">llama-3.1-70b</text>
								<text x="394" y="393" font-size="11" fill="#a3a3a3" font-family="Inter, sans-serif">180ms &middot; $0.012</text>
								<rect x="490" y="366" width="40" height="18" rx="4" fill="#dcfce7"/>
								<text x="510" y="379" font-size="9" font-weight="600" fill="#16a34a" text-anchor="middle" font-family="Inter, sans-serif">200</text>
							</g>

							<!-- Column labels — larger -->
							<text x="116" y="44" font-size="11" fill="#a3a3a3" text-anchor="middle" font-family="Inter, sans-serif" font-weight="600" letter-spacing="0.08em">YOUR APP</text>
							<text x="282" y="138" font-size="11" fill="#a3a3a3" text-anchor="middle" font-family="Inter, sans-serif" font-weight="600" letter-spacing="0.08em">PROXY</text>
							<text x="450" y="44" font-size="11" fill="#a3a3a3" text-anchor="middle" font-family="Inter, sans-serif" font-weight="600" letter-spacing="0.08em">MODELS</text>
						</svg>
					</div>
				</InView>
			</div>
		</div>
	</div>
</section>

<!-- PRODUCT SHOWCASE — Stripe-style: text row + full-width screenshot -->
<section class="py-24 md:py-36 border-t border-border">
	<div class="max-w-[1200px] mx-auto px-6">
		<!-- Top row: heading left, description right -->
		<div class="grid grid-cols-1 md:grid-cols-2 gap-6 md:gap-16 mb-16">
			<InView>
				<div>
					<h2 class="text-[clamp(1.75rem,4vw,2.75rem)] font-bold tracking-tight text-text leading-tight">
						Observe every LLM call across your entire stack
					</h2>
					<a href="https://platform.traceway.ai/signup" class="inline-flex items-center gap-2 bg-text text-white font-semibold text-[15px] px-7 py-3 rounded-md hover:bg-text/85 transition-all mt-8">
						Try Traceway
						<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" /></svg>
					</a>
				</div>
			</InView>
			<InView delay={80}>
				<p class="text-[16px] text-text-secondary leading-relaxed md:pt-2">
					From the first API call to production at scale, Traceway gives you full visibility into cost, latency, and quality across every model and provider you use.
				</p>
			</InView>
		</div>
		<!-- Large product UI mock in browser frame -->
		<InView delay={120}>
			<div class="bg-[#f7f7f8] border border-[#e5e5e5] rounded-xl overflow-hidden shadow-2xl">
				<!-- Browser bar -->
				<div class="flex items-center gap-2 px-4 py-2.5 border-b border-[#e5e5e5] bg-[#fafafa]">
					<div class="flex items-center gap-1.5">
						<div class="w-2.5 h-2.5 rounded-full bg-[#e5e5e5]"></div>
						<div class="w-2.5 h-2.5 rounded-full bg-[#e5e5e5]"></div>
						<div class="w-2.5 h-2.5 rounded-full bg-[#e5e5e5]"></div>
					</div>
					<div class="flex-1 flex justify-center">
						<div class="bg-white border border-[#e5e5e5] rounded-md px-4 py-1 text-[11px] text-text-muted font-mono max-w-[300px] w-full text-center">
							platform.traceway.ai
						</div>
					</div>
				</div>
				<!-- App mock: sidebar + trace table -->
				<div class="flex bg-white" style="min-height:420px">
					<!-- Sidebar -->
					<div class="hidden md:flex flex-col w-[180px] border-r border-[#e5e5e5] bg-[#fafafa] p-3 pt-4 shrink-0">
						<!-- Logo -->
						<div class="flex items-center gap-1.5 px-2 mb-5">
							<svg class="w-4 h-4 shrink-0" viewBox="0 0 28 28" fill="none" xmlns="http://www.w3.org/2000/svg">
								<path d="M27.375 27.375H18.0938V24.2814H24.2814V18.0938H27.375V27.375Z" fill="#0a0a0a"/>
								<path fill-rule="evenodd" clip-rule="evenodd" d="M11.7656 0.375C18.0565 0.375 23.1562 5.47476 23.1562 11.7656C23.1562 18.0565 18.0565 23.1562 11.7656 23.1562C5.47476 23.1562 0.375 18.0565 0.375 11.7656C0.375 5.47476 5.47476 0.375 11.7656 0.375ZM11.8711 3.75C7.38594 3.75 3.75 7.38594 3.75 11.8711C3.75 16.3562 7.38594 19.9922 11.8711 19.9922C16.3562 19.9922 19.9922 16.3562 19.9922 11.8711C19.9922 7.38594 16.3562 3.75 11.8711 3.75Z" fill="#0a0a0a"/>
							</svg>
							<span class="text-[13px] font-semibold text-text">traceway</span>
						</div>
						<!-- Nav items -->
						{#each [
							{ name: 'Dashboard', active: false },
							{ name: 'Traces', active: true },
							{ name: 'Sessions', active: false },
							{ name: 'Review', active: false },
							{ name: 'Approvals', active: false },
							{ name: 'Analytics', active: false },
							{ name: 'Search', active: false },
							{ name: 'Datasets', active: false },
						] as nav}
							<div class="px-2 py-1.5 rounded-md text-[11px] {nav.active ? 'bg-white font-semibold text-text shadow-sm border border-[#e5e5e5]' : 'text-text-secondary'}">
								{nav.name}
							</div>
						{/each}
						<div class="mt-auto pt-4 border-t border-[#e5e5e5] mt-4">
							{#each ['General', 'Providers', 'API Keys', 'Team', 'Billing'] as s}
								<div class="px-2 py-1 text-[10px] text-text-muted">{s}</div>
							{/each}
						</div>
					</div>
					<!-- Main content: trace table -->
					<div class="flex-1 overflow-hidden">
						<!-- Toolbar -->
						<div class="flex items-center gap-3 px-4 py-2.5 border-b border-[#e5e5e5]">
							<div class="flex items-center gap-0 border border-[#e5e5e5] rounded-md overflow-hidden">
								<div class="px-2.5 py-1 text-[10px] font-semibold bg-white text-text border-r border-[#e5e5e5]">Spans</div>
								<div class="px-2.5 py-1 text-[10px] text-text-muted bg-[#fafafa]">Traces</div>
							</div>
							<div class="px-2.5 py-1 text-[10px] text-text-muted border border-[#e5e5e5] rounded-md bg-white">Last 7 days</div>
							<div class="flex-1"></div>
							<div class="px-3 py-1 text-[10px] text-text-muted border border-[#e5e5e5] rounded-md bg-white hidden sm:block">Search name, model, id...</div>
						</div>
						<!-- Column headers -->
						<div class="flex items-center px-4 py-2 border-b border-[#e5e5e5] bg-[#fafafa]">
							<span class="text-[9px] font-semibold text-text-muted uppercase tracking-wider w-[140px] shrink-0">Time</span>
							<span class="text-[9px] font-semibold text-text-muted uppercase tracking-wider flex-1">Name</span>
							<span class="text-[9px] font-semibold text-text-muted uppercase tracking-wider w-[60px] text-right hidden sm:block">Kind</span>
							<span class="text-[9px] font-semibold text-text-muted uppercase tracking-wider w-[90px] text-right hidden md:block">Model</span>
							<span class="text-[9px] font-semibold text-text-muted uppercase tracking-wider w-[50px] text-right hidden lg:block">Latency</span>
						</div>
						<!-- Rows -->
						{#each [
							{ time: '11:20:58', name: 'send-customer-response', kind: 'Custom', model: '–', latency: '16ms' },
							{ time: '11:20:58', name: 'human-approval-check', kind: 'Custom', model: '–', latency: '16ms' },
							{ time: '11:20:46', name: 'claude-draft-customer-reply', kind: 'LLM', model: 'claude-sonnet-4...', latency: '4.29s', highlight: true },
							{ time: '11:20:37', name: 'claude-decide-resolution', kind: 'LLM', model: 'claude-sonnet-4...', latency: '8.42s' },
							{ time: '11:20:37', name: 'lookup-account-context', kind: 'Custom', model: '–', latency: '23ms' },
							{ time: '11:20:33', name: 'claude-understand-intent', kind: 'LLM', model: 'claude-sonnet-4...', latency: '3.66s' },
							{ time: '11:20:33', name: 'receive-customer-request', kind: 'Custom', model: '–', latency: '25ms' },
							{ time: '11:20:33', name: 'send-customer-response', kind: 'Custom', model: '–', latency: '27ms' },
							{ time: '11:20:33', name: 'human-approval-check', kind: 'Custom', model: '–', latency: '27ms' },
							{ time: '11:20:28', name: 'claude-draft-customer-reply', kind: 'LLM', model: 'claude-sonnet-4...', latency: '5.52s' },
							{ time: '11:20:19', name: 'claude-decide-resolution', kind: 'LLM', model: 'claude-sonnet-4...', latency: '9.10s' },
							{ time: '11:20:18', name: 'lookup-account-context', kind: 'Custom', model: '–', latency: '41ms' },
							{ time: '11:20:16', name: 'claude-understand-intent', kind: 'LLM', model: 'claude-sonnet-4...', latency: '2.66s' },
						] as row}
							<div class="flex items-center px-4 py-[7px] border-b border-[#f0f0f0] hover:bg-[#fafafa] transition-colors {row.highlight ? 'bg-[#f0fdf4]' : ''}">
								<span class="text-[10px] text-text-muted font-mono w-[140px] shrink-0">{row.time}</span>
								<span class="text-[10px] text-text flex-1 truncate flex items-center gap-1.5">
									<span class="w-1.5 h-1.5 rounded-full bg-[#10b981] shrink-0"></span>
									{row.name}
								</span>
								<span class="text-[10px] text-text-muted w-[60px] text-right hidden sm:block">{row.kind}</span>
								<span class="text-[10px] text-text-muted font-mono w-[90px] text-right truncate hidden md:block">{row.model}</span>
								<span class="text-[10px] text-text-muted w-[50px] text-right hidden lg:block">{row.latency}</span>
							</div>
						{/each}
						<!-- Bottom bar -->
						<div class="flex items-center justify-between px-4 py-2 border-t border-[#e5e5e5] bg-[#fafafa]">
							<span class="text-[9px] text-text-muted">Rows per page: 100</span>
							<span class="text-[9px] text-text-muted">Total cost: $8.89 &middot; 168 of 168 spans</span>
						</div>
					</div>
				</div>
			</div>
		</InView>
	</div>
</section>

<!-- VALUE PROPS — 3 columns -->
<section class="py-20 md:py-28 border-t border-border">
	<div class="max-w-[1200px] mx-auto px-6">
		<div class="grid grid-cols-1 md:grid-cols-3 gap-10 md:gap-12">
			<InView>
				<div>
					<div class="w-10 h-10 rounded-lg border border-border flex items-center justify-center mb-4">
						<svg class="w-5 h-5 text-text" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z" /></svg>
					</div>
					<p class="text-[15px] leading-relaxed">
						<span class="font-semibold text-text">Debug faster.</span>
						<span class="text-text-secondary"> Trace every request from SDK to model. See inputs, outputs, latency, and cost in one view.</span>
					</p>
					<a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="inline-flex items-center gap-1 text-[14px] text-text font-medium mt-4 hover:gap-2 transition-all">
						Read the docs
						<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" /></svg>
					</a>
				</div>
			</InView>
			<InView delay={80}>
				<div>
					<div class="w-10 h-10 rounded-lg border border-border flex items-center justify-center mb-4">
						<svg class="w-5 h-5 text-text" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18L9 11.25l4.306 4.307a11.95 11.95 0 015.814-5.519l2.74-1.22m0 0l-5.94-2.28m5.94 2.28l-2.28 5.941" /></svg>
					</div>
					<p class="text-[15px] leading-relaxed">
						<span class="font-semibold text-text">Ship with confidence.</span>
						<span class="text-text-secondary"> Run evaluations against production data. Compare models side-by-side. Catch regressions before your users do.</span>
					</p>
					<a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="inline-flex items-center gap-1 text-[14px] text-text font-medium mt-4 hover:gap-2 transition-all">
						Read the docs
						<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" /></svg>
					</a>
				</div>
			</InView>
			<InView delay={160}>
				<div>
					<div class="w-10 h-10 rounded-lg border border-border flex items-center justify-center mb-4">
						<svg class="w-5 h-5 text-text" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" /></svg>
					</div>
					<p class="text-[15px] leading-relaxed">
						<span class="font-semibold text-text">Own your data.</span>
						<span class="text-text-secondary"> Self-host for free forever, or use our cloud. Open source, MIT licensed. No vendor lock-in.</span>
					</p>
					<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="inline-flex items-center gap-1 text-[14px] text-text font-medium mt-4 hover:gap-2 transition-all">
						View on GitHub
						<svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" /></svg>
					</a>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- FEATURES — Browserbase-style sticky scroll -->
<section id="features" class="py-24 md:py-36 border-t border-border">
	<div class="max-w-[1200px] mx-auto px-6">
		<InView>
			<h2 class="text-[clamp(1.75rem,4vw,2.75rem)] font-bold tracking-tight text-text mb-4">Features</h2>
			<p class="text-[16px] text-text-secondary max-w-lg mb-20">
				The full observability stack for AI &mdash; trace, analyze, evaluate, and review every call in one platform.
			</p>
		</InView>

		<!-- Sticky scroll container -->
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-8 lg:gap-16">
			<!-- Left: sticky visual -->
			<div class="hidden lg:block">
				<div class="sticky top-32" style="height: calc(100vh - 200px); display: flex; align-items: flex-start; padding-top: 40px;">
					<div class="w-full relative">
						<!-- Traces mock -->
						<div class="visual-panel {activeFeature === 0 ? 'is-visible' : ''}" style={activeFeature === 0 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="mock shadow-md" style="font-size:12px">
								<div class="mock-header" style="padding:10px 16px">
									<span style="font-size:12px">Traces</span>
									<span class="ml-auto text-[10px] font-normal text-text-muted">Last 1h</span>
								</div>
								{#each [
									{ name: 'customer-support-bot', spans: '4 spans', ms: '2.3s', cost: '$0.043', status: '#10b981' },
									{ name: 'email-drafting-agent', spans: '7 spans', ms: '4.1s', cost: '$0.089', status: '#10b981' },
									{ name: 'code-review-bot', spans: '3 spans', ms: '1.8s', cost: '$0.021', status: '#ef4444' },
									{ name: 'data-extraction', spans: '5 spans', ms: '3.2s', cost: '$0.056', status: '#10b981' },
									{ name: 'search-assistant', spans: '6 spans', ms: '2.7s', cost: '$0.034', status: '#10b981' },
									{ name: 'onboarding-flow', spans: '8 spans', ms: '5.4s', cost: '$0.112', status: '#10b981' },
									{ name: 'summarization-pipeline', spans: '4 spans', ms: '3.0s', cost: '$0.067', status: '#eab308' },
								] as row}
									<div class="mock-row" style="padding:9px 16px">
										<div class="mock-dot" style="background:{row.status}"></div>
										<span class="font-mono text-[11px] text-text flex-1 truncate">{row.name}</span>
										<span class="text-[10px] text-text-muted">{row.spans}</span>
										<span class="text-[10px] text-text-muted w-10 text-right">{row.ms}</span>
										<span class="text-[10px] text-text-muted w-12 text-right">{row.cost}</span>
									</div>
								{/each}
							</div>
						</div>

						<!-- Analytics mock -->
						<div class="visual-panel {activeFeature === 1 ? 'is-visible' : ''}" style={activeFeature === 1 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="grid grid-cols-3 gap-3 mb-4">
								<div class="mock shadow-md p-4">
									<div class="text-[10px] text-text-muted mb-1">Total Cost</div>
									<div class="text-[20px] font-bold text-text">$142.38</div>
									<div class="flex items-center gap-1 mt-1"><span class="text-[10px] text-emerald-500 font-medium">-12%</span><span class="text-[9px] text-text-muted">vs last week</span></div>
								</div>
								<div class="mock shadow-md p-4">
									<div class="text-[10px] text-text-muted mb-1">LLM Calls</div>
									<div class="text-[20px] font-bold text-text">12,847</div>
									<div class="flex items-center gap-1 mt-1"><span class="text-[10px] text-emerald-500 font-medium">+8%</span><span class="text-[9px] text-text-muted">vs last week</span></div>
								</div>
								<div class="mock shadow-md p-4">
									<div class="text-[10px] text-text-muted mb-1">Avg Latency</div>
									<div class="text-[20px] font-bold text-text">340ms</div>
									<div class="flex items-center gap-1 mt-1"><span class="text-[10px] text-emerald-500 font-medium">-5%</span><span class="text-[9px] text-text-muted">P95: 1.2s</span></div>
								</div>
							</div>
							<div class="mock shadow-md p-4">
								<div class="flex items-center justify-between mb-3">
									<span class="text-[11px] font-semibold text-text">Tokens by model</span>
									<span class="text-[9px] text-text-muted">Last 7 days</span>
								</div>
								<div class="flex items-end gap-2 h-24">
									{#each [40, 65, 55, 80, 70, 45, 90] as h}
										<div class="flex-1 flex flex-col gap-0.5">
											<div class="rounded-sm bg-[#6366f1]/20" style="height:{h * 0.4}%"></div>
											<div class="rounded-sm bg-[#10b981]/30" style="height:{h * 0.6}%"></div>
										</div>
									{/each}
								</div>
								<div class="flex items-center gap-3 mt-3">
									<span class="flex items-center gap-1 text-[9px] text-text-muted"><span class="w-2 h-2 rounded-sm bg-[#10b981]/30"></span>Input</span>
									<span class="flex items-center gap-1 text-[9px] text-text-muted"><span class="w-2 h-2 rounded-sm bg-[#6366f1]/20"></span>Output</span>
								</div>
							</div>
						</div>

						<!-- Evaluations mock -->
						<div class="visual-panel {activeFeature === 2 ? 'is-visible' : ''}" style={activeFeature === 2 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="mock shadow-md">
								<div class="mock-header" style="padding:10px 16px">
									<span style="font-size:12px">Eval Runs</span>
									<span class="ml-auto text-[10px] font-normal text-text-muted">accuracy-benchmark-v3</span>
								</div>
								{#each [
									{ name: 'gpt-4o', score: '94%', status: '#10b981' },
									{ name: 'claude-3.5-sonnet', score: '91%', status: '#10b981' },
									{ name: 'gpt-4o-mini', score: '78%', status: '#eab308' },
									{ name: 'llama-3.1-70b', score: '72%', status: '#eab308' },
									{ name: 'mistral-large', score: '85%', status: '#10b981' },
								] as run}
									<div class="mock-row" style="padding:10px 16px">
										<div class="mock-dot" style="background:{run.status}"></div>
										<span class="font-mono text-[11px] text-text flex-1">{run.name}</span>
										<span class="mock-chip" style="background:{run.status}15;color:{run.status}">{run.score}</span>
									</div>
								{/each}
							</div>
						</div>

						<!-- Search mock -->
						<div class="visual-panel {activeFeature === 3 ? 'is-visible' : ''}" style={activeFeature === 3 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="mock shadow-md">
								<div class="px-4 py-3 border-b border-[#f0f0f0] flex items-center gap-2">
									<svg class="w-3.5 h-3.5 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z" /></svg>
									<span class="text-[11px] text-text-muted font-mono">kind:llm model:gpt-4o cost:&gt;0.01</span>
								</div>
								<div class="p-3 flex flex-wrap gap-1.5">
									<span class="mock-chip bg-[#dbeafe] text-[#2563eb]" style="font-size:10px;padding:2px 8px">kind:llm</span>
									<span class="mock-chip bg-[#f3e8ff] text-[#7c3aed]" style="font-size:10px;padding:2px 8px">model:gpt-4o</span>
									<span class="mock-chip bg-[#dcfce7] text-[#16a34a]" style="font-size:10px;padding:2px 8px">cost:&gt;0.01</span>
									<span class="mock-chip bg-[#fef3c7] text-[#d97706]" style="font-size:10px;padding:2px 8px">since:1h</span>
								</div>
								<div class="px-4 py-2 text-[10px] text-text-muted border-t border-[#f0f0f0]">247 results in 12ms</div>
							</div>
						</div>

						<!-- Review mock -->
						<div class="visual-panel {activeFeature === 4 ? 'is-visible' : ''}" style={activeFeature === 4 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="mock shadow-md">
								<div class="mock-header" style="padding:10px 16px">
									<span style="font-size:12px">Review Queue</span>
									<span class="mock-chip bg-[#fef3c7] text-[#d97706] ml-auto" style="font-size:10px;padding:2px 8px">3 pending</span>
								</div>
								{#each [
									{ preview: '"I found your order and have initiated a refund..."', state: 'Pending', c: '#eab308' },
									{ preview: '"Here are the refund steps for your account..."', state: 'Claimed', c: '#6366f1' },
									{ preview: '"Your account has been updated with the new plan..."', state: 'Approved', c: '#10b981' },
									{ preview: '"Based on your usage, I recommend upgrading..."', state: 'Pending', c: '#eab308' },
								] as item}
									<div class="mock-row" style="padding:10px 16px">
										<div class="mock-dot" style="background:{item.c}"></div>
										<span class="text-[11px] text-text-secondary flex-1 truncate">{item.preview}</span>
										<span class="mock-chip" style="background:{item.c}15;color:{item.c};font-size:10px;padding:2px 8px">{item.state}</span>
									</div>
								{/each}
							</div>
						</div>

						<!-- Datasets mock -->
						<div class="visual-panel {activeFeature === 5 ? 'is-visible' : ''}" style={activeFeature === 5 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="mock shadow-md">
								<div class="mock-header" style="padding:10px 16px">
									<span style="font-size:12px">Datasets</span>
								</div>
								{#each [
									{ name: 'customer-support-v2', count: '1,247 rows', date: '2h ago' },
									{ name: 'code-review-golden', count: '328 rows', date: '1d ago' },
									{ name: 'summarization-test', count: '89 rows', date: '3d ago' },
									{ name: 'onboarding-flows', count: '512 rows', date: '5d ago' },
								] as ds}
									<div class="mock-row" style="padding:10px 16px">
										<span class="font-mono text-[11px] text-text flex-1">{ds.name}</span>
										<span class="text-[10px] text-text-muted">{ds.count}</span>
										<span class="text-[10px] text-text-muted w-12 text-right">{ds.date}</span>
									</div>
								{/each}
							</div>
						</div>

						<!-- Sessions mock -->
						<div class="visual-panel {activeFeature === 6 ? 'is-visible' : ''}" style={activeFeature === 6 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="mock shadow-md">
								<div class="mock-header" style="padding:10px 16px">
									<span style="font-size:12px">Sessions</span>
								</div>
								{#each [
									{ id: 'sess_a8f2...', turns: '12 turns', tokens: '4.2k tok', cost: '$0.034' },
									{ id: 'sess_c3d1...', turns: '8 turns', tokens: '2.8k tok', cost: '$0.021' },
									{ id: 'sess_e7b4...', turns: '23 turns', tokens: '9.1k tok', cost: '$0.089' },
									{ id: 'sess_f1a9...', turns: '5 turns', tokens: '1.4k tok', cost: '$0.012' },
								] as s}
									<div class="mock-row" style="padding:10px 16px">
										<span class="font-mono text-[11px] text-text flex-1">{s.id}</span>
										<span class="text-[10px] text-text-muted">{s.turns}</span>
										<span class="text-[10px] text-text-muted w-14 text-right">{s.tokens}</span>
										<span class="text-[10px] text-text-muted w-12 text-right">{s.cost}</span>
									</div>
								{/each}
							</div>
						</div>

						<!-- Integrations mock -->
						<div class="visual-panel {activeFeature === 7 ? 'is-visible' : ''}" style={activeFeature === 7 ? '' : 'position:absolute;top:0;left:0;right:0'}>
							<div class="mock shadow-md">
								<div class="mock-header" style="padding:10px 16px">
									<span style="font-size:12px">Provider Connections</span>
								</div>
								{#each [
									{ name: 'OpenAI', status: 'Connected', c: '#10b981' },
									{ name: 'Anthropic', status: 'Connected', c: '#10b981' },
									{ name: 'Ollama (local)', status: 'Connected', c: '#10b981' },
									{ name: 'Azure OpenAI', status: 'Not configured', c: '#a3a3a3' },
									{ name: 'Google Vertex AI', status: 'Not configured', c: '#a3a3a3' },
								] as p}
									<div class="mock-row" style="padding:10px 16px">
										<span class="text-[11px] text-text flex-1">{p.name}</span>
										<span class="mock-chip" style="background:{p.c}15;color:{p.c};font-size:10px;padding:2px 8px">{p.status}</span>
									</div>
								{/each}
							</div>
						</div>
					</div>
				</div>
			</div>

			<!-- Right: scrollable feature list -->
			<div>
				{#each features as feature, i}
					<div
						class="feature-item {activeFeature === i ? 'is-active' : ''}"
						bind:this={featureEls[i]}
						role="button"
						tabindex="0"
						onclick={() => { activeFeature = i; }}
						onkeydown={(e) => { if (e.key === 'Enter') activeFeature = i; }}
					>
						<h3 class="text-[20px] font-semibold text-text mb-2">{feature.title}</h3>
						<p class="text-[14px] text-text-secondary leading-relaxed">{feature.desc}</p>

						<!-- Mobile: show mock inline -->
						<div class="lg:hidden mt-5">
							{#if feature.mockType === 'traces'}
								<div class="mock shadow-sm">
									<div class="mock-header"><span>Traces</span><span class="ml-auto text-[9px] font-normal text-text-muted">Last 1h</span></div>
									{#each [
										{ name: 'customer-support-bot', spans: '4', ms: '2.3s', cost: '$0.043', status: '#10b981' },
										{ name: 'email-drafting-agent', spans: '7', ms: '4.1s', cost: '$0.089', status: '#10b981' },
										{ name: 'code-review-bot', spans: '3', ms: '1.8s', cost: '$0.021', status: '#ef4444' },
									] as row}
										<div class="mock-row">
											<div class="mock-dot" style="background:{row.status}"></div>
											<span class="font-mono text-[10px] text-text flex-1 truncate">{row.name}</span>
											<span class="text-[9px] text-text-muted">{row.spans} spans</span>
											<span class="text-[9px] text-text-muted w-8 text-right">{row.ms}</span>
											<span class="text-[9px] text-text-muted w-10 text-right">{row.cost}</span>
										</div>
									{/each}
								</div>
							{:else if feature.mockType === 'analytics'}
								<div class="grid grid-cols-3 gap-2">
									<div class="mock shadow-sm p-3">
										<div class="text-[9px] text-text-muted mb-1">Total Cost</div>
										<div class="text-[14px] font-bold text-text">$142.38</div>
									</div>
									<div class="mock shadow-sm p-3">
										<div class="text-[9px] text-text-muted mb-1">LLM Calls</div>
										<div class="text-[14px] font-bold text-text">12,847</div>
									</div>
									<div class="mock shadow-sm p-3">
										<div class="text-[9px] text-text-muted mb-1">Avg Latency</div>
										<div class="text-[14px] font-bold text-text">340ms</div>
									</div>
								</div>
							{:else if feature.mockType === 'evaluations'}
								<div class="mock shadow-sm">
									<div class="mock-header"><span>Eval Runs</span></div>
									{#each [
										{ name: 'gpt-4o', score: '94%', status: '#10b981' },
										{ name: 'claude-3.5', score: '91%', status: '#10b981' },
										{ name: 'gpt-4o-mini', score: '78%', status: '#eab308' },
									] as run}
										<div class="mock-row">
											<div class="mock-dot" style="background:{run.status}"></div>
											<span class="font-mono text-[10px] text-text flex-1">{run.name}</span>
											<span class="mock-chip" style="background:{run.status}15;color:{run.status}">{run.score}</span>
										</div>
									{/each}
								</div>
							{:else if feature.mockType === 'search'}
								<div class="mock shadow-sm">
									<div class="p-2.5 flex flex-wrap gap-1.5">
										<span class="mock-chip bg-[#dbeafe] text-[#2563eb]">kind:llm</span>
										<span class="mock-chip bg-[#f3e8ff] text-[#7c3aed]">model:gpt-4o</span>
										<span class="mock-chip bg-[#dcfce7] text-[#16a34a]">cost:&gt;0.01</span>
									</div>
								</div>
							{:else if feature.mockType === 'review'}
								<div class="mock shadow-sm">
									<div class="mock-header"><span>Review Queue</span><span class="mock-chip bg-[#fef3c7] text-[#d97706] ml-auto">3 pending</span></div>
									{#each [
										{ preview: '"I found your order..."', state: 'Pending', c: '#eab308' },
										{ preview: '"Here are the refund steps..."', state: 'Claimed', c: '#6366f1' },
									] as item}
										<div class="mock-row">
											<div class="mock-dot" style="background:{item.c}"></div>
											<span class="text-[10px] text-text-secondary flex-1 truncate">{item.preview}</span>
											<span class="mock-chip" style="background:{item.c}15;color:{item.c}">{item.state}</span>
										</div>
									{/each}
								</div>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
</section>

<!-- PRICING -->
<section id="pricing" class="py-24 md:py-36 border-t border-border">
	<div class="max-w-[1200px] mx-auto px-6">
		<InView>
			<div class="mb-16">
				<h2 class="text-[clamp(1.75rem,4vw,2.75rem)] font-bold tracking-tight text-text">Start free. Scale when ready.</h2>
				<p class="text-text-secondary mt-4 max-w-md">Self-host for free forever, or let us handle the infrastructure.</p>
			</div>
		</InView>
		<div class="grid md:grid-cols-3 gap-4 max-w-3xl">
			{#each [
				{ name: 'Free', price: '$0', period: '/mo', pop: false, desc: 'Local dev and getting started.', features: ['10K spans/month', '7-day retention', '1 team member', 'Community support'], cta: 'Get started' },
				{ name: 'Pro', price: '$20', period: '/user/mo', pop: true, desc: 'Teams shipping AI to production.', features: ['1M spans/month', '30-day retention', '5 team members', 'Email support'], cta: 'Start free trial' },
				{ name: 'Team', price: '$100', period: '/user/mo', pop: false, desc: 'Full observability at scale.', features: ['10M spans/month', '90-day retention', '50 team members', 'Priority support'], cta: 'Get started' },
			] as plan, i}
				<InView delay={i * 60}>
					<div class="relative {plan.pop ? 'bg-white border-2 border-text shadow-lg' : 'bg-white border border-border'} rounded-lg p-6 flex flex-col h-full transition-all duration-200 hover:translate-y-[-2px] hover:shadow-md">
						{#if plan.pop}
							<div class="absolute -top-2.5 left-6">
								<span class="bg-text text-white text-[9px] font-bold uppercase tracking-wider px-3 py-0.5 rounded-full">Popular</span>
							</div>
						{/if}
						<div class="mb-5">
							<div class="text-[11px] {plan.pop ? 'text-text' : 'text-text-muted'} uppercase tracking-wider mb-2 font-mono font-bold">{plan.name}</div>
							<div class="flex items-baseline gap-1">
								<span class="text-3xl font-bold text-text">{plan.price}</span>
								<span class="text-sm text-text-muted">{plan.period}</span>
							</div>
							<p class="text-[13px] text-text-secondary mt-1.5">{plan.desc}</p>
						</div>
						<ul class="space-y-2.5 text-[13px] flex-1 mb-6">
							{#each plan.features as f}
								<li class="flex items-center gap-2 text-text-secondary">
									<svg class="w-3.5 h-3.5 shrink-0 {plan.pop ? 'text-text' : 'text-text-muted'}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" /></svg>
									{f}
								</li>
							{/each}
						</ul>
						<a href="https://platform.traceway.ai/signup" class="block text-center text-[14px] rounded-md px-4 py-2.5 transition-all {plan.pop ? 'font-semibold bg-text text-white hover:bg-text/85' : 'text-text border border-border hover:bg-bg-secondary'}">{plan.cta}</a>
					</div>
				</InView>
			{/each}
		</div>
		<div class="grid md:grid-cols-2 gap-4 max-w-3xl mt-4">
			{#each [
				{ name: 'Enterprise', desc: 'Unlimited everything. SSO, SAML, dedicated support.', cta: 'Contact', href: 'mailto:support@traceway.ai' },
				{ name: 'Self-hosted', desc: 'Free forever. Unlimited traces. No account needed.', cta: 'GitHub', href: 'https://github.com/blastgits/traceway' },
			] as row}
				<InView delay={80}>
					<div class="bg-white border border-border rounded-lg px-6 py-4 flex items-center justify-between gap-4 hover:shadow-sm transition-all">
						<div>
							<div class="text-[14px] font-semibold text-text">{row.name}</div>
							<p class="text-[13px] text-text-secondary mt-0.5">{row.desc}</p>
						</div>
						<a href={row.href} class="shrink-0 text-[13px] text-text border border-border rounded-md px-4 py-1.5 hover:bg-bg-secondary transition-all">{row.cta}</a>
					</div>
				</InView>
			{/each}
		</div>
		<p class="text-[12px] text-text-muted mt-6">
			Billed monthly via <a href="https://polar.sh" target="_blank" rel="noopener" class="text-accent hover:text-accent-dim transition-colors">Polar</a>. Cancel anytime.
		</p>
	</div>
</section>

<!-- CTA -->
<section class="py-24 md:py-36 border-t border-border grid-bg">
	<div class="max-w-[1200px] mx-auto px-6">
		<div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-8">
			<InView>
				<h2 class="text-[clamp(2rem,4.5vw,3.5rem)] font-bold tracking-tight leading-tight">What will you build?</h2>
			</InView>
			<InView delay={80}>
				<div class="flex flex-col sm:flex-row gap-3">
					<a href="mailto:support@traceway.ai" class="inline-flex items-center justify-center text-[15px] text-text border-2 border-text rounded-md px-8 py-3 hover:bg-text hover:text-white transition-all font-medium">Get a Demo</a>
					<a href="https://platform.traceway.ai/signup" class="inline-flex items-center justify-center text-[15px] text-white bg-text rounded-md px-8 py-3 hover:bg-text/85 transition-all font-semibold">Get Started</a>
				</div>
			</InView>
		</div>
	</div>
</section>

<!-- FOOTER -->
<footer class="bg-[#0f172a] text-white">
	<div class="max-w-[1200px] mx-auto px-6 py-12">
		<div class="grid grid-cols-2 md:grid-cols-4 gap-8">
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Product</h4>
				<ul class="space-y-2.5">
					<li><a href="#features" class="text-[13px] text-white/60 hover:text-white transition-colors">Features</a></li>
					<li><a href="#pricing" class="text-[13px] text-white/60 hover:text-white transition-colors">Pricing</a></li>
					<li><a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">Docs</a></li>
				</ul>
			</div>
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Company</h4>
				<ul class="space-y-2.5">
					<li><a href="/blog" class="text-[13px] text-white/60 hover:text-white transition-colors">Blog</a></li>
					<li><a href="mailto:support@traceway.ai" class="text-[13px] text-white/60 hover:text-white transition-colors">Contact</a></li>
				</ul>
			</div>
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Developers</h4>
				<ul class="space-y-2.5">
					<li><a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">GitHub</a></li>
					<li><a href="https://docs.traceway.ai" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">API Docs</a></li>
					<li><a href="https://github.com/blastgits/traceway/blob/main/CHANGELOG.md" target="_blank" rel="noopener" class="text-[13px] text-white/60 hover:text-white transition-colors">Changelog</a></li>
				</ul>
			</div>
			<div>
				<h4 class="text-[13px] font-semibold text-white mb-4">Legal</h4>
				<ul class="space-y-2.5">
					<li><a href="/privacy" class="text-[13px] text-white/60 hover:text-white transition-colors">Privacy Policy</a></li>
					<li><a href="/terms" class="text-[13px] text-white/60 hover:text-white transition-colors">Terms of Service</a></li>
				</ul>
			</div>
		</div>
		<div class="mt-10 pt-6 border-t border-white/10 flex flex-col md:flex-row items-center justify-between gap-4">
			<div class="flex items-center gap-4">
				<span class="text-[12px] text-white/40">MIT Licensed</span>
				<span class="text-[12px] text-white/40">Open source LLM observability</span>
			</div>
			<div class="flex items-center gap-4">
				<a href="https://github.com/blastgits/traceway" target="_blank" rel="noopener" class="text-white/40 hover:text-white transition-colors" aria-label="GitHub">
					<svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
				</a>
				<a href="https://x.com/traceway" target="_blank" rel="noopener" class="text-white/40 hover:text-white transition-colors" aria-label="X (Twitter)">
					<svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24"><path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/></svg>
				</a>
			</div>
		</div>
	</div>
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
