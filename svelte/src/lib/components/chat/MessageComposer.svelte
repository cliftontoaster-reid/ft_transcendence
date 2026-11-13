<script lang="ts">
	import EmojiPicker from './EmojiPicker.svelte';

	let {
		inputMessage = $bindable(''),
		isEmojiPickerOpen = $bindable(false),
		emojiSearchQuery = $bindable(''),
		isConnected,
		isDirectMessage,
		channelName,
		onSendMessage,
		onToggleEmojiPicker,
		onCloseEmojiPicker,
		onEmojiSelect
	}: {
		inputMessage?: string;
		isEmojiPickerOpen?: boolean;
		emojiSearchQuery?: string;
		isConnected: boolean;
		isDirectMessage: boolean;
		channelName: string;
		onSendMessage: (text: string) => void;
		onToggleEmojiPicker: () => void;
		onCloseEmojiPicker: () => void;
		onEmojiSelect: (emoji: string) => void;
	} = $props();

	let emojiPickerElement: HTMLDivElement | null = null;
	let emojiToggleElement: HTMLButtonElement | null = null;

	function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (isEmojiPickerOpen) return;
		const text = inputMessage.trim();
		if (!text) return;

		onSendMessage(text);
		inputMessage = '';
	}

	function handleWindowClick(event: MouseEvent) {
		if (!isEmojiPickerOpen || !(event.target instanceof Node)) return;
		if (emojiPickerElement?.contains(event.target) || emojiToggleElement?.contains(event.target))
			return;

		onCloseEmojiPicker();
	}
</script>

<svelte:window onclick={handleWindowClick} />

<form class="chat-input-form" onsubmit={handleSubmit}>
	<div bind:this={emojiPickerElement}>
		<EmojiPicker open={isEmojiPickerOpen} bind:query={emojiSearchQuery} onSelect={onEmojiSelect} />
	</div>
	<input
		type="text"
		bind:value={inputMessage}
		placeholder={isDirectMessage && channelName !== 'Chat général'
			? `Message à ${channelName}...`
			: 'Écrire un message...'}
		autocomplete="off"
		onfocus={onCloseEmojiPicker}
	/>
	<button
		bind:this={emojiToggleElement}
		class="emoji-toggle"
		type="button"
		aria-label="Ouvrir les emojis"
		title="Ouvrir les emojis"
		aria-expanded={isEmojiPickerOpen}
		aria-controls={isEmojiPickerOpen ? 'chat-emoji-picker' : undefined}
		onclick={onToggleEmojiPicker}
	>
		☺
	</button>
	<button type="submit" disabled={!isConnected || isEmojiPickerOpen || !inputMessage.trim()}>
		Send
	</button>
</form>

<style>
	.chat-input-form {
		position: relative;
		display: flex;
		flex: 0 0 auto;
		border-top: 1px solid #27272a;
		background: rgb(18 18 18 / 75%);
	}

	.chat-input-form input {
		flex: 1;
		border: none;
		outline: none;
		padding: 0.85rem 1rem;
		background: transparent;
		color: #ffffff;
		font-size: 0.9rem;
	}

	.chat-input-form input:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}

	.chat-input-form input::placeholder {
		color: #71717a;
	}

	.chat-input-form button {
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		padding: 0 1.25rem;
		background: #8f015f;
		color: white;
		cursor: pointer;
		font-size: 0.8rem;
		font-weight: 100;
		transition: background 0.2s;
	}

	.chat-input-form .emoji-toggle {
		flex: 0 0 44px;
		padding: 0;
		background: transparent;
		color: #a1a1aa;
		font-size: 1.4rem;
	}

	.chat-input-form .emoji-toggle:hover:not(:disabled) {
		background: #27272a;
		color: #e4e4e7;
	}

	.chat-input-form button:hover:not(:disabled) {
		background: #a2026d;
	}

	.chat-input-form button:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}
</style>
