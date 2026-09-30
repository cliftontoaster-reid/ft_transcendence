<script lang="ts">
  import { resolve } from '$app/paths';

  let { data } = $props();

  const color = $derived(data.color);
</script>

<svelte:head>
  <title>{color.text} — Color Summary</title>
</svelte:head>

<main>
  <a href={resolve('/')}>← Back to colors</a>

  <section class="summary">
    <div
      class="color-preview"
      style={`background-color: ${color.color}`}
    ></div>

    <h1>{color.text}</h1>

    <p>{color.summary}</p>

    <p>
      Hex value: <code>{color.color}</code>
    </p>
  </section>
</main>

<style>
  main {
    min-height: 100vh;
    padding: 50px 20px;
    box-sizing: border-box;
    color: white;
    font-family: Arial, sans-serif;
  }

  a {
    color: white;
  }

  .summary {
    max-width: 700px;
    margin: 60px auto;
    text-align: center;
  }

  .color-preview {
    width: 100%;
    height: 220px;
    border-radius: 16px;
    margin-bottom: 30px;
  }

  h1 {
    font-size: 3rem;
  }

  p {
    font-size: 1.2rem;
    line-height: 1.6;
  }

  code {
    padding: 4px 8px;
    border-radius: 5px;
    background: rgb(255 255 255 / 15%);
  }
</style>
