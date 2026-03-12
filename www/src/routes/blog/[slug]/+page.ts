import type { PageLoad } from './$types';
import { error } from '@sveltejs/kit';
import type { SvelteComponent } from 'svelte';

export interface ArticleData {
	title: string;
	description: string;
	date: string;
	author: string;
	slug: string;
	component: typeof SvelteComponent;
}

export const load: PageLoad = async ({ params }) => {
	const modules = import.meta.glob('/src/content/articles/*.md', { eager: true });

	const path = `/src/content/articles/${params.slug}.md`;
	const mod = modules[path] as
		| { default: typeof SvelteComponent; metadata: Omit<ArticleData, 'slug' | 'component'> }
		| undefined;

	if (!mod) {
		error(404, 'Article not found');
	}

	return {
		component: mod.default,
		title: mod.metadata.title,
		description: mod.metadata.description,
		date: mod.metadata.date,
		author: mod.metadata.author,
		slug: params.slug,
	};
};
