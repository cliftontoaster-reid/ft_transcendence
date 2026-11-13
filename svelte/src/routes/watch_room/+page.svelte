<script lang="ts">
  import Chat from '$lib/components/chat/Chat.svelte';
  import VideoPlayer from '$lib/components/videoPlayer/videoPlayer.svelte';
  import '$lib/components/chat/css/watch-room.scss';

  const film = {
    title: "Sintel",
    src: "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/Sintel.mp4",
    poster: "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/images/Sintel.jpg"
  };

  let isContactsOpen = $state(true);
  let playerContainer: HTMLDivElement;
  let isFullscreen = $state(false);
  let showOverlayChat = $state(false);
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

  function handleMouseMove() {
    if (!isFullscreen) return;
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
          class:visible={!isFullscreen || showOverlayChat}
          onclick={toggleFullscreen}
          title="Plein écran"
          aria-label="Activer le plein écran"
        >
          ⛶
        </button>

        {#if isFullscreen}
          <div class="fullscreen-chat-overlay" class:visible={showOverlayChat} class:contacts-hidden={!isContactsOpen}>
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

<style>
  .watch-page {
    min-height: 100vh;
    background-color: #0f0f0f;
    color: #ffffff;
    padding: 24px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  }

  .top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  .room-title {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
  }

  .watch-layout {
    display: grid;
    grid-template-columns: 1fr 440px;
    gap: 24px;
    flex: 1;
    align-items: start;
    transition: grid-template-columns 0.3s cubic-bezier(0.4, 0, 0.2, 1), gap 0.3s ease;
  }

  .video-section {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }

  .player-wrapper {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: 12px;
    overflow: hidden;
    background: #000000;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
  }

  .player-wrapper:fullscreen {
    width: 100vw;
    height: 100vh;
    aspect-ratio: auto;
    border-radius: 0;
  }

  .fullscreen-btn {
    position: absolute;
    bottom: 16px;
    right: 16px;
    z-index: 25;
    width: 36px;
    height: 36px;
    background: rgba(0, 0, 0, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: #ffffff;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 1.1rem;
    backdrop-filter: blur(4px);
    opacity: 0;
    transition: opacity 0.3s ease, background 0.2s ease, transform 0.1s ease;
    pointer-events: none;
  }

  .fullscreen-btn.visible {
    opacity: 1;
    pointer-events: auto;
  }

  .fullscreen-btn:hover {
    background: rgba(0, 0, 0, 0.85);
    border-color: rgba(255, 255, 255, 0.4);
    transform: scale(1.05);
  }

  .fullscreen-chat-overlay {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 400px;
    background: rgba(18, 18, 20, 0.85);
    backdrop-filter: blur(12px);
    border-left: 1px solid rgba(255, 255, 255, 0.1);
    z-index: 20;
    display: flex;
    flex-direction: column;
    transform: translateX(100%);
    opacity: 0;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s ease;
    pointer-events: none;
  }

  .fullscreen-chat-overlay.visible {
    transform: translateX(0);
    opacity: 1;
    pointer-events: auto;
  }

  .film-info h2 {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0;
  }

  .chat-sidebar {
    height: calc(100vh - 140px);
    min-height: 500px;
    background: rgba(24, 24, 27, 0.75);
    border: 1px solid #27272a;
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .chat-wrapper-inner {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .contacts-toggle-tab {
    position: absolute;
    top: 14px;
    right: 172px;
    z-index: 50;
    width: 28px;
    height: 28px;
    background: #27272a;
    border: 1px solid #3f3f46;
    color: #e4e4e7;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
    transition: right 0.3s cubic-bezier(0.4, 0, 0.2, 1), background 0.2s ease, color 0.2s ease;
  }

  .contacts-toggle-tab:hover {
    background: #ff7a1a;
    border-color: #ff7a1a;
    color: #ffffff;
  }

  .watch-layout.contacts-hidden .contacts-toggle-tab,
  .fullscreen-chat-overlay.contacts-hidden .contacts-toggle-tab {
    right: 12px; 
  }

  :global(.chat-container) {
    display: flex !important;
    flex-direction: row !important;
    height: 100% !important;
    width: 100% !important;
    overflow: hidden !important;
  }

  :global(.chat-container .conversation) {
    flex: 1 !important;
    min-width: 0 !important;
    display: flex !important;
    flex-direction: column !important;
    height: 100% !important;
    overflow: hidden !important;
  }

  :global(.chat-container aside) {
    width: 170px !important;
    flex-shrink: 0 !important;
    height: 100% !important;
    overflow-y: auto !important;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1), width 0.3s ease, opacity 0.2s ease;
  }

  .watch-layout.contacts-hidden :global(.chat-container) :global(aside),
  .fullscreen-chat-overlay.contacts-hidden :global(.chat-container) :global(aside) {
    transform: translateX(100%);
    opacity: 0;
    width: 0 !important;
    min-width: 0 !important;
    padding: 0 !important;
    margin: 0 !important;
    border: none !important;
    overflow: hidden;
    pointer-events: none;
  }

  :global(.chat-container form),
  :global(.chat-container .message-composer) {
    display: flex !important;
    flex-wrap: nowrap !important;
    gap: 8px !important;
    box-sizing: border-box !important;
  }

  :global(.chat-container form input),
  :global(.chat-container .message-composer input),
  :global(.chat-container form textarea),
  :global(.chat-container .message-composer textarea) {
    flex: 1 !important;
    min-width: 0 !important;
  }

  :global(.chat-container form button[type="submit"]),
  :global(.chat-container .message-composer button[type="submit"]) {
    width: 36px !important;
    min-width: 36px !important;
    height: 36px !important;
    padding: 0 !important;
    font-size: 0 !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
    border-radius: 8px !important;
    flex-shrink: 0 !important;
  }

  :global(.chat-container form button[type="submit"]::after),
  :global(.chat-container .message-composer button[type="submit"]::after) {
    content: "➔" !important;
    font-size: 1rem !important;
    color: #ffffff !important;
  }

  @media (max-width: 1024px) {
    .watch-layout {
      grid-template-columns: 1fr;
      gap: 16px;
    }
    .chat-sidebar {
      height: 450px;
    }
  }
</style>