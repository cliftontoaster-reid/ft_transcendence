export type ChatEmoji = {
	glyph: string;
	keywords: string;
};

export const chatEmojis: ChatEmoji[] = [
	{ glyph: '😀', keywords: 'smile happy joy' },
	{ glyph: '😂', keywords: 'laugh tears funny' },
	{ glyph: '🥰', keywords: 'love affection heart in love' },
	{ glyph: '😎', keywords: 'cool sunglasses sun' },
	{ glyph: '😢', keywords: 'sad cry tear' },
	{ glyph: '👍', keywords: 'thumbs up yes ok agree' },
	{ glyph: '👎', keywords: 'thumbs down no disagree' },
	{ glyph: '🙏', keywords: 'thank you pray please' },
	{ glyph: '❤️', keywords: 'heart love red' },
	{ glyph: '🎉', keywords: 'party celebration confetti' },
	{ glyph: '🔥', keywords: 'fire hot trending' },
	{ glyph: '👋', keywords: 'wave hello goodbye' },
	{ glyph: '😁', keywords: 'grin teeth happy' },
	{ glyph: '😆', keywords: 'laugh squint' },
	{ glyph: '🤣', keywords: 'rolling laughing lol' },
	{ glyph: '😊', keywords: 'smile shy happy' },
	{ glyph: '😉', keywords: 'wink playful' },
	{ glyph: '😍', keywords: 'heart eyes love adore' },
	{ glyph: '😘', keywords: 'kiss love' },
	{ glyph: '😜', keywords: 'tongue playful silly' },
	{ glyph: '🤔', keywords: 'think ponder question' },
	{ glyph: '😴', keywords: 'sleep tired sleepy' },
	{ glyph: '😭', keywords: 'sob cry tears' },
	{ glyph: '😡', keywords: 'angry mad' },
	{ glyph: '🤯', keywords: 'mind blown shocked' },
	{ glyph: '🥳', keywords: 'party birthday celebration' },
	{ glyph: '😇', keywords: 'angel innocent halo' },
	{ glyph: '🤗', keywords: 'hug welcome' },
	{ glyph: '🙂', keywords: 'smile calm' },
	{ glyph: '🙃', keywords: 'upside down irony silly' },
	{ glyph: '👏', keywords: 'clap applause bravo' },
	{ glyph: '🙌', keywords: 'hands raised celebrate win' },
	{ glyph: '💪', keywords: 'strength strong muscle' },
	{ glyph: '💯', keywords: 'hundred perfect score' },
	{ glyph: '💖', keywords: 'sparkling heart love' },
	{ glyph: '💕', keywords: 'hearts love' },
	{ glyph: '💔', keywords: 'broken heart heartbreak sad' },
	{ glyph: '✨', keywords: 'sparkles magic shiny' },
	{ glyph: '🌟', keywords: 'star shine favorite' },
	{ glyph: '🚀', keywords: 'rocket launch space' }
];

function normalizeSearchText(text: string): string {
	return text
		.normalize('NFD')
		.replace(/[\u0300-\u036f]/g, '')
		.toLowerCase();
}

export function searchChatEmojis(query: string): ChatEmoji[] {
	const normalizedQuery = normalizeSearchText(query.trim());
	if (!normalizedQuery) return chatEmojis;

	return chatEmojis.filter((emoji) =>
		normalizeSearchText(`${emoji.glyph} ${emoji.keywords}`).includes(normalizedQuery)
	);
}
