<script>
  let { 
    title = 'Film',
    description = 'description du film',
    imageUrl = 'https://picsum.photos/1280/720',
    primaryActionText = 'Play',
    secondaryActionText = 'More infos',
    playHref = '/watch_room',
    onPrimaryClick,
    onSecondaryClick
  } = $props();

  // Variable pour gérer l'ouverture de la modale "More infos"
  let isModalOpen = $state(false);
</script>

<div class="overview-container">
  <!-- Image de fond -->
  <img src={imageUrl} alt={title} class="overview-bg" loading="eager" />

  <!-- Overlay sombre pour la lisibilité du texte -->
  <div class="overview-overlay"></div>

  <!-- Contenu texte + boutons superposés -->
  <div class="overview-content">
    <h1 class="title">{title}</h1>
    {#if description}
      <p class="description">{description}</p>
    {/if}

    <div class="actions">
      {#if primaryActionText}
        <a href={playHref} class="btn primary" onclick={onPrimaryClick}>
          <svg viewBox="0 0 24 24" fill="currentColor" class="icon play-icon">
            <path d="M8 5v14l11-7z"/>
          </svg>
          {primaryActionText}
        </a>
      {/if}

      {#if secondaryActionText}
        <button type="button" class="btn secondary" onclick={() => { isModalOpen = true; if (onSecondaryClick) onSecondaryClick(); }}>
          <svg viewBox="0 0 24 24" fill="currentColor" class="icon info-icon">
            <path d="M11 7h2v2h-2zm0 4h2v6h-2zm1-9C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8-8 8z"/>
          </svg>
          {secondaryActionText}
        </button>
      {/if}
    </div>
  </div>
</div>

<!-- Modale d'informations (More infos) -->
{#if isModalOpen}
  <div class="modal-backdrop" onclick={() => isModalOpen = false} role="presentation">
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog">
      <button class="close-btn" onclick={() => isModalOpen = false}>&times;</button>
      <h2>{title}</h2>
      <p>{description}</p>
    </div>
  </div>
{/if}

<style>
  .overview-container {
    position: relative;
    width: 90%;
    max-width: none;
    margin: 0 auto;
    aspect-ratio: 16 / 9;
    border-radius: 16px;
    overflow: hidden;
    color: #ffffff;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
  }

  .overview-bg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    z-index: 1;
  }

  .overview-overlay {
    position: absolute;
    inset: 0;
    z-index: 2;
    background: linear-gradient(
      to top,
      rgba(0, 0, 0, 0.85) 0%,
      rgba(0, 0, 0, 0.4) 40%,
      rgba(0, 0, 0, 0.05) 100%
    ),
    linear-gradient(
      to right,
      rgba(0, 0, 0, 0.75) 0%,
      transparent 60%
    );
  }

  .overview-content {
    position: relative;
    z-index: 3;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    height: 100%;
    padding: clamp(1.5rem, 4vw, 3rem);
    max-width: 650px;
    box-sizing: border-box;
  }

  .title {
    margin: 0 0 0.75rem 0;
    font-size: clamp(2rem, 5vw, 3.75rem);
    font-weight: 800;
    line-height: 1.05;
    letter-spacing: -0.02em;
    text-transform: uppercase;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.5);
  }

  .description {
    margin: 0 0 1.5rem 0;
    font-size: clamp(0.9rem, 1.8vw, 1.15rem);
    line-height: 1.4;
    opacity: 0.9;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .actions {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: clamp(0.5rem, 1vw, 0.75rem) clamp(1rem, 2vw, 1.5rem);
    border-radius: 6px;
    font-weight: 700;
    font-size: clamp(0.8rem, 1.3vw, 0.95rem);
    text-decoration: none;
    border: none;
    cursor: pointer;
    transition: transform 0.15s ease, opacity 0.2s ease, background-color 0.2s ease;
    box-sizing: border-box;
  }

  .btn:hover {
    transform: scale(1.03);
  }

  .btn:active {
    transform: scale(0.97);
  }

  .btn.primary {
    background-color: #ffffff;
    color: #000000;
  }

  .btn.primary:hover {
    background-color: #f1f5f9;
  }

  .btn.secondary {
    background-color: rgba(255, 255, 255, 0.2);
    color: #ffffff;
    backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.3);
  }

  .btn.secondary:hover {
    background-color: rgba(255, 255, 255, 0.3);
  }

  .icon {
    width: 1.2em;
    height: 1.2em;
  }

  /* Overlay / Modale */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgba(0, 0, 0, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(4px);
  }

  .modal-card {
    position: relative;
    background: #181818;
    color: #ffffff;
    padding: 2rem;
    border-radius: 12px;
    max-width: 500px;
    width: 90%;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
  }

  .close-btn {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: none;
    border: none;
    color: #fff;
    font-size: 1.5rem;
    cursor: pointer;
  }

  @media (max-width: 640px) {
    .overview-container {
      width: 92%;
      border-radius: 12px;
    }

    .actions {
      flex-direction: column;
      align-items: stretch;
      gap: 0.5rem;
    }

    .btn {
      width: 100%;
      justify-content: center;
    }
  }
</style>