<script lang="ts">
  let { color, text }: { color: string; text: string } = $props();
</script>

<div class="color-card" style={`background-color: ${color}`}>
  <h2>{text}</h2>
  <p>{color}</p>
</div>

<style>
  .color-card {
    min-height: 150px;
    margin: 0px;
    padding: 20px;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    color: white;
  }

  h2,
  p {
    margin: 5px;
  }
</style>
