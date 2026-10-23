<script lang="ts">
  let {
    contact,
    onClose
  }: {
    contact: { name: string; online: boolean };
    onClose: () => void;
  } = $props();
</script>

<div class="modal-backdrop">
  <button class="backdrop-close" type="button" aria-label="Close activity" onclick={onClose}></button>
  <dialog
    open
    class="activity-dialog"
    aria-labelledby="activity-title"
  >
    <button class="close-button" type="button" aria-label="Close activity" onclick={onClose}>
      &times;
    </button>
    <h2 id="activity-title">{contact.name}'s activity</h2>

    <section class="activity-section">
      <h3>Currently watching</h3>
      <p>No current activity to show.</p>
    </section>

    <section class="activity-section">
      <h3>Recent activity</h3>
      <p>Activity details are not available yet.</p>
    </section>

    <p class="privacy-note">
      Activity is only visible when allowed by {contact.name}'s privacy settings.
    </p>
  </dialog>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1001;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgb(0 0 0 / 70%);
  }

  .backdrop-close {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: 0;
    background: transparent;
    cursor: default;
  }

  .activity-dialog {
    position: relative;
    z-index: 1;
    width: min(100%, 420px);
    margin: 0;
    padding: 1.5rem;
    border: 1px solid #3f3f46;
    border-radius: 6px;
    background: #18181b;
    color: #fff;
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 2px rgb(255 122 26 / 35%),
      0 0 24px 6px rgb(255 122 26 / 15%);
    box-sizing: border-box;
  }

  .activity-dialog h2 {
    margin: 0 2rem 1.25rem 0;
    color: #ffad5c;
    font-size: 1.1rem;
  }

  .close-button {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    border: 0;
    background: transparent;
    color: #a1a1aa;
    font-size: 1.5rem;
    line-height: 1;
    cursor: pointer;
  }

  .close-button:hover,
  .close-button:focus-visible {
    color: #ffad5c;
  }

  .activity-section {
    margin-top: 1rem;
  }

  .activity-section h3 {
    margin: 0 0 0.35rem;
    font-size: 0.9rem;
  }

  .activity-section p,
  .privacy-note {
    margin: 0;
    color: #a1a1aa;
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .privacy-note {
    margin-top: 1.25rem;
    padding-top: 0.85rem;
    border-top: 1px solid #3f3f46;
  }
</style>
