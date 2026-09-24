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

  let title = $state('Hello!');

  function handleClick(): void {
    title = 'You clicked the button!';
  }

const colors = [
  {color: '#ef4444', text: 'Red'},
  {color: '#3b82f6', text: 'Blue'},
  {color: '#22c55e', text: 'Green'},
  {color: '#eab308', text: 'Yellow'},
  {color: '#f97316', text: 'Orange'},
  {color: '#a855f7', text: 'Purple'},
  {color: '#ec4899', text: 'Pink'},
  {color: '#06b6d4', text: 'Cyan'},
  {color: '#14b8a6', text: 'Teal'},
  {color: '#84cc16', text: 'Lime'},
  {color: '#6366f1', text: 'Indigo'},
  {color: '#8b5cf6', text: 'Violet'},
  {color: '#0ea5e9', text: 'Sky Blue'},
  {color: '#64748b', text: 'Slate Gray'},
  {color: '#78716c', text: 'Stone Gray'},
  {color: '#92400e', text: 'Brown'},
  {color: '#f8fafc', text: 'White'},
  {color: '#1e293b', text: 'Navy'},
  {color: '#111827', text: 'Black'}
];

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
        {#each colors as { color, text } (color)}
          <Color {color} {text} />
        {/each}
      </Carousel>
    {/if}
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    background: linear-gradient(to bottom, black 0%, black 5%, red 8%, red 12%, black 15%);
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
    max-width: 1500px;
    margin: 50px auto 0;
  }
</style>