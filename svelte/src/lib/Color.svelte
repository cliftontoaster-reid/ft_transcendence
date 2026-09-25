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
    <span>{text}</span>
  </div>
</a>

<style>
  .color-card {
    display: block;
    width: 100%;
    height: 150px;
    padding: 5px;
    box-sizing: border-box;
    color: inherit;
    text-decoration: none;
  }

  .color-content {
    width: 100%;
    height: 140px;
    border-radius: 12px;
    color: white;

    display: flex;
    align-items: center;
    justify-content: center;

    box-sizing: border-box;
    transform: scale(1);
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease,
      filter 0.2s ease;
  }

  .color-content span {
    transition: transform 0.2s ease;
  }

  .color-card:hover .color-content {
    /*
     * This stays inside the carousel's available area.
     * The shadow and brightness create the expanded appearance
     * without being clipped.
     */
    box-shadow:
      0 0 0 4px rgb(255 255 255 / 25%),
      0 12px 25px rgb(0 0 0 / 55%);
    filter: brightness(1.15);
  }

  .color-card:hover .color-content span {
    transform: scale(1.08);
  }
</style>
