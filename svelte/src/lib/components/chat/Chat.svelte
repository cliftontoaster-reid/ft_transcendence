<script lang="ts">
	import ContactsSidebar from '../Contacts_sidebar.svelte';
	import { chatStore, type ChatMessage } from '$lib/chat/store.svelte';
	import MessageList from './MessageList.svelte';
	import MessageComposer from './MessageComposer.svelte';
	import { onMount, onDestroy } from 'svelte';

	let {
		variant = 'home',
		directMessageContact = null,
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
			? 'Chat général'
			: chatStore.currentChannelId?.replace('dm:', '') || 'Chat général'
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

	function sendMessage(text: string) {
		chatStore.sendMessage(text, replyTo?.id);
		replyTo = null;
	}

	function reactToMessage(message: ChatMessage, emoji: string) {
		chatStore.sendMessage(emoji, message.id);
	}

	function addEmoji(emoji: string) {
		inputMessage += emoji;
		closeEmojiPicker();
	}

	function closeEmojiPicker() {
		isEmojiPickerOpen = false;
		emojiSearchQuery = '';
	}

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

<div
	class="chat-container"
	class:connected={isConnected}
	class:watch-room={variant === 'watch-room'}
>
	<section class="conversation" aria-label="Conversation">
		<div class="chat-header">
			<span class="status-dot" class:connected={isConnected}></span>
			{#if isDM}
				<button
					class="back-button"
					type="button"
					onclick={handleBackToGlobal}
					aria-label="Retour au chat général"
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
	/*
  .chat-container.connected {
    border-color: #22c55e;
  }
*/
	.conversation {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-width: 0;
		height: 100%;
	}

	.chat-container:hover,
	.chat-container:focus-within {
		box-shadow: none;
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

	@media (max-width: 520px) {
		.chat-container {
			flex-direction: column;
		}

		.conversation {
			min-height: 300px;
		}
	}
</style>
