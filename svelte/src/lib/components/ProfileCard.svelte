<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ageLabels, type Profile } from '$lib/profile';

	let {
		profile,
		actions,
		children
	}: {
		profile: Profile;
		/** Buttons shown next to the name (e.g. "Edit"). */
		actions?: Snippet;
		/** Content rendered between the header and the info panels (e.g. an edit form). */
		children?: Snippet;
	} = $props();
</script>

<main class="page-shell">
	<section class="profile-card">
		<div class="cover"></div>

		<div class="profile-header">
			{#if profile.avatarUrl}
				<img class="avatar" src={profile.avatarUrl} alt={profile.displayName} />
			{:else}
				<div class="avatar avatar-fallback" aria-label="Avatar">
					{profile.displayName.slice(0, 1).toUpperCase()}
				</div>
			{/if}

			<div class="profile-meta">
				<div class="identity">
					<span class="eyebrow">Account</span>
					<h1>{profile.displayName}</h1>
					{#if profile.username}
						<p class="username">@{profile.username}</p>
					{/if}
				</div>

				{#if actions}
					<div class="actions">
						{@render actions()}
					</div>
				{/if}
			</div>
		</div>

		{@render children?.()}

		<div class="content-grid">
			<section class="panel full-width">
				<h2>Bio</h2>
				<p>{profile.bio}</p>
			</section>

			<section class="panel full-width">
				<h2>Information</h2>
				<ul class="info-list">
					<li>
						<span>ID</span>
						<strong>{profile.id}</strong>
					</li>
					<li>
						<span>Username</span>
						<strong>{profile.username || '—'}</strong>
					</li>
					<li>
						<span>Age bracket</span>
						<strong>{ageLabels[profile.age]}</strong>
					</li>
				</ul>
			</section>
		</div>
	</section>
</main>

<style>
	* {
		box-sizing: border-box;
	}

	.page-shell {
		min-height: calc(100vh - 2rem);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding: 1rem 0 2rem;
	}

	.profile-card {
		width: 100%;
		max-width: 1080px;
		min-height: 640px;
		background: rgba(18, 18, 18, 0.96);
		border: 1px solid rgba(255, 138, 29, 0.2);
		border-radius: 18px;
		overflow: hidden;
		box-shadow: 0 18px 40px rgba(0, 0, 0, 0.35);
	}

	.cover {
		height: 180px;
		background: linear-gradient(135deg, #ff8a1d 0%, #6a2f00 100%);
	}

	.profile-header {
		display: flex;
		gap: 20px;
		padding: 0 20px 20px;
		margin-top: -42px;
		align-items: end;
	}

	.avatar {
		width: 128px;
		height: 128px;
		border-radius: 18px;
		object-fit: cover;
		border: 3px solid #050505;
		background: #111;
	}

	.avatar-fallback {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: #ffe3c8;
		font-size: 2rem;
		font-weight: 800;
		background: linear-gradient(135deg, #8b3f0f 0%, #ff8a1d 100%);
	}

	.profile-meta {
		flex: 1;
		display: flex;
		justify-content: space-between;
		align-items: end;
		gap: 20px;
	}

	.eyebrow {
		display: inline-block;
		font-size: 0.72rem;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: #ffb06b;
		margin-bottom: 8px;
	}

	.identity h1 {
		margin: 0;
		color: #ff9a3d;
		font-size: clamp(2rem, 4vw, 3rem);
		line-height: 1.1;
	}

	.username {
		margin: 8px 0 0;
		color: #ffb06b;
		font-size: 1rem;
	}

	.actions {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
	}

	.content-grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 16px;
		padding: 0 20px 20px;
	}

	.full-width {
		width: 100%;
	}

	.panel {
		background: rgba(22, 22, 22, 0.92);
		border: 1px solid rgba(255, 138, 29, 0.18);
		border-radius: 16px;
		padding: 18px;
	}

	.panel h2 {
		margin: 0 0 16px;
		font-size: 1.1rem;
		color: #ff9a3d;
	}

	.panel p {
		margin: 0;
		line-height: 1.7;
		color: #f6d2af;
	}

	.info-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 16px;
	}

	.info-list li {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 16px;
		padding-bottom: 10px;
		border-bottom: 1px solid rgba(255, 138, 29, 0.18);
	}

	.info-list li:last-child {
		border-bottom: none;
		padding-bottom: 0;
	}

	.info-list span {
		color: #ffb06b;
	}

	.info-list strong {
		color: #ffe3c8;
		font-weight: 600;
	}

	@media (max-width: 760px) {
		.page-shell {
			padding: 0 0 1rem;
		}
		.profile-card {
			border-radius: 12px;
			min-height: 0;
		}
		.cover {
			height: 120px;
		}
		.profile-header {
			display: flex;
			flex-direction: column;
			align-items: flex-start;
			gap: 12px;
			padding: 0 16px 16px;
			margin-top: -28px;
		}
		.avatar {
			width: 92px;
			height: 92px;
			border-radius: 14px;
		}
		.profile-meta {
			width: 100%;
			display: flex;
			flex-direction: column;
			align-items: flex-start;
			gap: 14px;
		}
		.actions {
			width: 100%;
		}
		.content-grid {
			display: flex;
			flex-direction: column;
			gap: 12px;
			padding: 0 16px 16px;
		}
		.panel {
			border-radius: 12px;
			padding: 16px;
		}
		.info-list li {
			flex-direction: column;
			align-items: flex-start;
		}
	}
</style>
