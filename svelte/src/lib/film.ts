export type Film = {
	slug: string;
	title: string;
	src: string;
	poster: string;
};

export const films: Film[] = [
	{
		slug: 'never-gonna-give-you-up',
		title: 'Never Gonna Give You Up',
		src: 'https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4',
		poster: 'https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=800'
	},
	{
		slug: 'elephants-dream',
		title: "Elephant's Dream",
		src: 'https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4',
		poster: 'https://picsum.photos/1280/720'
	}
];

export const defaultFilm = films[0];
export const findFilm = (slug: string) => films.find((f) => f.slug === slug);