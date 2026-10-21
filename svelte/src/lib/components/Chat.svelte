<script lang="ts">
  // Liste des messages (démarre vide ou avec des vrais messages du serveur)
  let messages = $state<{ sender: string; text: string }[]>([]);
  let inputMessage = $state('');

  function sendMessage(e: Event) {
    e.preventDefault();
    if (!inputMessage.trim()) return;

    // Ajout du message localement (en attendant la liaison avec le serveur/WebSocket)
    messages = [...messages, { sender: 'Moi', text: inputMessage }];
    inputMessage = '';
  }
</script>

<div class="chat-container">
  <div class="chat-header">
    <span class="status-dot"></span>
    <h3>Chat</h3>
  </div>

  <div class="chat-messages">
      {#each messages as msg}
        <div class="message {msg.sender === 'Moi' ? 'user' : 'other'}">
          <span class="sender">{msg.sender}</span>
          <p>{msg.text}</p>
        </div>
      {/each}
  </div>

  <form class="chat-input-form" onsubmit={sendMessage}>
    <input 
      type="text" 
      bind:value={inputMessage} 
      placeholder="Write your message..." 
      autocomplete="off"
    />
    <button type="submit">Envoyer</button>
  </form>
</div>

<style>
  .chat-container {
    position: relative;
    background: #18181b;
    border: 1px solid #27272a;
    border-radius: 12px;
    width: 100%;
    max-width: 450px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 1px rgb(255 122 26 / 18%),
      0 0 16px 3px rgb(255 122 26 / 8%);
    transition: box-shadow 0.25s ease;
  }

  .chat-container:hover,
  .chat-container:focus-within {
    box-shadow:
      0 8px 24px rgba(0, 0, 0, 0.4),
      0 0 8px 2px rgb(255 122 26 / 65%),
      0 0 24px 6px rgb(255 122 26 / 30%);
  }

  .chat-header {
    background: #202024;
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

  .chat-messages {
    padding: 1rem;
    height: 300px;
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
    display: flex;
    border-top: 1px solid #27272a;
    background: #121212;
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
    background: #8f015f;
    color: white;
    border: none;
    padding: 0 1.25rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .chat-input-form button:hover {
    background: #a2026d;
  }
</style>