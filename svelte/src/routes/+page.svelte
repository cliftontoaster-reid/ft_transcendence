<!--
  +page.svelte - svelte
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
  import { browser } from '$app/environment';
  import { getContext } from 'svelte';
  import Color from '$lib/components/CarouselPartical.svelte';
  import CarouselRow from '$lib/components/CarouselRow.svelte';
  import Chat from '$lib/components/chat/Chat.svelte';
  import Overview from '$lib/components/Overview.svelte';
  import { colors } from '$lib/colors';
  import { SEARCH_QUERY_CONTEXT, type SearchQueryState } from '$lib/search-context';
  import '$lib/components/chat/css/home.scss';

  const carouselCategories = [
    {
      title: 'Warm colors',
      slugs: ['red', 'orange', 'yellow', 'crimson', 'coral', 'peach', 'gold', 'amber'],
    },
    {
      title: 'Cool colors',
      slugs: ['blue', 'cyan', 'teal', 'sky-blue', 'turquoise', 'navy', 'indigo'],
    },
    {
      title: 'Green shades',
      slugs: ['green', 'lime', 'olive', 'mint', 'emerald', 'forest-green'],
    },
    {
      title: 'Neutral colors',
      slugs: ['slate-gray', 'stone-gray', 'brown', 'white', 'black', 'beige', 'cream', 'silver'],
    },
    {
      title: 'Purple and pink',
      slugs: ['purple', 'pink', 'violet', 'lavender', 'magenta'],
    },
  ].map((category) => ({
    title: category.title,
    items: colors.filter((color) => category.slugs.includes(color.slug)),
  }));
  const minCategoryItemCount = Math.min(...carouselCategories.map(({ items }) => items.length));

  const searchQuery = getContext<SearchQueryState>(SEARCH_QUERY_CONTEXT);
  let directMessageContact = $state<string | null>(null);
  let innerWidth = $state(browser ? window.innerWidth : 1200);

  let responsiveParticlesToShow = $derived.by(() => {
    if (innerWidth < 200) return 1;
    if (innerWidth < 450) return 3;
    if (innerWidth < 1024) return 4;
    return 7;
  });

  let particlesToShow = $derived(Math.min(responsiveParticlesToShow, minCategoryItemCount));
  let particlesToScroll = $derived(Math.max(1, Math.floor(particlesToShow)));
  let filteredColors = $derived(
    colors.filter(
      (color) =>
      color.title.toLowerCase().includes(searchQuery.value.toLowerCase()),
    ),
  );
  let isSearching = $derived(searchQuery.value.trim().length > 0);
</script>

<svelte:window bind:innerWidth />

<div class="home-layout">
  {#if !isSearching}
    <div class="overview-chat-container">
      <div class="home-content">
        <section class="overview-section">
          <Overview
            title="Titre"
            imageUrl="https://picsum.photos/1280/720"
            playHref="/watch_room"
          />
        </section>
      </div>

      <aside class="chat-sidebar">
        <Chat
          variant="home"
          {directMessageContact}
          onBackToGlobalChat={() => (directMessageContact = null)}
          onDirectMessage={(contact) => (directMessageContact = contact)}
        />
      </aside>
    </div>
  {/if}

  <section class="browse-section" aria-label="Browse colors">
    {#if isSearching}
      <div
        class="grid-container"
        style={`grid-template-columns: repeat(${particlesToShow}, minmax(0, 1fr));`}
      >
        {#each filteredColors as color (color.slug)}
          <Color {...color} />
        {/each}
      </div>
    {:else}
      {#each carouselCategories as category (category.title)}
        <CarouselRow
          title={category.title}
          items={category.items}
          {particlesToShow}
          {particlesToScroll}
        />
      {/each}
    {/if}
  </section>
</div>
