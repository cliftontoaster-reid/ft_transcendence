import { error } from '@sveltejs/kit';
import { defaultFilm, findFilm } from '$lib/film';
import type { PageLoad } from './$types';

export const load: PageLoad = ({ url }) => {
	const slug = url.searchParams.get('film');
	if (!slug) return { film: defaultFilm };

	const film = findFilm(slug);
	if (!film) error(404, 'Film not found');
	return { film };
};
