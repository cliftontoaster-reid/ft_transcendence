import { recommendationClient } from '$lib/server/grpc';
import { toSuggestion, type Suggestion } from '$lib/recommendations';
import { uuidToBytes } from '$lib/uuid';
import type { PageServerLoad } from './$types';

const LIMIT = 12;

export const load: PageServerLoad = async ({ locals }) => {
	const trendingPromise: Promise<Suggestion[] | null> = recommendationClient
		.getTrending({ limit: LIMIT })
		.then((response) => response.items.map(toSuggestion))
		.catch((error) => {
			console.error('RecommendationService.GetTrending failed', error);
			return null;
		});

	const forYouPromise: Promise<Suggestion[] | null> = locals.userId
		? recommendationClient
				.getForUser({
					userId: { value: uuidToBytes(locals.userId) },
					limit: LIMIT
				})
				.then((response) => response.items.map(toSuggestion))
				.catch((error) => {
					console.error('RecommendationService.GetForUser failed', error);
					return null;
				})
		: Promise.resolve([]);

	const [trending, forYou] = await Promise.all([trendingPromise, forYouPromise]);

	return {
		trending: trending ?? [],
		forYou: forYou ?? [],
		trendingError: trending === null,
		forYouError: forYou === null
	};
};
