<script lang="ts">
  import ContactsSidebar from '$lib/components/Contacts_sidebar.svelte';
  import { chatStore } from '$lib/chat/store.svelte';
  import { prepare, layout } from '@chenglou/pretext';
  import { onMount, onDestroy } from 'svelte';

  let {
    directMessageContact = null,
    onBackToGlobalChat,
    onDirectMessage
  }: {
    directMessageContact?: string | null;
    onBackToGlobalChat?: () => void;
    onDirectMessage?: (contact: string) => void;
  } = $props();

  let inputMessage = $state('');
  let chatMessagesRef: HTMLDivElement | null = null;
  let isInitialized = $state(false);

  // Pretext prepared text cache
  const preparedTextCache = new Map<string, ReturnType<typeof prepare>>();

  // Derived state from store
  const messages = $derived(chatStore.currentMessages);
  const dmChannels = $derived(chatStore.dmChannels);
  const knownUsers = $derived(chatStore.knownUsersList);
  const currentChannelName = $derived(
    chatStore.currentChannelId === 'global' 
      ? 'Chat général' 
      : chatStore.currentChannelId?.replace('dm:', '') || 'Chat général'
  );
  const isDM = $derived(chatStore.currentChannelId?.startsWith('dm:') ?? false);
  const isConnected = $derived(chatStore.isConnected);
  const myUserId = $derived(chatStore.currentUserId);

  // Prepare text for Pretext measurement
  function getPreparedText(text: string) {
    if (!preparedTextCache.has(text)) {
      // Use the same font as in CSS
      preparedTextCache.set(text, prepare(text, '14px system-ui, sans-serif', {
        whiteSpace: 'pre-wrap',
        wordBreak: 'normal',
      }));
    }
    return preparedTextCache.get(text)!;
  }

  // Calculate message bubble height using Pretext
  function getMessageHeight(text: string, maxWidth: number = 400): number {
    const prepared = getPreparedText(text);
    const { height } = layout(prepared, maxWidth, 20); // line-height: 20px
    return height;
  }

  function scrollToBottom() {
    if (chatMessagesRef) {
      chatMessagesRef.scrollTop = chatMessagesRef.scrollHeight;
    }
  }

  async function initChat() {
    if (isInitialized) return;
    
    // Generate a user ID (in real app, this would come from auth)
    const userId = crypto.getRandomValues(new Uint8Array(16));
    userId[6] = (userId[6] & 0x0f) | 0x40;
    userId[8] = (userId[8] & 0x3f) | 0x80;
    
    try {
      await chatStore.connect(userId, 'demo-token', 'Moi');
      isInitialized = true;
      
      // If we have a direct message contact, start DM
      if (directMessageContact) {
        chatStore.startDM(directMessageContact);
      }
    } catch (e) {
      console.error('Failed to connect to chat:', e);
    }
  }

  function handleSendMessage(e: Event) {
    e.preventDefault();
    const text = inputMessage.trim();
    if (!text) return;
    
    chatStore.sendMessage(text);
    inputMessage = '';
    
    // Scroll to bottom after message is sent
    setTimeout(scrollToBottom, 0);
  }

  function handleBackToGlobal() {
    chatStore.switchToGlobalChat();
    onBackToGlobalChat?.();
  }

  function handleDirectMessage(userId: string) {
    chatStore.startDM(userId);
    onDirectMessage?.(userId);
  }

  // Effect to scroll to bottom when messages change
  $effect(() => {
    messages; // subscribe to changes
    setTimeout(scrollToBottom, 0);
  });

  // Effect for directMessageContact prop changes
  $effect(() => {
    if (directMessageContact && isInitialized) {
      chatStore.startDMByName(directMessageContact);
    } else if (!directMessageContact && isInitialized && isDM) {
      chatStore.switchToGlobalChat();
    }
  });

  onMount(() => {
    initChat();
  });

  onDestroy(() => {
    chatStore.disconnect();
  });
</script>

<div class="chat-container" class:connected={isConnected}>
  <section class="conversation" aria-label="Conversation">
    <div class="chat-header">
      <span class="status-dot" class:connected={isConnected}></span>
      {#if isDM}
        <button class="back-button" type="button" onclick={handleBackToGlobal} aria-label="Retour au chat général">
          ←
        </button>
      {/if}
      <h3>{currentChannelName}</h3>
    </div>

    <div class="chat-messages" aria-live="polite" role="log" bind:this={chatMessagesRef}>
      {#if messages.length === 0}
        <p class="empty-chat">
          {isDM && currentChannelName !== 'Chat général'
            ? `Commencez une conversation avec ${currentChannelName}.`
            : 'Aucun message pour le moment.'}
        </p>
      {:else}
        {#each messages as msg (msg.id)}
          <div 
            class="message {myUserId && msg.senderId === myUserId ? 'user' : 'other'}" 
            class:edited={msg.edited}
            style:max-width="80%"
          >
            <span class="sender">{msg.senderName}</span>
            <p class="message-text">{msg.text}</p>
            {#if msg.edited}
              <span class="edited-badge">édité</span>
            {/if}
            <span class="timestamp">{msg.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
          </div>
        {/each}
      {/if}
    </div>

    <form class="chat-input-form" onsubmit={handleSendMessage}>
      <input
        type="text"
        bind:value={inputMessage}
        placeholder={isDM && currentChannelName !== 'Chat général'
          ? `Message à ${currentChannelName}...`
          : 'Écrire un message...'}
        autocomplete="off"
        disabled={!isConnected}
      />
      <button type="submit" disabled={!isConnected || !inputMessage.trim()}>
        Send
      </button>
    </form>
  </section>

  <ContactsSidebar embedded {knownUsers} onDirectMessage={handleDirectMessage} />
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

  .chat-container.connected {
    border-color: #22c55e;
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
    background-color: #71717a;
    border-radius: 50%;
    transition: background-color 0.2s;
  }

  .status-dot.connected {
    background-color: #22c55e;
    box-shadow: 0 0 8px #22c55e;
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
    line-height: 1;
  }

  .back-button:hover {
    color: #ffad5c;
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
    line-height: 1.4;
    position: relative;
    animation: fadeIn 0.2s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .message.other {
    background: #27272a;
    color: #e4e4e7;
    align-self: flex-start;
    border-bottom-left-radius: 2px;
  }

  .message.user {
    background: #8f015f;
    color: #ffffff;
    align-self: flex-end;
    border-bottom-right-radius: 2px;
  }

  .message.edited {
    opacity: 0.9;
  }

  .sender {
    display: block;
    font-size: 0.7rem;
    opacity: 0.7;
    margin-bottom: 0.25rem;
    font-weight: 500;
  }

  .message-text {
    margin: 0;
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  .edited-badge {
    font-size: 0.6rem;
    opacity: 0.6;
    margin-left: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .timestamp {
    display: block;
    font-size: 0.6rem;
    opacity: 0.5;
    margin-top: 0.25rem;
    text-align: right;
  }

  .message.user .timestamp {
    text-align: right;
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

  .chat-input-form input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .chat-input-form input::placeholder {
    color: #71717a;
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

  .chat-input-form button:hover:not(:disabled) {
    background: #a2026d;
  }

  .chat-input-form button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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