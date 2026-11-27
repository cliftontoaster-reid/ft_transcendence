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

  // Active speaker state (set to { name: currentSpeakerName } when someone speaks, or null when silent)
  let isVoiceCallActive = $state(true);
  let isSpeakerLocked = $state(false);
  let activeSpeaker = $state<{ name: string } | null>(null);
  const activeSpeakerInitial = $derived(
    activeSpeaker?.name?.trim().charAt(0).toUpperCase() || ''
  );

  // Mouse coordinates and visibility states
  let mouseX = $state(0);
  let mouseY = $state(0);
  let chatMouseX = $state(0);
  let chatMouseY = $state(0);
  let isCursorVisible = $state(false);

  let mouseTimer: NodeJS.Timeout;
  let cursorIdleTimer: NodeJS.Timeout;

  function toggleFullscreen() {
    if (!document.fullscreenElement) {
      playerContainer.requestFullscreen().catch((err) => {
        console.error(`Fullscreen error: ${err.message}`);
      });
    } else {
      document.exitFullscreen();
    }
  }

  function handleFullscreenChange() {
    isFullscreen = !!document.fullscreenElement;
    if (isFullscreen) {
      isContactsOpen = false;
    } else {
      isContactsOpen = true;
      showOverlayChat = false;
      isCursorVisible = false;
    }
  }

  function handleMouseMove(event: MouseEvent) {
    if (!isFullscreen) return;

    // Calculate coordinates relative to player container for custom cursor
    if (playerContainer) {
      const rect = playerContainer.getBoundingClientRect();
      mouseX = event.clientX - rect.left;
      mouseY = event.clientY - rect.top;
    }

    // Calculate coordinates relative to chat overlay for spotlight gradient
    if (chatOverlay) {
      const chatRect = chatOverlay.getBoundingClientRect();
      chatMouseX = event.clientX - chatRect.left;
      chatMouseY = event.clientY - chatRect.top;
    }

    // Show overlay chat and keep it active
    showOverlayChat = true;
    clearTimeout(mouseTimer);
    mouseTimer = setTimeout(() => {
      showOverlayChat = false;
    }, 2500);

    // Make custom cursor visible and reset idle fade-out timer
    isCursorVisible = true;
    clearTimeout(cursorIdleTimer);
    cursorIdleTimer = setTimeout(() => {
      isCursorVisible = false;
    }, 1200);
  }
</script>

<svelte:head>
  <title>{film.title} - Watch Room</title>
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

        <!-- Standard fullscreen button (Normal mode) -->
        <button
          class="fullscreen-btn"
          class:visible={!isFullscreen}
          onclick={toggleFullscreen}
          title="Fullscreen"
          aria-label="Toggle fullscreen"
        >
          ⛶
        </button>

        {#if isFullscreen}
          <!-- Compact custom cursor with purple fill and idle fade-out -->
          <div
            class="dionysus-cursor"
            class:visible={isCursorVisible}
            style="left: {mouseX}px; top: {mouseY}px;"
          ></div>

          <!-- Fullscreen chat overlay with spotlight effect -->
          <div
            class="fullscreen-chat-overlay"
            class:visible={showOverlayChat}
            class:contacts-hidden={!isContactsOpen}
            bind:this={chatOverlay}
            style="--mouse-x: {chatMouseX}px; --mouse-y: {chatMouseY}px;"
          >
            <div class="chat-wrapper-inner">
              {#if isVoiceCallActive}
                <div class="speaker-rect" class:locked={isSpeakerLocked}>
                  {#if activeSpeaker}
                    <span class="speaker-avatar">{activeSpeakerInitial}</span>
                    <span class="speaker-name">{activeSpeaker.name}</span>
                  {:else}
                    <span class="speaker-name">Voice Standby</span>
                  {/if}
                </div>
              {/if}
              <Chat variant="watch-room" />
              <button
                class="speaker-lock-btn"
                class:locked={isSpeakerLocked}
                onclick={() => (isSpeakerLocked = !isSpeakerLocked)}
                title={isSpeakerLocked ? "Unlock camera" : "Lock camera"}
                aria-label={isSpeakerLocked ? "Unlock camera" : "Lock camera"}
                aria-pressed={isSpeakerLocked}
              >
                {isSpeakerLocked ? '\u{1F512}\u{FE0E}' : '\u{1F513}\u{FE0E}'}
              </button>
              <button
                class="contacts-toggle-tab"
                onclick={() => (isContactsOpen = !isContactsOpen)}
                title={isContactsOpen ? "Hide contacts" : "Show contacts"}
                aria-label="Toggle contacts"
              >
                {isContactsOpen ? '›' : '‹'}
              </button>
              <button
                class="fullscreen-toggle-btn"
                onclick={toggleFullscreen}
                title="Exit fullscreen"
                aria-label="Exit fullscreen"
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

    <!-- Normal sidebar chat -->
    <aside class="chat-sidebar" aria-label="Watch room chat">
      <div class="chat-wrapper-inner">
        {#if isVoiceCallActive}
          <div class="speaker-rect" class:locked={isSpeakerLocked}>
            {#if activeSpeaker}
              <span class="speaker-avatar">{activeSpeakerInitial}</span>
              <span class="speaker-name">{activeSpeaker.name}</span>
            {:else}
              <span class="speaker-name">Voice Standby</span>
            {/if}
          </div>
        {/if}
        <Chat variant="watch-room" />
        <button
          class="speaker-lock-btn"
          class:locked={isSpeakerLocked}
          onclick={() => (isSpeakerLocked = !isSpeakerLocked)}
          title={isSpeakerLocked ? "Unlock camera" : "Lock camera"}
          aria-label={isSpeakerLocked ? "Unlock camera" : "Lock camera"}
          aria-pressed={isSpeakerLocked}
        >
          {isSpeakerLocked ? '🔒' : '🔓'}
        </button>
        <button
          class="contacts-toggle-tab"
          onclick={() => (isContactsOpen = !isContactsOpen)}
          title={isContactsOpen ? "Hide contacts" : "Show contacts"}
          aria-label="Toggle contacts"
        >
          {isContactsOpen ? '›' : '‹'}
        </button>
      </div>
    </aside>
  </div>
</main>