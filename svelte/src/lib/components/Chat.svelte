<script lang="ts">
  import ContactsSidebar from '$lib/components/Contacts_sidebar.svelte';

  type ChatMessage = { sender: string; text: string };

  let {
    directMessageContact = null,
    onBackToGlobalChat,
    onDirectMessage
  }: {
    directMessageContact?: string | null;
    onBackToGlobalChat?: () => void;
    onDirectMessage?: (contact: string) => void;
  } = $props();

  let messagesByConversation = $state<Record<string, ChatMessage[]>>({});
  let inputMessage = $state('');
  let conversationKey = $derived(directMessageContact ? `dm:${directMessageContact}` : 'global');
  let messages = $derived(messagesByConversation[conversationKey] ?? []);
  let chatContacts = $derived(
    Object.entries(messagesByConversation)
      .filter(([key, conversationMessages]) => key.startsWith('dm:') && conversationMessages.length > 0)
      .map(([key]) => key.slice(3)),
  );

  function sendMessage(e: Event) {
    e.preventDefault();
    const text = inputMessage.trim();
    if (!text) return;

    messagesByConversation[conversationKey] = [...messages, { sender: 'Moi', text }];
    inputMessage = '';
  }
</script>

<div class="chat-container">
  <section class="conversation" aria-label="Conversation">
    <div class="chat-header">
      <span class="status-dot"></span>
      {#if directMessageContact}
        <button class="back-button" type="button" onclick={onBackToGlobalChat} aria-label="Retour au chat général">
          ←
        </button>
      {/if}
      <h3>{directMessageContact ?? 'Chat général'}</h3>
    </div>

    <div class="chat-messages" aria-live="polite">
      {#if messages.length === 0}
        <p class="empty-chat">
          {directMessageContact
            ? `Commencez une conversation avec ${directMessageContact}.`
            : 'Aucun message pour le moment.'}
        </p>
      {:else}
        {#each messages as msg}
          <div class="message {msg.sender === 'Moi' ? 'user' : 'other'}">
            <span class="sender">{msg.sender}</span>
            <p>{msg.text}</p>
          </div>
        {/each}
      {/if}
    </div>

    <form class="chat-input-form" onsubmit={sendMessage}>
      <input
        type="text"
        bind:value={inputMessage}
        placeholder={directMessageContact
          ? `Message à ${directMessageContact}...`
          : 'Écrire un message...'}
        autocomplete="off"
      />
      <button type="submit">Send</button>
    </form>
  </section>

  <ContactsSidebar embedded {chatContacts} {onDirectMessage} />
</div>

<style>
  .chat-container {
    position: relative;
    background: rgb(24 24 27 / 75%);
    border: 1px solid #27272a;
    border-radius: 6px;
    width: 100%;
    max-width: none;
    display: flex;
    flex-direction: row;
    overflow: hidden;
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 1px rgb(255 122 26 / 18%),
      0 0 16px 3px rgb(255 122 26 / 8%);
    transition: box-shadow 0.25s ease;
  }

  .conversation {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    height: 100%;
  }

  .chat-container:hover,
  .chat-container:focus-within {
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 2px rgb(255 122 26 / 65%),
      0 0 24px 6px rgb(255 122 26 / 30%);
  }

  .chat-header {
    background: rgb(32 32 36 / 75%);
    padding: 1rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    border-bottom: 1px solid #27272a;
  }

  .status-dot {
    width: 10px;
    height: 10px;
    background-color: #22c55e;
    border-radius: 50%;
  }

  .chat-header h3 {
    margin: 0;
    color: #ffffff;
    font-size: 1rem;
  }

  .back-button {
    border: 0;
    padding: 0;
    background: transparent;
    color: #ffad5c;
    font-size: 1.25rem;
    cursor: pointer;
  }

  .chat-messages {
    flex: 1;
    padding: 1rem;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .empty-chat {
    color: #71717a;
    font-size: 0.85rem;
    text-align: center;
    margin: auto 0;
    align-self: center;
  }

  .message {
    padding: 0.75rem 1rem;
    border-radius: 8px;
    max-width: 80%;
    font-size: 0.9rem;
  }

  /* Style pour les autres utilisateurs connectés */
  .message.other {
    background: #27272a;
    color: #e4e4e7;
    align-self: flex-start;
  }

  /* Style pour l'utilisateur courant */
  .message.user {
    background: #8f015f;
    color: #ffffff;
    align-self: flex-end;
  }

  .sender {
    display: block;
    font-size: 0.75rem;
    opacity: 0.7;
    margin-bottom: 0.25rem;
  }

  .message p {
    margin: 0;
  }

  .chat-input-form {
    flex: 0 0 auto;
    display: flex;
    border-top: 1px solid #27272a;
    background: rgb(18 18 18 / 75%);
  }

  .chat-input-form input {
    flex: 1;
    background: transparent;
    border: none;
    padding: 0.85rem 1rem;
    color: #ffffff;
    outline: none;
    font-size: 0.9rem;
  }

  .chat-input-form button {
    display: flex;
    align-items: center;
    justify-content: center;
    background: #8f015f;
    color: white;
    border: none;
    padding: 0 1.25rem;
    font-size: 0.8rem;
    font-weight: 100;
    cursor: pointer;
    transition: background 0.2s;
  }

  .chat-input-form button:hover {
    background: #a2026d;
  }

  @media (max-width: 520px) {
    .chat-container {
      flex-direction: column;
    }

    .conversation {
      min-height: 300px;
    }
  }
</style>