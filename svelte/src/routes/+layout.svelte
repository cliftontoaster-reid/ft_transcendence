<!--
  +layout.svelte - svelte
  Copyright (C) 2026 Clifton Toaster Reid

  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU Affero General Public License as
  published by the Free Software Foundation, either version 3 of the
  License, or (at your option) any later version.

  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
  GNU Affero General Public License for more details.

  You should have received a copy of the GNU Affero General Public License
  along with this program. If not, see <https://www.gnu.org/licenses/>.
 -->

<script lang="ts">
	import { setContext } from 'svelte';
	import type { Pathname } from '$app/types';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import SearchBar from '$lib/SearchBar.svelte';
	import { SEARCH_QUERY_CONTEXT, type SearchQueryState } from '$lib/search-context';
	import { locales, localizeHref } from '$lib/paraglide/runtime';
	import * as m from '$lib/paraglide/messages';
	import './layout.scss';
	import favicon from '$lib/assets/favicon.svg';
	import '@fontsource/hanken-grotesk/600.css';

	// Importation librarie for icons
	import { Home, Movie, LiveTv, Person, Settings } from '@material-symbols-svg/svelte/rounded';

	let { children } = $props();
	const searchQuery: SearchQueryState = $state({ value: '' });
	setContext(SEARCH_QUERY_CONTEXT, searchQuery);

	let menuOpen = $state(false);
	let menuRef: HTMLDivElement | undefined = $state();

	function handleWindowClick(e: MouseEvent) {
		if (menuOpen && menuRef && !menuRef.contains(e.target as Node)) {
			menuOpen = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && menuOpen) {
			menuOpen = false;
		}
	}
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleKeydown} />

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<!-- Navbar -->
<header class="site-header">
	<nav class="nav-container">
		<a href="/" class="logo" data-sveltekit-reload>Dyonisus</a>
		<div class="nav-search">
			<SearchBar bind:searchQuery={searchQuery.value} />
		</div>
	<div class="nav-links">
		<a href="/" class="nav-item" data-sveltekit-reload title={m.nav_home()}>
			<Home class="icon" />
		</a>
		<a href="/movies" class="nav-item" title={m.nav_movies()}>
			<Movie class="icon" />
		</a>
		<a href="/watch_room" class="nav-item" title={m.nav_watch_room()}>
			<LiveTv class="icon" />
		</a>
		<a href="/profile" class="nav-item" title={m.nav_profile()}>
			<Person class="icon" />
		</a>
		<div class="settings-menu" bind:this={menuRef}>
			<button
				type="button"
				class="nav-item settings-btn"
				title={m.nav_settings()}
				aria-label={m.nav_settings()}
				aria-expanded={menuOpen}
				aria-haspopup="menu"
				onclick={() => (menuOpen = !menuOpen)}
			>
				<Settings class="icon" />
			</button>

			{#if menuOpen}
				<div class="settings-dropdown" role="menu" aria-label={m.nav_language()}>
					<span class="dropdown-title">{m.nav_language()}</span>
					{#each locales as locale (locale)}
						<a
							href={resolve(localizeHref(page.url.pathname, { locale }) as Pathname)}
							role="menuitem"
							class="dropdown-item"
							onclick={() => (menuOpen = false)}
						>
							{locale}
						</a>
					{/each}
				</div>
			{/if}
		</div>
		<a href="/login" class="login-link">{m.nav_login()}</a>
	</div>
	</nav>
</header>

<main class="site-main">
	{@render children()}
</main>

<footer class="site-footer">
	<p>© 2026 - All rights reserved.</p>
</footer>

<div style="display:none">
	{#each locales as locale (locale)}
		<a href={resolve(localizeHref(page.url.pathname, { locale }) as Pathname)}>{locale}</a>
	{/each}
</div>

<style>
	.site-header {
		position: relative;
		top: 0;
		left: 0;
		width: 100%;
		z-index: 1000;
		background: linear-gradient(to bottom, rgba(0, 0, 0, 0.8));
		padding: 1.2rem 2rem;
		box-sizing: border-box;
	}

	.site-header::after {
		content: '';
		position: absolute;
		right: 0;
		bottom: 0;
		left: 0;
		height: 2px;
		background: linear-gradient(
			90deg,
			transparent,
			#ff7a1a 15%,
			#ffad5c 50%,
			#ff7a1a 85%,
			transparent
		);
		box-shadow:
			0 0 8px 2px rgb(255 122 26 / 65%),
			0 0 24px 6px rgb(255 122 26 / 30%);
		opacity: 0.3;
		transition: opacity 0.25s ease;
		pointer-events: none;
	}

	.site-header:hover::after,
	.site-header:focus-within::after {
		opacity: 1;
	}

	.nav-container {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.nav-search {
		flex: 1;
		max-width: 400px;
		margin: 0 auto;
	}

	.nav-search :global(.search-container) {
		max-width: none;
		margin: 0;
	}

	.logo {
		display: inline-block;
		font-weight: 800;
		font-size: 1.5rem;
		letter-spacing: 0.03em;
		background: linear-gradient(110deg, #fab183 0%, #ff7a1a 38%, #d45e14 78%, #ffbb5c 100%);
		background-clip: text;
		-webkit-text-fill-color: transparent;
		filter: drop-shadow(0 0 6px rgb(255 122 26 / 25%));
		text-decoration: none;
		transition:
			filter 0.25s ease,
			letter-spacing 0.25s ease;
	}

	.logo:hover,
	.logo:focus-visible {
		filter: drop-shadow(0 0 5px rgb(255 173 92 / 75%)) drop-shadow(0 0 14px rgb(212 20 122 / 55%));
		letter-spacing: 0.06em;
	}

	.nav-links {
		display: flex;
		gap: 1.5rem;
		align-items: center;
	}

	.nav-links a {
		color: #ffad5c;
		text-decoration: none;
		transition:
			color 0.2s,
			filter 0.2s;
	}

	.nav-item {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.5rem;
	}

	.nav-links a:hover,
	.nav-links a:focus-visible {
		color: #ff7a1a;
		filter: drop-shadow(0 0 5px rgb(255 173 92 / 75%)) drop-shadow(0 0 14px rgb(255 122 26 / 55%));
	}

	.login-link {
		padding: 0.5rem 0.8rem;
		border: 1px solid rgb(255 173 92 / 50%);
		border-radius: 999px;
		font-weight: 700;
	}

	/* setting icons */
	:global(.icon) {
		width: 26px;
		height: 26px;
		fill: currentColor;
	}


	.settings-menu {
		position: relative;
		display: inline-flex;
		align-items: center;
	}

	.settings-btn {
		background: transparent;
		border: none;
		color: #ffad5c;
		cursor: pointer;
		transition:
			color 0.2s,
			filter 0.2s;
	}

	.settings-btn:hover,
	.settings-btn:focus-visible {
		color: #ff7a1a;
		filter: drop-shadow(0 0 5px rgb(255 173 92 / 75%)) drop-shadow(0 0 14px rgb(255 122 26 / 55%));
	}

	.settings-dropdown {
		position: absolute;
		top: calc(100% + 0.5rem);
		right: 0;
		min-width: 150px;
		padding: 0.4rem;
		border-radius: 0.6rem;
		background: #141416;
		border: 1px solid rgb(255 173 92 / 35%);
		box-shadow: 0 10px 28px rgb(0 0 0 / 65%);
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		z-index: 1100;
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
	}

	.dropdown-title {
		padding: 0.25rem 0.6rem;
		font-size: 0.72rem;
		letter-spacing: 0.06em;
		color: #a1a1aa;
		font-family: 'Hanken Grotesk', sans-serif;
	}

	.dropdown-item {
		padding: 0.45rem 0.6rem;
		border-radius: 0.4rem;
		font-size: 0.9rem;
		color: #ffad5c;
		font-family: 'Hanken Grotesk', sans-serif;
	}

	.dropdown-item:hover,
	.dropdown-item:focus-visible {
		background: rgb(255 122 26 / 14%);
	}

	.site-main {
		min-height: 100vh;
		max-width: 1600px;
		margin: 0 auto;
		padding: 0 1.5rem 2rem;
		box-sizing: border-box;
	}

	.site-footer {
		text-align: center;
		max-width: 1400px;
		margin: 0 auto;
		padding: 2rem;
		color: #71717a;
		font-size: 0.85rem;
		box-sizing: border-box;
	}

	@media (max-width: 640px) {
		.nav-container {
			flex-wrap: wrap;
			gap: 0.75rem;
		}

		.nav-search {
			order: 1;
			flex-basis: 100%;
			max-width: none;
		}

		.nav-links {
			gap: 0.5rem;
		}
	}
</style>
