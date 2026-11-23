<!--
  RecommendationRow.svelte - svelte
  Copyright (C) 2026 Clifton Toaster Reid

  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU Affero General Public License as
  published by the Free Software Foundation, either version 3 of the
  License, or (at your option) any later version.

  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
  GNU Affero General Public License for more details.

  You should have received a copy of the GNU Affero General Public License
  along with this program. If not, see <https://www.gnu.org/licenses/>.
-->

<script lang="ts">
	import type { Suggestion } from '$lib/recommendations';

	let {
		title,
		items,
		emptyMessage,
		errorMessage
	}: {
		title: string;
		items: Suggestion[];
		emptyMessage: string;
		errorMessage?: string;
	} = $props();
</script>

<section class="recommendation-row" aria-label={title}>
	<h2 class="row-title">{title}</h2>

	{#if errorMessage}
		<p class="empty error" role="alert">{errorMessage}</p>
	{:else if items.length === 0}
		<p class="empty">{emptyMessage}</p>
	{:else}
		<ul class="cards">
			{#each items as item (item.id)}
				<li class="card">
					<span class="card-title">{item.title}</span>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.recommendation-row {
		width: 100%;
		min-width: 0;
		padding: 0 0 2rem;
		box-sizing: border-box;
	}

	.row-title {
		margin: 0;
		color: #ffad5c;
		font-size: clamp(1.05rem, 1.6vw, 1.25rem);
	}

	.empty {
		margin: 0.75rem 0 0;
		color: #ffffff;
		font-size: 0.95rem;
	}

	.error {
		color: #ffad5c;
	}

	.cards {
		display: flex;
		gap: 0.75rem;
		margin: 0.85rem 0 0;
		padding: 0 0 0.5rem;
		list-style: none;
		overflow-x: auto;
		scroll-snap-type: x proximity;
	}

	.card {
		display: flex;
		flex: 0 0 clamp(10rem, 28vw, 13rem);
		align-items: flex-end;
		min-height: 7rem;
		padding: 1rem;
		box-sizing: border-box;
		border: 1px solid rgb(255 173 92 / 30%);
		border-radius: 12px;
		background-color: #3b2943;
		scroll-snap-align: start;
	}

	.card-title {
		color: #ffffff;
		font-size: clamp(0.95rem, 1.4vw, 1.05rem);
		line-height: 1.35;
		overflow-wrap: anywhere;
	}

	@media (max-width: 700px) {
		.card {
			flex-basis: clamp(9rem, 44vw, 11rem);
			min-height: 6rem;
		}
	}

	@media (max-width: 420px) {
		.card {
			flex-basis: 78vw;
		}
	}
</style>
