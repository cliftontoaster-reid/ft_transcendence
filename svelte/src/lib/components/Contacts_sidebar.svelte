<script lang="ts">
  import ContactActivity from '$lib/components/ContactActivity.svelte';

  let {
    onDirectMessage,
    embedded = false,
    chatContacts = []
  }: {
    onDirectMessage?: (contact: string) => void;
    embedded?: boolean;
    chatContacts?: string[];
  } = $props();

  let contacts = $state([
    { name: 'Julien', initials: 'J', online: true },
    { name: 'Léo', initials: 'L', online: true },
    { name: 'Bastien', initials: 'B', online: false },
  ]);
  const actions = ['Send a direct message', 'Invite to a room', 'View activity'];

  let activeContact = $state<string | null>(null);
  let selectedAction = $state('');
  let activityContact = $state<(typeof contacts)[number] | null>(null);
  let contactFilter = $state('all');
  let isAddContactOpen = $state(false);
  let newContactName = $state('');
  let addContactError = $state('');
  let visibleContacts = $derived(
    contacts.filter((contact) => {
      if (contactFilter === 'online') return contact.online;
      if (contactFilter === 'chats') return chatContacts.includes(contact.name);
      return true;
    }),
  );

  function toggleContact(name: string) {
    activeContact = activeContact === name ? null : name;
    selectedAction = '';
  }

  function chooseAction(action: string, contact: string) {
    if (action === 'Send a direct message') {
      onDirectMessage?.(contact);
      selectedAction = '';
    } else if (action === 'View activity') {
      activityContact = contacts.find((item) => item.name === contact) ?? null;
      selectedAction = '';
    } else {
      selectedAction = `${action} — ${contact} (coming soon)`;
    }
    activeContact = null;
  }

  function addContact(event: SubmitEvent) {
    event.preventDefault();
    const name = newContactName.trim();
    if (!name) {
      addContactError = 'Enter a contact name.';
      return;
    }
    if (contacts.some((contact) => contact.name.toLowerCase() === name.toLowerCase())) {
      addContactError = 'This contact is already on your list.';
      return;
    }

    contacts.push({ name, initials: name.charAt(0).toUpperCase(), online: false });
    newContactName = '';
    addContactError = '';
    isAddContactOpen = false;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      activeContact = null;
      isAddContactOpen = false;
    }
  }

  function handleWindowClick(event: MouseEvent) {
    if (!(event.target instanceof Element)) return;
    if (event.target.closest('.contact, .contact-actions')) return;

    activeContact = null;
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} onclick={handleWindowClick} />

