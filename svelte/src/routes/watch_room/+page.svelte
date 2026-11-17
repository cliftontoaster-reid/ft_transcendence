<script lang="ts">
  import Chat from '$lib/components/chat/Chat.svelte';
  import VideoPlayer from '$lib/components/videoPlayer/videoPlayer.svelte';
  import '$lib/components/chat/css/watch-room.scss';

  const film = {
    title: "Never Gonna Give You Up",
    src: "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
    poster: "https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=800"
  };

  let isContactsOpen = $state(true);
  let playerContainer: HTMLDivElement;
  let chatOverlay: HTMLDivElement;
  let isFullscreen = $state(false);
  let showOverlayChat = $state(false);
  let mouseX = $state(0);
  let mouseY = $state(0);
  let mouseTimer: NodeJS.Timeout;

  function toggleFullscreen() {
    if (!document.fullscreenElement) {
      playerContainer.requestFullscreen().catch(err => {
        console.error(`Erreur lors du passage en plein écran : ${err.message}`);
      });
    } else {
      document.exitFullscreen();
    }
  }

  function handleFullscreenChange() {
    isFullscreen = !!document.fullscreenElement;
    if (!isFullscreen) {
      showOverlayChat = false;
    }
  }

  function handleMouseMove(event: MouseEvent) {
    if (!isFullscreen) return;
    if (chatOverlay) {
      const rect = chatOverlay.getBoundingClientRect();
      mouseX = event.clientX - rect.left;
      mouseY = event.clientY - rect.top;
    }
    showOverlayChat = true;
    clearTimeout(mouseTimer);
    mouseTimer = setTimeout(() => {
      showOverlayChat = false;
    }, 2500);
  }
</script>

<svelte:head>
  <title>{film.title} - Lecture</title>
</svelte:head>

<svelte:document onfullscreenchange={handleFullscreenChange} />

<main class="watch-page">
  <div class="top-bar">
    <h1 class="room-title">Watch Room</h1>
  </div>

  <div class="watch-layout" class:contacts-hidden={!isContactsOpen}>
    <section class="video-section">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div 
        class="player-wrapper" 
        bind:this={playerContainer}
        onmousemove={handleMouseMove}
      >
        <VideoPlayer 
          src={film.src} 
          poster={film.poster} 
          title={film.title} 
        />

        <button 
          class="fullscreen-btn" 
          class:visible={!isFullscreen}
          onclick={toggleFullscreen}
          title="Plein écran"
          aria-label="Activer le plein écran"
        >
          ⛶
        </button>

        {#if isFullscreen}
          <div 
            class="fullscreen-chat-overlay" 
            class:visible={showOverlayChat} 
            class:contacts-hidden={!isContactsOpen}
            bind:this={chatOverlay}
            style="--mouse-x: {mouseX}px; --mouse-y: {mouseY}px;"
          >
            <div class="chat-wrapper-inner">
              <Chat variant="watch-room" />
              <button 
                class="contacts-toggle-tab" 
                onclick={() => isContactsOpen = !isContactsOpen}
                title={isContactsOpen ? "Masquer les contacts" : "Afficher les contacts"}
                aria-label="Basculer les contacts"
              >
                {isContactsOpen ? '›' : '‹'}
              </button>
              <button 
                class="fullscreen-toggle-btn" 
                onclick={toggleFullscreen}
                title="Plein écran"
                aria-label="Basculer le plein écran"
              >
                ⛶
              </button>
            </div>
          </div>
        {/if}
      </div>

      <div class="film-info">
        <h2>{film.title}</h2>
      </div>
    </section>

    <aside class="chat-sidebar" aria-label="Chat de la watch room">
      <div class="chat-wrapper-inner">
        <Chat variant="watch-room" />
        <button 
          class="contacts-toggle-tab" 
          onclick={() => isContactsOpen = !isContactsOpen}
          title={isContactsOpen ? "Masquer les contacts" : "Afficher les contacts"}
          aria-label="Basculer les contacts"
        >
          {isContactsOpen ? '›' : '‹'}
        </button>
      </div>
    </aside>
  </div>
</main>