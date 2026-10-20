<script lang="ts">
  import { browser } from '$app/environment';
  import Carousel from 'svelte-carousel';
  import ColorCard from '$lib/Color.svelte';
  import type { Color } from '$lib/colors';

  let {
    title,
    items,
    particlesToShow,
    particlesToScroll,
  }: {
    title: string;
    items: Color[];
    particlesToShow: number;
    particlesToScroll: number;
  } = $props();
</script>

<section class="carousel-section" aria-label={title}>
  <h2 class="carousel-title">{title}</h2>

  {#if browser}
    <div class="carousel-container">
      {#key particlesToShow}
        <Carousel
          {particlesToShow}
          {particlesToScroll}
          dots={false}
          swiping={true}
          arrows={false}
        >
          {#each items as color (color.slug)}
            <ColorCard {...color} />
          {/each}
        </Carousel>
      {/key}
    </div>
  {/if}
</section>

<style>
  .carousel-section {
    width: 100%;
    padding: 0 0 2rem;
  }

  .carousel-title {
    margin: 0;
    color: white;
    font-size: 1.25rem;
  }

  .carousel-container {
    width: 100%;
    margin: 1rem auto 0;
    padding: 1rem 0;
    box-sizing: border-box;
  }
</style>