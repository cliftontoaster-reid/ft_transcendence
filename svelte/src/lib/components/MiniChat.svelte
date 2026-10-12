<script lang="ts">
  let messages = $state([
    { sender: 'Dyonisus Bot', text: 'Bienvenue sur Dyonisus ! Prêt pour une session film ? 🍿' }
  ]);
  
  let inputMessage = $state('');

  function sendMessage(e: Event) {
    e.preventDefault();
    if (!inputMessage.trim()) return;

    messages = [...messages, { sender: 'Moi', text: inputMessage }];
    const userText = inputMessage;
    inputMessage = '';

    // Petite réponse automatique légère après un court délai
    setTimeout(() => {
      messages = [...messages, { 
        sender: 'Dyonisus Bot', 
        text: `J'ai bien reçu ton message : "${userText}". La Watch Room arrive bientôt !` 
      }];
    }, 600);
  }
</script>

<div class="chat-container">
  <div class="chat-header">
    <span class="status-dot"></span>
    <h3>Mini Chat Dyonisus</h3>
  </div>

  <div class="chat-messages">
    {#each messages as msg}
      <div class="message {msg.sender === 'Moi' ? 'user' : 'bot'}">
        <span class="sender">{msg.sender}</span>
        <p>{msg.text}</p>
      </div>
    {/each}
  </div>

  <form class="chat-input-form" onsubmit={sendMessage}>
    <input 
      type="text" 
      bind:value={inputMessage} 
      placeholder="Écris un message..." 
      autocomplete="off"
    />
    <button type="submit">Envoyer</button>
  </form>
</div>

<style>
  .chat-container {
    background: #18181b;
    border: 1px: solid #27272a;
    border-radius: 12px;
    width: 100%;
    max-width: 450px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
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
    height: 250px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .message {
    padding: 0.75rem 1rem;
    border-radius: 8px;
    max-width: 80%;
    font-size: 0.9rem;
  }

  .message.bot {
    background: #27272a;
    color: #e4e4e7;
    align-self: flex-start;
  }

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