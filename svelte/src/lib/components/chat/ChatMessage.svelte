<script lang="ts">
	import type { ChatMessage as ChatMessageData } from '$lib/chat/store.svelte';

	let {
		message,
		isOwn,
		isEmojiPickerOpen,
		onToggleEmojiPicker,
		onReply
	}: {
		message: ChatMessageData;
		isOwn: boolean;
		isEmojiPickerOpen: boolean;
		onToggleEmojiPicker: () => void;
		onReply: () => void;
	} = $props();
</script>

<div class="message" class:user={isOwn} class:other={!isOwn} class:edited={message.edited}>
	<span class="sender">{message.senderName}</span>
	<p class="message-text">{message.text}</p>
	{#if message.edited}
		<span class="edited-badge">édité</span>
	{/if}
	<span class="timestamp">
		{message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
	</span>
	<div class="message-actions">
		<button
			type="button"
			aria-label="Ouvrir les emojis"
			title="Réagir"
			aria-expanded={isEmojiPickerOpen}
			aria-controls={isEmojiPickerOpen ? 'chat-emoji-picker' : undefined}
			onclick={onToggleEmojiPicker}
		>
			☺
		</button>
		<button type="button" aria-label="Répondre à ce message" title="Répondre" onclick={onReply}>
			↩
		</button>
	</div>
</div>

<style>
	.message {
		position: relative;
		max-width: 80%;
		padding: 0.75rem 1rem;
		border-radius: 8px;
		font-size: 0.9rem;
		line-height: 1.4;
		animation: fadeIn 0.2s ease-out;
	}

	.message.other {
		align-self: flex-start;
		border-bottom-left-radius: 2px;
		background: #27272a;
		color: #e4e4e7;
	}

	.message.user {
		align-self: flex-end;
		border-bottom-right-radius: 2px;
		background: #8f015f;
		color: #ffffff;
	}

	.message.edited {
		opacity: 0.9;
	}

	.sender {
		display: block;
		margin-bottom: 0.25rem;
		font-size: 0.7rem;
		font-weight: 500;
		opacity: 0.7;
	}

	.message-text {
		margin: 0;
		white-space: pre-wrap;
		word-wrap: break-word;
	}

	.edited-badge {
		margin-left: 0.5rem;
		font-size: 0.6rem;
		letter-spacing: 0.5px;
		opacity: 0.6;
		text-transform: uppercase;
	}

	.timestamp {
		display: block;
		margin-top: 0.25rem;
		font-size: 0.6rem;
		opacity: 0.5;
		text-align: right;
	}

	.message-actions {
		position: absolute;
		top: -1.8rem;
		right: 0;
		z-index: 1;
		display: none;
		gap: 0.25rem;
		padding: 0.2rem;
		border: 1px solid #3f3f46;
		border-radius: 0.4rem;
		background: #27272a;
	}

	.message:hover .message-actions,
	.message:focus-within .message-actions {
		display: flex;
	}

	.message-actions button {
		padding: 0.2rem 0.35rem;
		border: 0;
		background: transparent;
		color: #a1a1aa;
		cursor: pointer;
		line-height: 1;
	}

	.message-actions button:first-child {
		font-size: 1.25rem;
	}

	.message-actions button:hover,
	.message-actions button:focus-visible {
		color: #ffad5c;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
