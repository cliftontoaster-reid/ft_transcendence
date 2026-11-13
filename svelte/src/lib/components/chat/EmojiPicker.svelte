<script lang="ts">
	import { searchChatEmojis } from '$lib/chat/emoji';

	let {
		open,
		query = $bindable(''),
		onSelect
	}: {
		open: boolean;
		query?: string;
		onSelect: (emoji: string) => void;
	} = $props();

	const visibleEmojis = $derived(searchChatEmojis(query));
</script>

{#if open}
	<div id="chat-emoji-picker" class="emoji-picker" role="group" aria-label="Choisir un emoji">
		<input
			class="emoji-search"
			type="search"
			bind:value={query}
			placeholder="Search emoji..."
			aria-label="Search emoji"
			autocomplete="off"
		/>
		{#if visibleEmojis.length > 0}
			<div class="emoji-grid">
				{#each visibleEmojis as emoji (emoji.glyph)}
					<button
						class="emoji-option"
						type="button"
						aria-label={`Insert ${emoji.glyph}`}
						onclick={() => onSelect(emoji.glyph)}
					>
						{emoji.glyph}
					</button>
				{/each}
			</div>
		{:else}
			<p class="emoji-empty">No emoji found</p>
		{/if}
	</div>
{/if}

<style>
	.emoji-picker {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 8px;
		z-index: 10;
		display: flex;
		flex-direction: column;
		gap: 8px;
		width: min(208px, calc(100vw - 32px));
		max-height: min(300px, 50vh);
		overflow: hidden;
		padding: 8px;
		border: 1px solid #3f3f46;
		border-radius: 8px;
		background: #18181b;
		box-shadow: 0 8px 24px rgb(0 0 0 / 40%);
	}

	.emoji-search {
		flex: 0 0 auto;
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		padding: 8px;
		border: 1px solid #3f3f46;
		border-radius: 4px;
		background: #27272a;
		color: #ffffff;
	}

	.emoji-grid {
		display: grid;
		grid-template-columns: repeat(4, 40px);
		justify-content: center;
		gap: 4px;
		min-height: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
	}

	.emoji-empty {
		margin: 0;
		padding: 8px;
		color: #a1a1aa;
		font-size: 0.85rem;
		text-align: center;
	}

	.emoji-option {
		width: 40px;
		height: 40px;
		justify-content: center;
		padding: 0;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: inherit;
		cursor: pointer;
		font-size: 1.25rem;
	}

	.emoji-option:hover,
	.emoji-option:focus-visible {
		background: #3f3f46;
	}
</style>
