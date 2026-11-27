<script lang="ts">
	// ce fichier sert à afficher un sélecteur d'emoji dans le chat.
	//  Il est utilisé par le composant ChatInput.svelte 
	// pour permettre aux utilisateurs de choisir des emojis à insérer dans leurs messages.
	import { searchChatEmojis } from '$lib/chat/emoji';
	import { tick } from 'svelte';

	let {
		open,
		anchor,
		pickerId = 'chat-emoji-picker',
		query = $bindable(''),
		onSelect,
		onClose
	}: {
		open: boolean;
		anchor: HTMLElement | null;
		pickerId?: string;
		query?: string;
		onSelect: (emoji: string) => void;
		onClose: () => void;
	} = $props();

	let pickerElement = $state<HTMLDivElement | null>(null);
	const visibleEmojis = $derived(searchChatEmojis(query));

	function handleOutsideClick(event: MouseEvent) {
		if (!open || !(event.target instanceof Node)) return;
		if (pickerElement?.contains(event.target) || anchor?.contains(event.target)) return;

		onClose();
	}

	$effect(() => {
		if (!open || !anchor) return;

		void tick().then(() => {
			const picker = pickerElement;
			if (!picker || !anchor || !open) return;
			if (!picker.matches(':popover-open')) picker.showPopover();

			const anchorRect = anchor.getBoundingClientRect();
			const pickerRect = picker.getBoundingClientRect();
			const gap = 8;
			const topAbove = anchorRect.top - pickerRect.height - gap;
			const top =
				topAbove >= gap
					? topAbove
					: Math.min(anchorRect.bottom + gap, window.innerHeight - pickerRect.height - gap);
			const left = Math.max(
				gap,
				Math.min(anchorRect.right - pickerRect.width, window.innerWidth - pickerRect.width - gap)
			);

			picker.style.top = `${top}px`;
			picker.style.left = `${left}px`;
		});
	});
</script>

<svelte:window onclick={handleOutsideClick} />

{#if open}
	<div
		bind:this={pickerElement}
		id={pickerId}
		class="emoji-picker"
		popover="manual"
		role="group"
		aria-label="Choisir un emoji"
	>
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
		position: fixed;
		inset: auto;
		top: 0;
		left: 0;
		z-index: 1000;
		margin: 0;
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
