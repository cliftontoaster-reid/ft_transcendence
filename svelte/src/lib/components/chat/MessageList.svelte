<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { ChatMessage as ChatMessageData } from '$lib/chat/store.svelte';
	import ChatMessage from './ChatMessage.svelte';

	let {
		messages,
		currentUserId,
		isDirectMessage,
		channelName,
		isEmojiPickerOpen,
		onToggleEmojiPicker,
		onReply
	}: {
		messages: ChatMessageData[];
		currentUserId: string | null;
		isDirectMessage: boolean;
		channelName: string;
		isEmojiPickerOpen: boolean;
		onToggleEmojiPicker: () => void;
		onReply: (message: ChatMessageData) => void;
	} = $props();

	let messagesElement: HTMLDivElement | null = null;
	let scrollTimeout: ReturnType<typeof setTimeout> | undefined;

	function scrollToBottom() {
		if (messagesElement) {
			messagesElement.scrollTop = messagesElement.scrollHeight;
		}
	}

	$effect(() => {
		messages;
		scrollTimeout = setTimeout(scrollToBottom, 0);
	});

	onDestroy(() => {
		if (scrollTimeout !== undefined) clearTimeout(scrollTimeout);
	});
</script>

<div class="chat-messages" aria-live="polite" role="log" bind:this={messagesElement}>
	{#if messages.length === 0}
		<p class="empty-chat">
			{isDirectMessage && channelName !== 'Chat général'
				? `Commencez une conversation avec ${channelName}.`
				: 'Aucun message pour le moment.'}
		</p>
	{:else}
		{#each messages as message (message.id)}
			<ChatMessage
				{message}
				isOwn={!!currentUserId && message.senderId === currentUserId}
				{isEmojiPickerOpen}
				{onToggleEmojiPicker}
				onReply={() => onReply(message)}
			/>
		{/each}
	{/if}
</div>

<style>
	.chat-messages {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 0.75rem;
		min-height: 0;
		overflow-y: auto;
		padding: 1rem;
	}

	.empty-chat {
		align-self: center;
		margin: auto 0;
		color: #71717a;
		font-size: 0.85rem;
		text-align: center;
	}
</style>
