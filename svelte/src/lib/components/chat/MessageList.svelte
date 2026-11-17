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

	// Cette fonction récupère les réponses à un message donné en filtrant la liste des messages pour trouver ceux qui ont l'ID du message d'origine comme valeur de la propriété repliesTo.
	function getReplies(messageId: string) {
		return messages.filter((candidate) => candidate.repliesTo === messageId);
	}

	function isEmojiReply(text: string) {
		return /^(?:[\p{Extended_Pictographic}\p{Emoji_Presentation}\uFE0F\u200D\p{Emoji_Modifier}\p{Regional_Indicator}\u20E3]+|[#*0-9]\uFE0F?\u20E3)$/u.test(
			text.trim()
		);
	}

	//fonction qui concernen les réponses émojis
	// Cette fonction récupère les réponses textuelles à un message donné en filtrant les réponses pour exclure celles qui sont des emojis.
	function getTextReplies(messageId: string) {
		return getReplies(messageId).filter((reply) => !isEmojiReply(reply.text));
	}

	// Cette fonction regroupe les réactions (réponses emoji) à un message donné en comptant combien de fois chaque emoji a été utilisé comme réaction.
	function getGroupedReactions(messageId: string) {
		const reactions = getReplies(messageId).filter((reply) => isEmojiReply(reply.text));
		const grouped: Record<string, number> = {};
		for (const reaction of reactions) {
			grouped[reaction.text] = (grouped[reaction.text] || 0) + 1;
		}
		return grouped;
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

<!-- // ce fichier sert à afficher la liste des messages dans le chat. -->
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
				{#if Object.keys(getGroupedReactions(message.id)).length > 0}
					<div class="reactions" class:own={message.isOwn} class:other={!message.isOwn}>
						{#each Object.entries(getGroupedReactions(message.id)) as [emoji, count]}
							<div class="reply emoji-pill" title={`Réaction de ${count} utilisateur(s)`}>
								<span>{emoji}</span>
								{#if count > 1}
									<span class="reaction-count">{count}</span>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
				{#if getTextReplies(message.id).length > 0} <!-- vérifie s'il y a des réponses textuelles au message -->
					<div class="replies" class:own={message.isOwn} class:other={!message.isOwn}>
						{#each getTextReplies(message.id) as reply (reply.id)}
							<div
								class="reply"
								title={`Réponse de ${reply.senderName}`}
							>
								<span class="reply-sender">{reply.senderName} :</span>
								{reply.text} <!-- affiche le texte de la réponse -->
							</div>
						{/each}
					</div>
				{/if}

				
			</div>
		{/each}
	{/if}
</div>

<!-- cette partie gère le style des messages dans la liste -->
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

    /* --- Conteneur des réactions émojis --- */
    .reactions {
        display: flex;
        max-width: 80%;
        flex-wrap: wrap;
        gap: 0.35rem;
        margin-top: 0.25rem;
    }

    .reactions.own {
        align-self: flex-end;
        justify-content: flex-end;
    }

    .reactions.other {
        align-self: flex-start;
        justify-content: flex-start;
    }

    /* --- Style de la pilule émoji --- */
    .emoji-pill {
        display: inline-flex;
        align-items: center;
        gap: 0.3rem;
        padding: 0.2rem 0.55rem;
        border: 1px solid #52525b;
        border-radius: 999px;
        background: #27272a;
        color: #ffffff;
        font-size: 0.9rem;
        line-height: 1.2;
        cursor: pointer;
        transition: border-color 0.15s ease, background-color 0.15s ease;
    }

    .emoji-pill:hover {
        border-color: #ffad5c;
        background: #35242b;
    }

    .reaction-count {
        font-size: 0.75rem;
        font-weight: 600;
        color: #e4e4e7;
    }

    /* --- Conteneur et style des réponses textuelles --- */
    .replies {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        max-width: 80%;
        margin-top: 0.25rem;
    }

    .replies.own {
        align-self: flex-end;
    }

    .replies.other {
        align-self: flex-start;
    }

    .reply {
        box-sizing: border-box;
        max-width: 100%;
        padding: 0.35rem 0.6rem;
        border-radius: 8px;
        color: #ffffff;
        overflow-wrap: anywhere;
        white-space: pre-wrap;
        font-size: 0.9rem;
    }

    .replies.own .reply {
        background: #8f015f;
    }

    .replies.other .reply {
        background: #27272a;
        color: #e4e4e7;
    }

    .reply-sender {
        margin-right: 0.25rem;
        font-size: 0.7rem;
        opacity: 0.7;
        font-weight: 600;
        color: #a1a1aa;
    }
</style>