<script lang="ts">
  let { color, text }: { color: string; text: string } = $props();
</script>

<button
  type="button"
  class="color-card"
  data-color-text={text}
>
  <div
    class="color-content"
    style={`background-color: ${color}`}
  >
    {text}
  </div>
</button>

<style>
  .color-card {
    position: relative;
    width: 100%;
    min-height: 150px;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
    overflow: visible;
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

    transition: 
      transform 0.2s ease,
      box-shadow 0.2s ease;
    
      transform-origin: center;
  }

  .color-content:hover {
  transform: scale(1.08);
  z-index: 10;
  box-shadow: 0 8px 20px rgb(0 0 0 / 35%);
  }
</style>
