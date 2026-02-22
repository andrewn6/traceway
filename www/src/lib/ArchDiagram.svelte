<script lang="ts">
	// Architecture diagram — hand-coded SVG matching the dark theme
	// Your App -> Traceway Daemon (Proxy + Storage + REST API) -> LLM Provider
	// Web UI <-> Traceway Daemon

	const boxFill = '#14141e';
	const boxStroke = '#252530';
	const boxStrokeHover = '#6ee7b7';
	const textColor = '#e8eaf0';
	const textMuted = '#555570';
	const textSecondary = '#8b8fa4';
	const accent = '#6ee7b7';
	const accentDim = '#34d399';
	const arrowColor = '#3a3a50';
	const font = "'JetBrains Mono', 'Fira Code', monospace";
</script>

<svg
	viewBox="0 0 820 380"
	class="w-full"
	xmlns="http://www.w3.org/2000/svg"
>
	<defs>
		<marker
			id="arrow-right"
			markerWidth="8"
			markerHeight="6"
			refX="8"
			refY="3"
			orient="auto"
		>
			<path d="M0,0 L8,3 L0,6" fill={arrowColor} />
		</marker>
		<marker
			id="arrow-left"
			markerWidth="8"
			markerHeight="6"
			refX="0"
			refY="3"
			orient="auto"
		>
			<path d="M8,0 L0,3 L8,6" fill={arrowColor} />
		</marker>
		<marker
			id="arrow-accent"
			markerWidth="8"
			markerHeight="6"
			refX="8"
			refY="3"
			orient="auto"
		>
			<path d="M0,0 L8,3 L0,6" fill={accentDim} opacity="0.5" />
		</marker>
	</defs>

	<!-- ═══ YOUR APP ═══ -->
	<g>
		<rect x="20" y="80" width="160" height="120" rx="6" fill={boxFill} stroke={boxStroke} stroke-width="1" />
		<!-- Label -->
		<text x="100" y="68" fill={textSecondary} font-family={font} font-size="10" text-anchor="middle" letter-spacing="0.08em">YOUR APP</text>
		<!-- Content -->
		<text x="100" y="135" fill={textMuted} font-family={font} font-size="12" text-anchor="middle">LLM calls</text>
		<text x="100" y="155" fill={textMuted} font-family={font} font-size="10" text-anchor="middle">:3001/v1/...</text>
	</g>

	<!-- ═══ TRACEWAY DAEMON ═══ -->
	<g>
		<!-- Outer container -->
		<rect x="270" y="30" width="280" height="320" rx="6" fill={boxFill} stroke={boxStroke} stroke-width="1" />
		<text x="410" y="20" fill={textSecondary} font-family={font} font-size="10" text-anchor="middle" letter-spacing="0.08em">TRACEWAY DAEMON</text>

		<!-- Proxy box -->
		<rect x="300" y="60" width="220" height="60" rx="4" fill="none" stroke={accentDim} stroke-width="1" opacity="0.3" />
		<text x="410" y="85" fill={accent} font-family={font} font-size="12" text-anchor="middle" font-weight="500">Proxy</text>
		<text x="410" y="103" fill={textMuted} font-family={font} font-size="10" text-anchor="middle">:3001 — intercept + trace</text>

		<!-- Storage box -->
		<rect x="300" y="145" width="220" height="70" rx="4" fill="none" stroke={boxStroke} stroke-width="1" />
		<text x="410" y="172" fill={accent} font-family={font} font-size="12" text-anchor="middle" font-weight="500">Storage</text>
		<text x="410" y="192" fill={textMuted} font-family={font} font-size="10" text-anchor="middle">SQLite / Postgres / Turbopuffer</text>

		<!-- REST API box -->
		<rect x="300" y="240" width="220" height="60" rx="4" fill="none" stroke={boxStroke} stroke-width="1" />
		<text x="410" y="265" fill={accent} font-family={font} font-size="12" text-anchor="middle" font-weight="500">REST API</text>
		<text x="410" y="283" fill={textMuted} font-family={font} font-size="10" text-anchor="middle">:3000/api — traces, analytics</text>

		<!-- Internal arrow: Proxy -> Storage -->
		<line x1="410" y1="120" x2="410" y2="145" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-right)" style="transform: rotate(90deg); transform-origin: 410px 132px;" />
		<line x1="410" y1="120" x2="410" y2="145" stroke={arrowColor} stroke-width="1" />
		<path d="M410,120 L410,145" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-accent)" />
	</g>

	<!-- ═══ LLM PROVIDER ═══ -->
	<g>
		<rect x="640" y="60" width="160" height="120" rx="6" fill={boxFill} stroke={boxStroke} stroke-width="1" />
		<text x="720" y="48" fill={textSecondary} font-family={font} font-size="10" text-anchor="middle" letter-spacing="0.08em">LLM PROVIDER</text>
		<text x="720" y="110" fill={textColor} font-family={font} font-size="11" text-anchor="middle">OpenAI</text>
		<text x="720" y="128" fill={textMuted} font-family={font} font-size="11" text-anchor="middle">Anthropic</text>
		<text x="720" y="146" fill={textMuted} font-family={font} font-size="11" text-anchor="middle">Ollama</text>
		<text x="720" y="164" fill={textMuted} font-family={font} font-size="10" text-anchor="middle">any OpenAI-compat.</text>
	</g>

	<!-- ═══ WEB UI ═══ -->
	<g>
		<rect x="20" y="240" width="160" height="60" rx="6" fill={boxFill} stroke={boxStroke} stroke-width="1" />
		<text x="100" y="275" fill={accent} font-family={font} font-size="12" text-anchor="middle" font-weight="500">Web UI</text>
		<text x="100" y="291" fill={textMuted} font-family={font} font-size="10" text-anchor="middle">:3000</text>
	</g>

	<!-- ═══ ARROWS ═══ -->

	<!-- Your App -> Proxy (request) -->
	<line x1="180" y1="120" x2="300" y2="90" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-right)" />
	<text x="235" y="96" fill={textMuted} font-family={font} font-size="9" text-anchor="middle">request</text>

	<!-- Proxy -> Your App (response) -->
	<line x1="300" y1="105" x2="180" y2="150" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-left)" />
	<text x="235" y="140" fill={textMuted} font-family={font} font-size="9" text-anchor="middle">response</text>

	<!-- Proxy -> LLM Provider -->
	<line x1="520" y1="80" x2="640" y2="100" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-right)" />
	<text x="580" y="82" fill={textMuted} font-family={font} font-size="9" text-anchor="middle">forward</text>

	<!-- LLM Provider -> Proxy -->
	<line x1="640" y1="140" x2="520" y2="105" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-left)" />
	<text x="580" y="135" fill={textMuted} font-family={font} font-size="9" text-anchor="middle">response</text>

	<!-- Web UI <-> REST API -->
	<line x1="180" y1="270" x2="300" y2="270" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-right)" />
	<line x1="300" y1="260" x2="180" y2="260" stroke={arrowColor} stroke-width="1" marker-end="url(#arrow-left)" />
	<text x="240" y="252" fill={textMuted} font-family={font} font-size="9" text-anchor="middle">API</text>

	<!-- Proxy -> Storage (vertical, inside daemon) -->
	<!-- Already drawn as internal connection above -->

	<!-- Storage -> REST API (vertical, inside daemon) -->
	<path d="M410,215 L410,240" stroke={arrowColor} stroke-width="1" />
</svg>
