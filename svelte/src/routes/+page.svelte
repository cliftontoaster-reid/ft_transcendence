<script lang="ts">
  import { browser } from '$app/environment';
  import Carousel from 'svelte-carousel';
  import Color from '$lib/Color.svelte';
  import SearchBar from '$lib/SearchBar.svelte';
  import { colors } from '$lib/colors';

  let title = $state('Hello!');
  let searchQuery = $state('');

  let innerWidth = $state(browser ? window.innerWidth : 1200);

  let particlesToShow = $derived.by(() => {
    if (innerWidth < 200) return 1;
    if (innerWidth < 450) return 3;
    if (innerWidth < 1024) return 4;
    return 7;
  });

  let particlesToScroll = $derived(Math.max(1, Math.floor(particlesToShow)));

  // Filter colors dynamically based on search input
  let filteredColors = $derived(
    colors.filter(c => 
      c.text.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.summary.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  // Check if the user is actively searching
  let isSearching = $derived(searchQuery.trim().length > 0);

  function handleClick(): void {
    title = 'You clicked the button!';
  }
</script>

<svelte:window bind:innerWidth />

<svelte:head>
  <title>Gradient Background</title>
</svelte:head>

<main>
  <h1>{title}</h1>

  <button type="button" onclick={handleClick}>
    Click me
  </button>

  <SearchBar bind:searchQuery />

  {#if !isSearching}
    <!-- 1. Default View: Carousel -->
    <div class="carousel-container">
      {#if browser}
        {#key particlesToShow}
        <Carousel {particlesToShow} {particlesToScroll} dots={false} swiping={true} arrows={false}>
          {#each colors as color (color.slug)}
            <Color {...color} />
          {/each}
        </Carousel>
        {/key}
      {/if}
    </div>
  {:else}
    <!-- 2. Search View: Grid matching the carousel's exact column count -->
    <div class="grid-container" style="grid-template-columns: repeat({particlesToShow}, 1fr);">
      {#each filteredColors as color (color.slug)}
        <Color {...color} />
      {/each}
    </div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    background: linear-gradient(
      to bottom,
      black 0%,
      black 5%,
      red 8%,
      red 12%,
      black 15%
    );
    color: white;
    font-family: Arial, sans-serif;
  }

  main {
    min-height: 100vh;
    padding: 50px 20px;
    box-sizing: border-box;
    text-align: center;
  }

  button {
    padding: 10px 20px;
    font-size: 16px;
    cursor: pointer;
    margin-bottom: 20px;
  }

  .carousel-container {
    width: 100%;
    max-width: 1500px;
    margin: 30px auto 0;
    padding: 30px 20px;
    box-sizing: border-box;
    background: transparent;
  }

  .grid-container {
    display: grid;
    gap: 10px;
    width: 100%;
    max-width: 1500px;
    margin: 30px auto 0;
    padding: 30px 20px;
    box-sizing: border-box;
  }
</style>