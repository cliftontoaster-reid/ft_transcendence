<script lang="ts">
  let {
    color,
    text,
    onclick
  }: {
    color: string;
    text: string;
    onclick: () => void;
  } = $props();
</script>

<button
  type="button"
  class="color-card"
  onclick={onclick}
>
  <div class="color-content" style={`background-color: ${color}`}>
    {text}
  </div>
</button>

<style>
  .color-card {
    width: 100%;
    min-height: 150px;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .color-content {
    width: 98%;
    height: 140px;
    box-sizing: border-box;
    border-radius: 12px;
    color: white;

    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>