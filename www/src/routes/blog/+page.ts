import type { PageLoad } from './$types';

export interface ArticleMeta {
	title: string;
	description: string;
	date: string;
	author: string;
	slug: string;
}

export const load: PageLoad = async () => {
	const modules = import.meta.glob('/src/content/articles/*.md', { eager: true });

	const articles: ArticleMeta[] = Object.entries(modules).map(([path, mod]) => {
		const slug = path.split('/').pop()?.replace('.md', '') ?? '';
		const metadata = (mod as { metadata: Omit<ArticleMeta, 'slug'> }).metadata;
		return { ...metadata, slug };
	});

	// Sort by date descending
	articles.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());

	return { articles };
};