{#if activityContact}
  <ContactActivity contact={activityContact} onClose={() => (activityContact = null)} />
{/if}

<aside class="contacts-sidebar" class:embedded>
  <section class="contacts-panel" aria-label="Contacts">
    <div class="contacts-heading">
      <h2>Contacts</h2>
      <div class="contact-toolbar">
        <label class="contact-filter-label">
          <span class="visually-hidden">Filter contacts</span>
          <select bind:value={contactFilter} aria-label="Filter contacts">
            <option value="all">All</option>
            <option value="online">Online</option>
            <option value="chats">In chats</option>
          </select>
        </label>
        <button
          class="add-contact-button"
          type="button"
          aria-label="Add contact"
          aria-expanded={isAddContactOpen}
          onclick={() => {
            isAddContactOpen = !isAddContactOpen;
            addContactError = '';
          }}
        >
          <span aria-hidden="true">+</span>
          <span>Add</span>
        </button>
      </div>
    </div>
    {#if isAddContactOpen}
      <form class="add-contact-form" onsubmit={addContact}>
        <label class="visually-hidden" for="new-contact-name">Contact name</label>
        <input
          id="new-contact-name"
          bind:value={newContactName}
          placeholder="Contact name"
          autocomplete="off"
        />
        <button type="submit">Add</button>
        {#if addContactError}
          <p class="add-contact-error" role="alert">{addContactError}</p>
        {/if}
      </form>
    {/if}
    <ul class="contact-list">
      {#each visibleContacts as contact (contact.name)}
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

  .contacts-sidebar.embedded {
    width: 140px;
    min-width: 140px;
    margin: 0;
    height: 100%;
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

  .embedded .contacts-panel {
    height: 100%;
    background: rgb(24 24 27 / 75%);
    border-radius: 0;
    border-left: 1px solid #27272a;
    box-shadow: none;
  }

  .embedded .contacts-panel:hover,
  .embedded .contacts-panel:focus-within {
    box-shadow: none;
  }

  .contacts-panel:hover,
  .contacts-panel:focus-within {
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 2px rgb(255 122 26 / 65%),
      0 0 24px 6px rgb(255 122 26 / 30%);
  }

  .contacts-panel h2 {
    margin: 0;
    color: #ffad5c;
    font-size: 0.9rem;
  }

  .contacts-title {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .contacts-heading {
    display: grid;
    justify-items: start;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
  }

  .contact-toolbar {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    width: 100%;
  }

  .contact-filter-label {
    flex: 1;
    min-width: 0;
  }

  .contact-filter-label select {
    width: 100%;
    box-sizing: border-box;
  }

  .add-contact-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.2rem;
    flex: 1;
    min-width: 0;
    width: 100%;
    height: 22px;
    padding: 0 0.25rem;
    border: 1px solid rgb(255 173 92 / 65%);
    border-radius: 4px;
    background: #35242b;
    color: #ffad5c;
    font-size: 0.65rem;
    line-height: 1;
    box-sizing: border-box;
    cursor: pointer;
    transition:
      background-color 0.2s ease,
      box-shadow 0.2s ease,
      border-color 0.2s ease;
  }

  .add-contact-button:hover,
  .add-contact-button:focus-visible {
    border-color: #ffad5c;
    background: #472b28;
    box-shadow: 0 0 6px rgb(255 122 26 / 55%);
  }

  .add-contact-button:focus-visible,
  .add-contact-form input:focus-visible,
  .add-contact-form button:focus-visible {
    outline: 2px solid #ffad5c;
    outline-offset: 2px;
  }

  .add-contact-form {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
  }

  .add-contact-form input {
    min-width: 0;
    padding: 0.35rem;
    border: 1px solid #3f3f46;
    border-radius: 4px;
    background: #202024;
    color: #fff;
    font-size: 0.7rem;
  }

  .add-contact-form button {
    padding: 0 0.45rem;
    border: 1px solid rgb(255 173 92 / 65%);
    border-radius: 4px;
    background: #35242b;
    color: #ffad5c;
    font-size: 0.7rem;
    cursor: pointer;
  }

  .add-contact-form button:hover {
    background: #472b28;
  }

  .add-contact-error {
    grid-column: 1 / -1;
    margin: 0;
    color: #fca5a5;
    font-size: 0.65rem;
  }

  .contact-filter-label select {
    height: 22px;
    padding: 0 0.25rem;
    border: 1px solid rgb(255 173 92 / 65%);
    border-radius: 4px;
    background: #35242b;
    color: #ffad5c;
    font-size: 0.65rem;
    text-align: center;
    text-align-last: center;
    cursor: pointer;
    transition:
      background-color 0.2s ease,
      box-shadow 0.2s ease,
      border-color 0.2s ease;
  }

  .contact-filter-label select:hover,
  .contact-filter-label select:focus-visible {
    border-color: #ffad5c;
    background: #472b28;
    box-shadow: 0 0 6px rgb(255 122 26 / 55%);
  }

  .contact-filter-label select:focus-visible {
    outline: 2px solid #ffad5c;
    outline-offset: 2px;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    clip-path: inset(50%);
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

    .contacts-sidebar.embedded {
      width: 140px;
      min-width: 140px;
      height: 100%;
    }

    .embedded .contacts-panel {
      height: 100%;
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

    .contacts-sidebar.embedded {
      width: 100%;
      min-width: 0;
      height: auto;
    }

    .embedded .contacts-panel {
      height: auto;
      border-top: 1px solid #27272a;
      border-left: 0;
    }
  }
</style>