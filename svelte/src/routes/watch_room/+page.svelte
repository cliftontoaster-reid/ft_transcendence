<script lang="ts">
   import Chat from '$lib/components/chat/Chat.svelte';
   import VideoPlayer from '$lib/components/videoPlayer/videoPlayer.svelte';
   import '$lib/components/chat/css/watch-room.scss';
   import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
  const film = $derived(data.film);

  function handleBack() {
    // Permet de retourner à la page précédente
    history.back();
  }
</script>

<svelte:head>
  <title>{film.title} - Lecture</title>
</svelte:head>

<main class="watch-page">
  <!-- Bouton de retour en haut de page -->
  <button class="back-btn" onclick={handleBack}>
    ← Retour au catalogue
  </button>

  <div class="watch-content">
    <div class="player-wrapper">
      <VideoPlayer
        src={film.src}
        poster={film.poster}
        title={film.title}
      />
    </div>

    <aside class="chat-sidebar" aria-label="Chat de la watch room">
      <Chat variant="watch-room" />
    </aside>
  </div>
</main>
