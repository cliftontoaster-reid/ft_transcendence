<script lang="ts">
  import { page } from '$app/stores';
  import { base, resolve } from '$app/paths';
  import { colors } from '$lib/colors';

  // Get the slug from the URL parameters
  const slug = $derived($page.params.slug);
  const isRed = $derived(slug === 'red');

  // Find the matching color object from your list
  const colorData = $derived(colors.find(c => c.slug === slug));
</script>

<svelte:head>
  <title>{colorData?.text ?? 'Color Details'}</title>
</svelte:head>

<main>
  {#if isRed}
    <!-- Red Page: Plays the Rickroll Video -->
    <h1>You found the secret video!</h1>
    <div class="video-container">
      <video controls autoplay class="centered-video">
        <source src="{base}/rickroll.mp4" type="video/mp4" />
        Your browser does not support the video tag.
      </video>
    </div>
  {:else}
    <!-- Standard Page: Blue, Green, etc. -->
    <h1>{colorData?.text ?? 'Color Details'}</h1>
    <p>{colorData?.summary ?? 'Detailed information about this color.'}</p>
    
    <div 
      class="color-preview" 
      style="background-color: {colorData?.color ?? '#000'};"
    ></div>
  {/if}

  <div class="back-link-container">
    <a href={resolve('/')}>← Back to Home</a>
  </div>
</main>

<style>
  main {
    min-height: 100vh;
    padding: 50px 20px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    text-align: center;
    color: white;
    font-family: Arial, sans-serif;
  }

  .video-container {
    width: 100%;
    max-width: 800px;
    margin: 30px auto;
    display: flex;
    justify-content: center;
  }

  .centered-video {
    width: 100%;
    max-width: 800px;
    border-radius: 12px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.6);
  }

  .color-preview {
    width: 200px;
    height: 200px;
    margin: 30px auto;
    border-radius: 16px;
  }

  .back-link-container {
    margin-top: 40px;
  }

  a {
    color: white;
    text-decoration: underline;
    font-size: 16px;
  }
</style>