import type { Recommendation } from '$lib/proto/dionysus/recommendations/v1/recommendations_pb';
import { bytesToUuid } from '$lib/uuid';

export type Suggestion = {
	id: string;
	title: string;
	reason: string;
};

export function toSuggestion(item: Recommendation): Suggestion {
	return {
		id: bytesToUuid(item.mediaId?.value ?? new Uint8Array()),
		title: item.title.trim() || 'Untitled',
		reason: item.reason
	};
}
