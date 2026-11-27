
<script lang="ts">

// le fichier chat est répartie sur plusieurs composants pour une meilleure organisation et réutilisabilité. 
// Le composant principal `Chat.svelte` gère l'état global du chat, y compris la connexion, l'envoi de messages, 
// et la gestion des messages directs. Il utilise des composants enfants tels que `MessageList` pour afficher les messages,
//  `MessageComposer` pour composer de nouveaux messages, et `ContactsSidebar` pour afficher les contacts disponibles pour 
// les messages directs.



	// Importation des composants et des modules nécessaires
	import ContactsSidebar from '../Contacts_sidebar.svelte';
	import { chatStore, type ChatMessage } from '$lib/chat/store.svelte';
	import MessageList from './MessageList.svelte';
	import MessageComposer from './MessageComposer.svelte';
	import * as m from '$lib/paraglide/messages.js';
	// onMount et onDestroy sont des fonctions du cycle de vie de Svelte qui permettent d'exécuter du code
	//  lorsque le composant est monté ou détruit.
	import { onMount, onDestroy } from 'svelte';

	// let correspond à une variable réactive dans Svelte.
	// Les variables réactives sont automatiquement mises à jour lorsque leur valeur change,
	// et elles déclenchent une mise à jour du DOM si elles sont utilisées dans le template du composant.
	let {
		variant = 'home', // la variable est une string qui peut être soit 'home' soit 'watch-room', et elle détermine le style et le comportement du chat.
		directMessageContact = null, // la variable est une string ou null, et elle représente le contact avec lequel l'utilisateur a ouvert une conversation directe, pour les messages en privé.
		onBackToGlobalChat,
		onDirectMessage
	}: {
		variant?: 'home' | 'watch-room';
		directMessageContact?: string | null;
		onBackToGlobalChat?: () => void;
		onDirectMessage?: (contact: string) => void;
	} = $props();

	let inputMessage = $state('');
	let isInitialized = $state(false);
	let isEmojiPickerOpen = $state(false);
	let emojiSearchQuery = $state('');

	let replyTo = $state<{ id: string; senderName: string; text: string } | null>(null);

	function startReply(msg: { id: string; senderName: string; text: string }) {
		replyTo = { id: msg.id, senderName: msg.senderName, text: msg.text };
	}

	// Derived state from store
	const messages = $derived(chatStore.currentMessages);
	const knownUsers = $derived(chatStore.knownUsersList);
	const currentChannelName = $derived(
		chatStore.currentChannelId === 'global'
			? m.chat_general_channel_name()
			: chatStore.currentChannelId?.replace('dm:', '') || m.chat_general_channel_name()
	);
	const isDM = $derived(chatStore.currentChannelId?.startsWith('dm:') ?? false);
	const isConnected = $derived(chatStore.isConnected);
	const myUserId = $derived(chatStore.currentUserId);

	async function initChat() {
		if (isInitialized) return;

		// Generate a user ID (in real app, this would come from auth)
		const userId = crypto.getRandomValues(new Uint8Array(16));
		userId[6] = (userId[6] & 0x0f) | 0x40;
		userId[8] = (userId[8] & 0x3f) | 0x80;

		try {
			await chatStore.connect(userId, 'demo-token', m.chat_me_name());
			isInitialized = true;

			// If we have a direct message contact, start DM
			if (directMessageContact) {
				chatStore.startDM(directMessageContact);
			}
		} catch (e) {
			console.error('Failed to connect to chat:', e);
		}
	}

	// Functions to handle chat actions
	function sendMessage(text: string) {
		chatStore.sendMessage(text, replyTo?.id);
		replyTo = null;
	}

	// Function to handle reacting to a message
	function reactToMessage(message: ChatMessage, emoji: string, existingReplyId?: string) {
		if (existingReplyId) {
			// Si l'utilisateur a déjà mis ce même émoji, on retire la réaction
			// (à brancher sur la méthode de suppression de chatStore / Protobuf)
			chatStore.deleteMessage?.(existingReplyId);
			return;
		}
		chatStore.sendMessage(emoji, message.id);
	}

	// Function to add an emoji to the input message
	function addEmoji(emoji: string) {
		inputMessage += emoji;
		closeEmojiPicker();
	}

	// Function to close the emoji picker
	function closeEmojiPicker() {
		isEmojiPickerOpen = false;
		emojiSearchQuery = ''; // remettre la recherche d'emoji à une chaîne vide lorsque le picker est fermé
	}

	// Function to toggle the emoji picker
	function toggleEmojiPicker() {
		isEmojiPickerOpen = !isEmojiPickerOpen;
		emojiSearchQuery = '';
	}

	function handleBackToGlobal() {
		chatStore.switchToGlobalChat();
		onBackToGlobalChat?.();
	}

	function handleDirectMessage(userId: string) {
		chatStore.startDM(userId);
		onDirectMessage?.(userId);
	}

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


<!-- cette partie du code est le template HTML du composant Chat.svelte.
 Il utilise des directives Svelte pour lier les données et gérer les événements. 
 Le template est structuré en deux parties principales : la section de conversation et la barre latérale des contacts.
 La section de conversation contient l'en-tête du chat, la liste des messages et le composeur de messages. 
La barre latérale des contacts affiche les utilisateurs connus et permet de démarrer des conversations directes. */ -->

<div
	class="chat-container"
	class:connected={isConnected}
	class:watch-room={variant === 'watch-room'}
>
	<section class="conversation" aria-label={m.chat_conversation_label()}>
		<div class="chat-header">
			<span class="status-dot" class:connected={isConnected}></span>
			{#if isDM}
				<button
					class="back-button"
					type="button"
					onclick={handleBackToGlobal}
					aria-label={m.chat_back_to_global_label()}
				>
					←
				</button>
			{/if}
			<h3>{currentChannelName}</h3>
		</div>

		<MessageList
			{messages}
			currentUserId={myUserId}
			isDirectMessage={isDM}
			channelName={currentChannelName}
			onReply={(message: ChatMessage) => startReply(message)}
			onReact={reactToMessage}
		/>

		<MessageComposer
			bind:inputMessage
			bind:isEmojiPickerOpen
			bind:emojiSearchQuery
			{isConnected}
			isDirectMessage={isDM}
			channelName={currentChannelName}
			onSendMessage={sendMessage}
			onToggleEmojiPicker={toggleEmojiPicker}
			onCloseEmojiPicker={closeEmojiPicker}
			onEmojiSelect={addEmoji}
		/>
	</section>

	<ContactsSidebar embedded {knownUsers} onDirectMessage={handleDirectMessage} />
</div>

<style>
    .chat-container {
        position: relative;
        width: 100%;
        max-width: none;
        display: flex;
        flex-direction: row;
        overflow: hidden;
    }

    .conversation {
        display: flex;
        flex: 1;
        flex-direction: column;
        min-width: 0;
        height: 100%;
    }

    .chat-header {
        padding: 1rem;
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .status-dot {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        transition: background-color 0.2s;
    }

    .chat-header h3 {
        margin: 0;
        font-size: 1rem;
    }

    .back-button {
        border: 0;
        padding: 0;
        background: transparent;
        font-size: 1.25rem;
        cursor: pointer;
        line-height: 1;
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
