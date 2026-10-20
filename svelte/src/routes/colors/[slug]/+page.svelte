<script lang="ts">
  import { page } from '$app/stores';
  import { resolve } from '$app/paths';
  import { colors } from '$lib/colors';

  const slug = $derived($page.params.slug);
  const colorItem = $derived(colors.find(c => c.slug === slug));
  const backUrl = $derived(resolve('/'));
</script>

<div class="top-bar">
  <a href={backUrl} class="back-link">← Back to colors</a>
</div>

<main>
  {#if colorItem}
    <div class="color-preview" style={`background-color: ${colorItem.color}`}></div>
    
    <h1 class="color-title">{colorItem.text}</h1>
    <p class="color-summary">{colorItem.summary}</p>
  {:else}
    <h1>Color not found</h1>
    <a href={backUrl} class="back-link">Back to colors</a>
  {/if}
</main>

<style>
  .top-bar {
    position: absolute;
    top: 25px;
    left: 25px;
    z-index: 10;
  }

  .back-link {
    color: white;
    text-decoration: none;
    font-weight: bold;
    padding: 8px 16px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    transition: background 0.2s ease;
  }

  .back-link:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  main {
    min-height: 100vh;
    padding: 90px 20px 40px;
    box-sizing: border-box;
    max-width: 900px;
    margin: 0 auto;
    text-align: center;
    color: white;
    font-family: Arial, sans-serif;
  }

  .color-preview {
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: 16px;
    margin-bottom: 30px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.6);
  }

  .color-title {
    font-size: 2.5rem;
    margin: 0 0 15px 0;
  }

  .color-summary {
    font-size: 1.15rem;
    line-height: 1.6;
    color: rgba(255, 255, 255, 0.85);
    max-width: 700px;
    margin: 0 auto;
  }
</style>