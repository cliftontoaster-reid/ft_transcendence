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

  let startTime = 0;
  let startX = 0;
  let startY = 0;
  let isDragging = false;

  function handlePointerDown(event: PointerEvent) {
    startTime = Date.now();
    startX = event.clientX;
    startY = event.clientY;
    isDragging = false;
  }

  function handlePointerMove(event: PointerEvent) {
    const moveX = Math.abs(event.clientX - startX);
    const moveY = Math.abs(event.clientY - startY);
    
    if (moveX > 15 || moveY > 15) {
      isDragging = true;
    }
  }

  function handleClick(event: MouseEvent) {
    const elapsedTime = Date.now() - startTime;

    // Only block click if it was an intentional long drag/swipe
    if (isDragging && elapsedTime > 250) {
      event.preventDefault();
      event.stopPropagation();
    }
    
    isDragging = false;
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