<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { ChatMessage as ChatMessageData } from '$lib/chat/store.svelte';
	import ChatMessage from './ChatMessage.svelte';

	let {
		messages,
		currentUserId,
		isDirectMessage,
		channelName,
		onReply,
		onReact
	}: {
		messages: ChatMessageData[];
		currentUserId: string | null;
		isDirectMessage: boolean;
		channelName: string;
		onReply: (message: ChatMessageData) => void;
		onReact: (message: ChatMessageData, emoji: string) => void;
	} = $props();

	let messagesElement: HTMLDivElement | null = null;
	let scrollTimeout: ReturnType<typeof setTimeout> | undefined;
	let reactionPickerMessageId = $state<string | null>(null);

	function scrollToBottom() {
		if (messagesElement) {
			messagesElement.scrollTop = messagesElement.scrollHeight;
		}
	}

	function getReplies(messageId: string) {
		return messages.filter((candidate) => candidate.repliesTo === messageId);
	}

	function isEmojiReply(text: string) {
		return /^(?:[\p{Extended_Pictographic}\p{Emoji_Presentation}\uFE0F\u200D\p{Emoji_Modifier}\p{Regional_Indicator}\u20E3]+|[#*0-9]\uFE0F?\u20E3)$/u.test(
			text.trim()
		);
	}

	function toggleReactionPicker(messageId: string) {
		reactionPickerMessageId = reactionPickerMessageId === messageId ? null : messageId;
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
		{#each messages.filter((message) => !message.repliesTo) as message (message.id)}
			<div class="message-group" class:own={message.isOwn} class:other={!message.isOwn}>
				<ChatMessage
					{message}
					isOwn={!!currentUserId && message.senderId === currentUserId}
					isEmojiPickerOpen={reactionPickerMessageId === message.id}
					onToggleEmojiPicker={() => toggleReactionPicker(message.id)}
					onReply={() => onReply(message)}
					onReact={(emoji: string) => onReact(message, emoji)}
				/>
				{#if getReplies(message.id).length > 0}
					<div class="replies" class:own={message.isOwn} class:other={!message.isOwn}>
						{#each getReplies(message.id) as reply (reply.id)}
							<div
								class="reply"
								class:emoji-reply={isEmojiReply(reply.text)}
								title={isEmojiReply(reply.text) ? `Réaction de ${reply.senderName}` : undefined}
							>
								{#if !isEmojiReply(reply.text)}
									<span class="reply-sender">{reply.senderName} :</span>
								{/if}
								{reply.text}
							</div>
						{/each}
					</div>
				{/if}
			</div>
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

	.message-group {
		display: flex;
		width: 100%;
		flex-direction: column;
		gap: 0.35rem;
	}

	.reply {
		box-sizing: border-box;
		max-width: 100%;
		padding: 0.35rem 0.6rem;
		border-radius: 8px;
		color: #ffffff;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}

	.replies {
		display: flex;
		max-width: 80%;
		flex-wrap: wrap;
		gap: 0.35rem;
	}

	.replies.own {
		align-self: flex-end;
	}

	.replies.other {
		align-self: flex-start;
	}

	.reply:not(.emoji-reply) {
		width: fit-content;
		border-radius: 8px;
	}

	.replies.own .reply:not(.emoji-reply) {
		background: #8f015f;
	}

	.replies.other .reply:not(.emoji-reply) {
		background: #27272a;
		color: #e4e4e7;
	}

	.reply.emoji-reply {
		padding: 0.2rem 0.5rem;
		border: 1px solid #52525b;
		border-radius: 999px;
		background: #27272a;
		font-size: 1.15rem;
		line-height: 1.4;
	}

	.reply-sender {
		margin-right: 0.25rem;
		font-size: 0.7rem;
		opacity: 0.7;
	}
</style>
