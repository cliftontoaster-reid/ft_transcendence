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
  import './layout.css';
  import favicon from '$lib/assets/favicon.svg';

  // Importation librarie for icons
  import { Home, Movie, LiveTv, Person } from '@material-symbols-svg/svelte/rounded';

  let { children } = $props();
  const searchQuery: SearchQueryState = $state({ value: '' });
  setContext(SEARCH_QUERY_CONTEXT, searchQuery);
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<!-- Navbar -->
<header class="site-header">
  <nav class="nav-container">
    <a href="/" class="logo">Dyonisus</a>
    <div class="nav-search">
      <SearchBar bind:searchQuery={searchQuery.value} />
    </div>
    <div class="nav-links">
      <a href="/" class="nav-item" title="Home">
        <Home class="icon" />
      </a>
      <a href="/movies" class="nav-item" title="Movies">
        <Movie class="icon" />
      </a>
      <a href="/watch_room" class="nav-item" title="Watch Room">
        <LiveTv class="icon" />
      </a>
      <a href="/profile" class="nav-item" title="Profile">
        <Person class="icon" />
      </a>
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
    background: linear-gradient(90deg, transparent, #ff7a1a 15%, #ffad5c 50%, #ff7a1a 85%, transparent);
    box-shadow: 0 0 8px 2px rgb(255 122 26 / 65%), 0 0 24px 6px rgb(255 122 26 / 30%);
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
    background: linear-gradient(110deg, #83367a 0%, #ff7a1a 38%, #d45e14 78%, #ffbb5c 100%);
    background-clip: text;
    -webkit-text-fill-color: transparent;
    filter: drop-shadow(0 0 6px rgb(255 122 26 / 25%));
    text-decoration: none;
    transition: filter 0.25s ease, letter-spacing 0.25s ease;
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
    color: #ffffff;
    text-decoration: none;
    transition: opacity 0.2s, color 0.2s;
  }

  .nav-item {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem;
  }

  .nav-links a:hover {
    color: #8f015f;
    opacity: 1;
  }

  /* setting icons */
  :global(.icon) {
    width: 26px;
    height: 26px;
    fill: currentColor;
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