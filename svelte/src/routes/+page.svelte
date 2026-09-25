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
  import Carousel from 'svelte-carousel';
  import Color from '$lib/Color.svelte';
  import { colors } from '$lib/colors';

  let title = $state('Hello!');

  function handleClick(): void {
    title = 'You clicked the button!';
  }
</script>

<svelte:head>
  <title>Gradient Background</title>
</svelte:head>

<main>
  <h1>{title}</h1>

  <button type="button" onclick={handleClick}>
    Click me
  </button>

  <div class="carousel-container">
    {#if browser}
      <Carousel particlesToShow={7} particlesToScroll={4}>
        {#each colors as color (color.slug)}
          <Color {...color} />
        {/each}
      </Carousel>
    {/if}
  </div>
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
  }

  .carousel-container {
    width: 1500px;
    max-width: 100%;
    margin: 50px auto 0;
    background: transparent;
    padding: 30px 20px;
    box-sizing: border-box;
    background: transparent;
    overflow: visible;
  }

  .carousel-container :global(.carousel),
  .carousel-container :global(.carousel__container),
  .carousel-container :global(.carousel__track),
  .carousel-container :global(.carousel__viewport),
  .carousel-container :global(.slick-list),
  .carousel-container :global(.slick-track) {
    overflow: visible !important;
  }

  .carousel-container :global(*) {
    overflow: visible;
  }  
</style>