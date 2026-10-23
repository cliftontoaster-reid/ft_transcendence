<script lang="ts">
  let { onDirectMessage }: { onDirectMessage?: (contact: string) => void } = $props();

  const contacts = [
    { name: 'Julien', initials: 'J', online: true },
    { name: 'Léo', initials: 'L', online: true },
    { name: 'Bastien', initials: 'B', online: false },
  ];
  const actions = ['Send a direct message', 'Invite to a room', 'View activity'];

  let activeContact = $state<string | null>(null);
  let selectedAction = $state('');

  function toggleContact(name: string) {
    activeContact = activeContact === name ? null : name;
    selectedAction = '';
  }

  function chooseAction(action: string, contact: string) {
    if (action === 'Send a direct message') {
      onDirectMessage?.(contact);
      selectedAction = '';
    } else {
      selectedAction = `${action} — ${contact} (coming soon)`;
    }
    activeContact = null;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') activeContact = null;
  }

  function handleWindowClick(event: MouseEvent) {
    if (!(event.target instanceof Element)) return;
    if (event.target.closest('.contact, .contact-actions')) return;

    activeContact = null;
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} onclick={handleWindowClick} />

<aside class="contacts-sidebar">
  <section class="contacts-panel" aria-label="Contacts">
    <h2>Contacts</h2>
    <ul class="contact-list">
      {#each contacts as contact (contact.name)}
        <li class="contact-item">
          <button
            class="contact"
            type="button"
            aria-haspopup="menu"
            aria-expanded={activeContact === contact.name}
            onclick={() => toggleContact(contact.name)}
          >
            <span class="contact-avatar">{contact.initials}</span>
            <span class="contact-name">{contact.name}</span>
            <span
              class:offline={!contact.online}
              class="contact-status"
              aria-label={contact.online ? 'Online' : 'Offline'}
            ></span>
          </button>
          {#if activeContact === contact.name}
            <div class="contact-actions" role="menu" aria-label={`Actions pour ${contact.name}`}>
              {#each actions as action (action)}
                <button
                  class="contact-action"
                  type="button"
                  role="menuitem"
                  onclick={() => chooseAction(action, contact.name)}
                >
                  {action}
                </button>
              {/each}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    <p class="action-notice" aria-live="polite">{selectedAction}</p>
  </section>
</aside>

<style>
  .contacts-sidebar {
    width: calc(100% + max(1.5rem, calc((100vw - 1600px) / 2 + 1.5rem)));
    margin-right: calc(-1 * max(1.5rem, calc((100vw - 1600px) / 2 + 1.5rem)));
    height: 100%;
    box-sizing: border-box;
  }

  .contacts-panel {
    height: 100%;
    padding: 0.9rem 0.75rem;
    background: #18181b;
    color: #fff;
    border-radius: 6px;
    box-sizing: border-box;
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 1px rgb(255 122 26 / 18%),
      0 0 16px 3px rgb(255 122 26 / 8%);
    transition: box-shadow 0.25s ease;
  }

  .contacts-panel:hover,
  .contacts-panel:focus-within {
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 2px rgb(255 122 26 / 65%),
      0 0 24px 6px rgb(255 122 26 / 30%);
  }

  .contacts-panel h2 {
    margin: 0 0 0.75rem;
    color: #ffad5c;
    font-size: 0.9rem;
  }

  .contact-list {
    display: grid;
    gap: 0.7rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .contact-item {
    position: relative;
    min-width: 0;
  }

  .contact {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .contact:hover .contact-name,
  .contact:focus-visible .contact-name {
    color: #ffad5c;
  }

  .contact:focus-visible,
  .contact-action:focus-visible {
    outline: 2px solid #ffad5c;
    outline-offset: 2px;
  }

  .contact-actions {
    position: absolute;
    top: calc(100% + 0.35rem);
    right: 0;
    z-index: 2;
    display: grid;
    width: max-content;
    min-width: 190px;
    padding: 0.35rem;
    border: 1px solid #3f3f46;
    border-radius: 8px;
    background: #202024;
    box-shadow: 0 8px 24px rgb(0 0 0 / 40%);
  }

  .contact-action {
    padding: 0.6rem 0.7rem;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: #fff;
    text-align: left;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .contact-action:hover {
    background: #35242b;
    color: #ffad5c;
  }

  .action-notice {
    min-height: 1em;
    margin: 0.8rem 0 0;
    color: #a1a1aa;
    font-size: 0.7rem;
  }

  .contact-avatar {
    display: grid;
    flex: 0 0 30px;
    width: 30px;
    height: 30px;
    place-items: center;
    border-radius: 50%;
    background: #35242b;
    color: #ffad5c;
    font-size: 0.65rem;
    font-weight: 700;
  }

  .contact-name {
    overflow: hidden;
    font-size: 0.8rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .contact-status {
    flex: 0 0 7px;
    width: 7px;
    height: 7px;
    margin-left: auto;
    border-radius: 50%;
    background: #22c55e;
  }

  .contact-status.offline {
    background: #71717a;
  }

  @media (max-width: 1000px) {
    .contacts-sidebar {
      width: 100%;
      margin-right: 0;
      height: auto;
    }

    .contacts-panel {
      height: auto;
    }
  }

  @media (max-width: 520px) {
    .contacts-panel {
      padding: 0.75rem 1rem;
    }

    .contact-list {
      display: flex;
      gap: 1rem;
      overflow-x: auto;
    }

    .contact-item {
      flex: 0 0 auto;
    }
  }
</style>