<script lang="ts">
	import { Add, PlayArrow } from '@material-symbols-svg/svelte';
	import Overview from '$lib/components/movies/Overview.svelte';

	interface DemoMovie {
		title: string;
		description: string;
		tags: string[];
		logo: URL;
		backdrop: URL;
		mobileBackdrop: URL;
		video: URL;
	}

	const stockVideo = new URL(
		'https://archive.org/download/spider-man-brand-new-day-trailer-2-2k-prores/SpiderManBrandNewDay_Trailer-2_2K_51_prores.mp4'
	);

	const movies: DemoMovie[] = [
		{
			title: 'Spider-Man: Brand New Day',
			description:
				'Peter Parker returns for a new chapter as Spider-Man, facing a dangerous threat and the responsibility that comes with protecting New York.',
			tags: ['2026', '13+', 'Action', 'Superhero'],
			logo: new URL(
				'https://dummyimage.com/900x260/000000/ffffff.png&text=SPIDER-MAN%3A+BRAND+NEW+DAY'
			),
			backdrop: new URL(
				'https://images.unsplash.com/photo-1519608487953-e999c86e7455?auto=format&fit=crop&w=2400&q=85'
			),
			mobileBackdrop: new URL(
				'https://images.unsplash.com/photo-1519501025264-65ba15a82390?auto=format&fit=crop&w=1200&q=85'
			),
			video: stockVideo
		}
	];

	let selected = $state(movies[0]);
	let message = $state('');

	function playMovie() {
		message = `Playing ${selected.title}`;
	}

	function addMovie() {
		message = `${selected.title} added to your list`;
	}
</script>

<svelte:head>
	<title>Spider-Man: Brand New Day</title>
	<meta
		name="description"
		content="Watch the trailer for Spider-Man: Brand New Day."
	/>
</svelte:head>

<main>
	<Overview
		logo={selected.logo}
		logo_msg={`${selected.title} logo`}
		tags={selected.tags}
		description={selected.description}
		desktop={{ image_src: selected.backdrop, video_src: selected.video }}
		mobile={{ image_src: selected.mobileBackdrop, video_src: selected.video }}
		buttons={[
			{ text: 'Play', icon: PlayArrow, onClick: playMovie },
			{ text: 'My List', icon: Add, onClick: addMovie }
		]}
	/>

	<section class="movie-details" aria-label="Movie details">
		<div>
			<p class="eyebrow">Featured title</p>
			<h1>{selected.title}</h1>
			{#if message}
				<p class="message" aria-live="polite">{message}</p>
			{/if}
		</div>
	</section>
</main>

<style lang="scss">
	:global(body) {
		margin: 0;
		background: #000;
		color: #fff;
	}

	main {
		width: min(1200px, calc(100% - 2rem));
		margin: 3rem auto;
		overflow: hidden;
		border: 1px solid #303030;
		border-radius: 1rem;
		background: #000;
		box-shadow: 0 1.5rem 4rem rgba(0, 0, 0, 0.45);
	}

	:global(.overview-container) {
		height: clamp(360px, 42vw, 500px);
		min-height: 0 !important;
	}

	:global(.overview-container .media-container) {
		height: 100%;
		min-height: 0 !important;
		overflow: hidden;
	}

	:global(.overview-container .media-asset) {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.movie-details {
		display: flex;
		align-items: center;
		gap: 2rem;
		padding: 1.5rem 5vw 3rem;
		background: #000;
	}

	.eyebrow {
		margin: 0 0 0.4rem;
		color: #a3a3a3;
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	h1 {
		margin: 0;
		font-size: clamp(1.5rem, 3vw, 2.5rem);
	}

	.message {
		margin: 0.5rem 0 0;
		color: #b3b3b3;
	}

	@media (max-width: 600px) {
		main {
			width: calc(100% - 1rem);
			margin: 0.5rem auto;
			border-radius: 0.75rem;
		}

		:global(.overview-container) {
			height: 420px;
		}

		.movie-details {
			align-items: flex-start;
			flex-direction: column;
			padding: 1.25rem;
		}
	}
</style>
