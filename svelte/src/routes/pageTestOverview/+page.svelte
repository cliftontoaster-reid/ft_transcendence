<script>
  import Overview from '$lib/components/Overview.svelte';
</script>

<section class="overview-section">
  <Overview 
    title="Titre" 
    imageUrl="https://picsum.photos/1280/720"
    playHref="/watch_room"
  />
</section>

<style>
  .overview-section {
    width: 100%;
    padding: 2rem 0;
  }
</style>