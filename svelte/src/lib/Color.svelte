<script lang="ts">
  let { color, text }: { color: string; text: string } = $props();
</script>

<div class="color-card">
  <div class="color-content" style={`background-color: ${color}`}>
    {text}
  </div>
</div>

<style>
  .color-card {
    width: 100%;
    min-height: 150px;
    margin: 0;
    box-sizing: border-box;
    background: transparent;
  }

  .color-content {
    width: 95%;
    height: 140px;
    box-sizing: border-box;
    border-radius: 12px;
    color: negative;

    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>