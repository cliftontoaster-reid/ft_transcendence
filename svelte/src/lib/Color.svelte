<script lang="ts">
  import { resolve } from '$app/paths';

  let {
    slug,
    color,
    text
  }: {
    slug: string;
    color: string;
    text: string;
  } = $props();

  const colorUrl = $derived(resolve(`/colors/${slug}`));
</script>

<a
  class="color-card"
  href={colorUrl}
  aria-label={`View summary for ${text}`}
>
  <div
    class="color-content"
    style={`background-color: ${color}`}
  >
    {text}
  </div>
</a>

<style>
  .color-card {
    display: block;
    position: relative;
    width: 100%;
    min-height: 150px;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-decoration: none;
    cursor: pointer;
    overflow: visible;
    z-index: 1;
  }

  .color-card:hover {
    z-index: 100;
  }

  .color-content {
    position: relative;
    width: 98%;
    height: 140px;
    box-sizing: border-box;
    border-radius: 12px;
    color: white;

    display: flex;
    align-items: center;
    justify-content: center;

    transform-origin: center;
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease;
  }

  .color-card:hover .color-content {
    transform: scale(1.08);
    box-shadow: 0 8px 20px rgb(0 0 0 / 35%);
  }
</style>
