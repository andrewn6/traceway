import type { SpanFilter } from './api';

const KEY_MAP: Record<string, keyof SpanFilter> = {
	kind: 'kind',
	model: 'model',
	provider: 'provider',
	status: 'status',
	name: 'name_contains',
	path: 'path',
	trace: 'trace_id',
	since: 'since',
	until: 'until',
	sort: 'sort_by',
	order: 'sort_order'
};

const REVERSE_MAP: Record<keyof SpanFilter, string> = {
	kind: 'kind',
	model: 'model',
	provider: 'provider',
	status: 'status',
	name_contains: 'name',
	path: 'path',
	trace_id: 'trace',
	since: 'since',
	until: 'until',
	duration_min: 'duration_min',
	duration_max: 'duration_max',
	tokens_min: 'tokens_min',
	cost_min: 'cost_min',
	sort_by: 'sort',
	sort_order: 'order'
};

const RELATIVE_TIME_RE = /^(\d+)(m|h|d)$/;

/** Parse "500ms", "1.5s", "2s" into milliseconds */
function parseDurationMs(value: string): number | null {
	const msMatch = value.match(/^(\d+)ms$/);
	if (msMatch) return parseInt(msMatch[1], 10);
	const sMatch = value.match(/^([\d.]+)s$/);
	if (sMatch) return Math.round(parseFloat(sMatch[1]) * 1000);
	// Bare number = ms
	const num = parseFloat(value);
	if (!isNaN(num)) return Math.round(num);
	return null;
}

export function parseRelativeTime(value: string): string | null {
	const match = value.match(RELATIVE_TIME_RE);
	if (!match) return null;
	const amount = parseInt(match[1], 10);
	const unit = match[2];
	const ms = unit === 'm' ? amount * 60_000 : unit === 'h' ? amount * 3_600_000 : amount * 86_400_000;
	return new Date(Date.now() - ms).toISOString();
}

function tokenize(input: string): string[] {
	const tokens: string[] = [];
	let i = 0;
	while (i < input.length) {
		// skip whitespace
		while (i < input.length && input[i] === ' ') i++;
		if (i >= input.length) break;

		let token = '';
		while (i < input.length && input[i] !== ' ') {
			if (input[i] === '"') {
				// consume quoted string (including colon-prefixed)
				i++; // skip opening quote
				while (i < input.length && input[i] !== '"') {
					token += input[i];
					i++;
				}
				if (i < input.length) i++; // skip closing quote
			} else {
				token += input[i];
				i++;
			}
		}
		if (token) tokens.push(token);
	}
	return tokens;
}

/**
 * Parse DSL input into a SpanFilter.
 *
 * Supported syntax:
 *   kind:llm_call model:gpt-4 status:failed since:1h
 *   duration:>500ms duration:<2s tokens:>1000 cost:>0.01
 *   sort:duration order:desc
 *   bare words -> name_contains
 */
export function parseDsl(input: string): SpanFilter {
	const filter: SpanFilter = {};
	const tokens = tokenize(input.trim());

	for (const token of tokens) {
		const colonIdx = token.indexOf(':');
		if (colonIdx > 0) {
			const key = token.slice(0, colonIdx);
			const value = token.slice(colonIdx + 1);

			// Handle comparison operators for numeric fields
			// duration:>500ms, duration:<2s, duration:500ms-2000ms
			if (key === 'duration' && value) {
				const rangeMatch = value.match(/^(\d+(?:ms|s)?)-(\d+(?:ms|s)?)$/);
				if (rangeMatch) {
					const min = parseDurationMs(rangeMatch[1]);
					const max = parseDurationMs(rangeMatch[2]);
					if (min !== null) filter.duration_min = String(min);
					if (max !== null) filter.duration_max = String(max);
				} else if (value.startsWith('>')) {
					const ms = parseDurationMs(value.slice(1));
					if (ms !== null) filter.duration_min = String(ms);
				} else if (value.startsWith('<')) {
					const ms = parseDurationMs(value.slice(1));
					if (ms !== null) filter.duration_max = String(ms);
				} else {
					// exact-ish: treat as min
					const ms = parseDurationMs(value);
					if (ms !== null) filter.duration_min = String(ms);
				}
				continue;
			}

			if (key === 'tokens' && value) {
				if (value.startsWith('>')) {
					const n = parseInt(value.slice(1), 10);
					if (!isNaN(n)) filter.tokens_min = String(n);
				} else {
					const n = parseInt(value, 10);
					if (!isNaN(n)) filter.tokens_min = String(n);
				}
				continue;
			}

			if (key === 'cost' && value) {
				if (value.startsWith('>')) {
					const n = parseFloat(value.slice(1));
					if (!isNaN(n)) filter.cost_min = String(n);
				} else {
					const n = parseFloat(value);
					if (!isNaN(n)) filter.cost_min = String(n);
				}
				continue;
			}

			const filterKey = KEY_MAP[key];
			if (filterKey && value) {
				if (filterKey === 'since' || filterKey === 'until') {
					const iso = parseRelativeTime(value);
					filter[filterKey] = iso ?? value;
				} else {
					filter[filterKey] = value;
				}
			} else if (!filterKey && value) {
				// unknown key -- treat whole token as name search
				filter.name_contains = (filter.name_contains ? filter.name_contains + ' ' : '') + token;
			}
		} else {
			// bare text -> name_contains
			filter.name_contains = (filter.name_contains ? filter.name_contains + ' ' : '') + token;
		}
	}

	return filter;
}

/** Format ms back to a readable duration string */
function formatDurationDsl(ms: string): string {
	const n = parseInt(ms, 10);
	if (isNaN(n)) return ms;
	if (n >= 1000 && n % 1000 === 0) return `${n / 1000}s`;
	return `${n}ms`;
}

export function filterToDsl(filter: SpanFilter): string {
	const parts: string[] = [];

	for (const [filterKey, dslKey] of Object.entries(REVERSE_MAP)) {
		const value = filter[filterKey as keyof SpanFilter];
		if (value === undefined || value === null || value === '') continue;

		// Skip numeric fields handled separately below
		if (['duration_min', 'duration_max', 'tokens_min', 'cost_min'].includes(filterKey)) continue;

		if (typeof value === 'string' && value.includes(' ')) {
			parts.push(`${dslKey}:"${value}"`);
		} else {
			parts.push(`${dslKey}:${value}`);
		}
	}

	// Duration range
	if (filter.duration_min && filter.duration_max) {
		parts.push(`duration:${formatDurationDsl(filter.duration_min)}-${formatDurationDsl(filter.duration_max)}`);
	} else if (filter.duration_min) {
		parts.push(`duration:>${formatDurationDsl(filter.duration_min)}`);
	} else if (filter.duration_max) {
		parts.push(`duration:<${formatDurationDsl(filter.duration_max)}`);
	}

	// Tokens
	if (filter.tokens_min) {
		parts.push(`tokens:>${filter.tokens_min}`);
	}

	// Cost
	if (filter.cost_min) {
		parts.push(`cost:>${filter.cost_min}`);
	}

	return parts.join(' ');
}
