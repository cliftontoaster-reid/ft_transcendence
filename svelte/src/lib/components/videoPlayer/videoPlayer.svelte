<script lang="ts">
  // --- LES PROPS (Données envoyées par la page parent) ---
  // En Svelte 5, on utilise $props() pour déclarer les valeurs entrantes.
  let { 
    src = "", 
    poster = "", 
    title = "" 
  } = $props();

  // --- LES ÉTATS INTERNES (Variables réactives du composant) ---
  // $state() indique à Svelte que ces variables vont changer au clic/action.
  let videoRef = $state<HTMLVideoElement | null>(null);
  let isPlaying = $state(false);

  // --- LES FONCTIONS ---
  function togglePlay() {
    if (!videoRef) return;

    if (isPlaying) {
      videoRef.pause();
    } else {
      videoRef.play();
    }
    isPlaying = !isPlaying;
  }
</script>

<!-- --- LA STRUCTURE HTML --- -->
<div class="player-container">
  <!-- Balise vidéo HTML5 standard -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_nonstatic_element_interactions -->
  <video 
    bind:this={videoRef} 
    {src} 
    {poster}
    onclick={togglePlay}
  >
    <track kind="captions" />
  </video>

  <!-- Overlay (Commandes au-dessus de la vidéo) -->
  <div class="overlay">
    {#if title}
      <h3 class="video-title">{title}</h3>
    {/if}

    <!-- Bouton central Play/Pause -->
    <button type="button" class="play-btn" onclick={togglePlay}>
      {isPlaying ? '❚❚' : '▶'}
    </button>
  </div>
</div>

<!-- --- LE STYLE CSS --- -->
<style>
  /* 1. Le conteneur s'adapte à 100% du parent qui l'accueille */
  .player-container {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 200px;
    background-color: #000;
    border-radius: 8px;
    overflow: hidden; /* Empêche les éléments de dépasser des coins arrondis */
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* 2. La vidéo remplit tout l'espace disponible */
  video {
    width: 100%;
    height: 100%;
    object-fit: contain; /* Conserve le ratio de la vidéo sans la déformer */
    cursor: pointer;
  }

  /* 3. L'overlay se superpose à la vidéo grâce au positionnement 'absolute' */
  .overlay {
    position: absolute;
    inset: 0; /* Équivalent à top:0; left:0; right:0; bottom:0; */
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 16px;
    box-sizing: border-box;
    background: linear-gradient(to bottom, rgba(0,0,0,0.6), transparent 40%, rgba(0,0,0,0.6));
    pointer-events: none; /* Laisse les clics traverser l'overlay vers la vidéo */
  }

  /* Réactiver les clics sur les boutons interactifs */
  .overlay button {
    pointer-events: auto;
  }

  .video-title {
    color: white;
    margin: 0;
    font-family: sans-serif;
    font-size: 1rem;
    align-self: flex-start;
  }

  .play-btn {
    background: rgba(229, 9, 20, 0.9);
    border: none;
    color: white;
    width: 50px;
    height: 50px;
    border-radius: 50%;
    font-size: 1.2rem;
    cursor: pointer;
    transition: transform 0.2s ease;
  }

  .play-btn:hover {
    transform: scale(1.1);
  }
</style>