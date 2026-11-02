<script lang="ts">
  // $lib pointe TOUJOURS vers src/lib/, peu importe où se trouve votre page
  import VideoPlayer from '$lib/components/videoPlayer/videoPlayer.svelte';

  // Exemple de données pour cette sous-page
  const film = {
    title: "Sintel",
    src: "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/Sintel.mp4",
    poster: "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/images/Sintel.jpg"
  };

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

  <div class="player-wrapper">
    <VideoPlayer 
      src={film.src} 
      poster={film.poster} 
      title={film.title} 
    />
  </div>
</main>

<style>
  .watch-page {
    min-height: 100vh;
    background-color: #0f0f0f;
    color: white;
    padding: 20px;
    box-sizing: border-box;
  }

  .back-btn {
    background: none;
    border: none;
    color: #aaa;
    font-size: 1rem;
    cursor: pointer;
    margin-bottom: 20px;
    transition: color 0.2s;
  }

  .back-btn:hover {
    color: #fff;
  }

  /* Taille du lecteur vidéo adaptée à la sous-page */
  .player-wrapper {
    width: 100%;
    max-width: 1200px;
    margin: 0 auto;
    aspect-ratio: 16 / 9;
    border-radius: 8px;
    overflow: hidden;
  }
</style>