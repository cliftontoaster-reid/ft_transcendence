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
  import type { Pathname } from '$app/types';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { locales, localizeHref } from '$lib/paraglide/runtime';
  import './layout.css';
  import favicon from '$lib/assets/favicon.svg';

  // Importation des icônes de ta librairie
  import { Home, Movie, LiveTv, Person } from '@material-symbols-svg/svelte/rounded';

  let { children } = $props();
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<!-- Barre de navigation -->
<header class="site-header">
  <nav class="nav-container">
    <a href="/" class="logo">Dyonisus</a>
    <div class="nav-links">
      <a href="/" class="nav-item" title="Accueil">
        <Home class="icon" />
      </a>
      <a href="/movies" class="nav-item" title="Films">
        <Movie class="icon" />
      </a>
      <a href="/watch_room" class="nav-item" title="Watch Room">
        <LiveTv class="icon" />
      </a>
      <a href="/profile" class="nav-item" title="Profil">
        <Person class="icon" />
      </a>
    </div>
  </nav>
</header>

<main class="site-main">
  {@render children()}
</main>

<footer class="site-footer">
  <p>© 2026 - Tous droits réservés</p>
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

  .nav-container {
    display: flex;
    justify-content: space-between;
    align-items: center;
    max-width: 1400px;
    margin: 0 auto;
  }

  .logo {
    font-weight: 800;
    font-size: 1.5rem;
    color: #8f015f;
    text-decoration: none;
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

  /* Contrôle de la taille et de la couleur des icônes de la librairie */
  :global(.icon) {
    width: 26px;
    height: 26px;
    fill: currentColor;
  }

  .site-main {
    min-height: 100vh;
    padding-top: 2rem; 
  }

  .site-footer {
    text-align: center;
    padding: 2rem;
    color: #71717a;
    font-size: 0.85rem;
  }
</style>