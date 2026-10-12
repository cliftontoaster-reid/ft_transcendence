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

  let startX = 0;
  let startY = 0;
  let isDragging = false;

  function handlePointerDown(event: PointerEvent) {
    startX = event.clientX;
    startY = event.clientY;
    isDragging = false;
  }

  function handlePointerMove(event: PointerEvent) {
    // If the pointer moves more than 5 pixels, consider it a drag/swipe
    const moveX = Math.abs(event.clientX - startX);
    const moveY = Math.abs(event.clientY - startY);
    if (moveX > 5 || moveY > 5) {
      isDragging = true;
    }
  }

  function handleClick(event: MouseEvent) {
    // If it was a drag, block the link navigation entirely
    if (isDragging) {
      event.preventDefault();
    }
  }
</script>

<a
  class="color-card"
  href={colorUrl}
  aria-label={`View summary for ${text}`}
  draggable="false"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onclick={handleClick}
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
    padding: 5px;
    box-sizing: border-box;
    color: inherit;
    text-decoration: none;
    -webkit-user-select: none;
    user-select: none;
    /* Prevents default browser touch actions from acting weirdly */
    touch-action: pan-y; 
  }

  .color-content {
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: 12px;
    color: white;

    display: flex;
    align-items: center;
    justify-content: center;

    box-sizing: border-box;
    transform: scale(1);
  }

  @media (hover: hover) {
    .color-card:hover .color-content {
      transition:
        transform 0.2s ease,
        box-shadow 0.2s ease,
        filter 0.2s ease;
      box-shadow:
        0 0 0 4px rgb(255 255 255 / 25%),
        0 12px 25px rgb(0 0 0 / 55%);
      filter: brightness(1.15);
    }

    .color-card:hover .color-content span {
      transition: transform 0.2s ease;
      transform: scale(1.08);
    }
  }
</style>