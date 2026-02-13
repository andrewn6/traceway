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
	until: 'until'
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
	until: 'until'
};

const RELATIVE_TIME_RE = /^(\d+)(m|h|d)$/;

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

export function parseDsl(input: string): SpanFilter {
	const filter: SpanFilter = {};
	const tokens = tokenize(input.trim());

	for (const token of tokens) {
		const colonIdx = token.indexOf(':');
		if (colonIdx > 0) {
			const key = token.slice(0, colonIdx);
			const value = token.slice(colonIdx + 1);
			const filterKey = KEY_MAP[key];
			if (filterKey && value) {
				if (filterKey === 'since' || filterKey === 'until') {
					const iso = parseRelativeTime(value);
					filter[filterKey] = iso ?? value;
				} else {
					filter[filterKey] = value;
				}
			} else if (!filterKey && value) {
				// unknown key — treat whole token as name search
				filter.name_contains = (filter.name_contains ? filter.name_contains + ' ' : '') + token;
			}
		} else {
			// bare text → name_contains
			filter.name_contains = (filter.name_contains ? filter.name_contains + ' ' : '') + token;
		}
	}

	return filter;
}

export function filterToDsl(filter: SpanFilter): string {
	const parts: string[] = [];

	for (const [filterKey, dslKey] of Object.entries(REVERSE_MAP)) {
		const value = filter[filterKey as keyof SpanFilter];
		if (value === undefined || value === '') continue;
		if (value.includes(' ')) {
			parts.push(`${dslKey}:"${value}"`);
		} else {
			parts.push(`${dslKey}:${value}`);
		}
	}

	return parts.join(' ');
}
