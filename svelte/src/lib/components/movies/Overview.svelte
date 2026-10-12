<script lang="ts">
	import { Refresh, VolumeOff, VolumeUp } from '@material-symbols-svg/svelte';
	import type { Component } from 'svelte';

	interface Button {
		icon?: Component;
		text: string;
		onClick: () => void;
	}

	interface MediaSource {
		image_src: URL;
		video_src?: URL;
	}

	interface Props {
		logo: URL;
		logo_msg: string;

		tags: string[];
		description: string;

		buttons: Button[];

		desktop: MediaSource;
		mobile?: MediaSource;
	}

	let { logo, logo_msg, tags, description, buttons, desktop, mobile }: Props = $props();

	let desktopEnded = $state(false);
	let mobileEnded = $state(false);

	let muted = $state(true);

	let desktopVideo: HTMLVideoElement | undefined = $state();
	let mobileVideo: HTMLVideoElement | undefined = $state();

	function handleDesktopEnd() {
		desktopEnded = true;
	}

	function handleMobileEnd() {
		mobileEnded = true;
	}
	function toggleMute() {
		muted = !muted;
		if (desktopVideo) {
			desktopVideo.muted = muted;
		}
		if (mobileVideo) {
			mobileVideo.muted = muted;
		}
	}

	function replayDesktop() {
		if (desktopVideo) {
			desktopEnded = false;
			desktopVideo.currentTime = 0;
			desktopVideo.play();
		}
	}
	function replayMobile() {
		if (mobileVideo) {
			mobileEnded = false;
			mobileVideo.currentTime = 0;
			mobileVideo.play();
		}
	}
</script>

<!-- Define the single reusable button block -->
{#snippet mediaButton(videoSrc: URL | undefined, ended: boolean, replayFn: () => void)}
	{#if videoSrc}
		<button
			onclick={ended ? replayFn : toggleMute}
			class="media-control-btn"
			aria-label={ended ? 'Replay video' : muted ? 'Unmute video' : 'Mute video'}
		>
			{#if ended}
				<Refresh width="18px" height="18px" class="icon" />
			{:else if muted}
				<VolumeOff width="18px" height="18px" class="icon" />
			{:else}
				<VolumeUp width="18px" height="18px" class="icon" />
			{/if}
		</button>
	{/if}
{/snippet}

<div class="overview-container">
	<!-- ================= DESKTOP VIEW ================= -->
	<div class="media-container desktop-only">
		{#if desktop.video_src && !desktopEnded}
			<video
				class="media-asset"
				bind:this={desktopVideo}
				src={desktop.video_src.href}
				autoplay
				muted={muted}
				playsinline
				onended={handleDesktopEnd}
			></video>
		{:else}
			<img src={desktop.image_src.href} alt="" class="media-asset" />
		{/if}

		<div class="media-gradient"></div>

		<div class="media-info">
			<enhanced:img class="logo" src={logo} alt={logo_msg} />

			<div class="tags">
				{tags.join(' · ')}
			</div>

			<p class="description">{description}</p>

			<div class="actions">
				{#each buttons as button}
					<button type="button" onclick={button.onClick}>
						{#if button.icon}
							<button.icon width="20px" height="20px" />
						{/if}
						{button.text}
					</button>
				{/each}
			</div>
		</div>

		{@render mediaButton(desktop.video_src, desktopEnded, replayDesktop)}
	</div>

	<!-- ================= MOBILE VIEW ================= -->
	<div class="media-container mobile-only">
		{#if mobile?.video_src && !mobileEnded}
			<video
				class="media-asset"
				bind:this={mobileVideo}
				src={mobile.video_src.href}
				autoplay
				muted={muted}
				playsinline
				onended={handleMobileEnd}
			></video>
		{:else if mobile}
			<img src={mobile.image_src.href} alt="" class="media-asset" />
		{:else}
			<img src={desktop.image_src.href} alt="" class="media-asset" />
		{/if}

		<div class="media-gradient"></div>

		<div class="media-info">
			<enhanced:img class="logo" src={logo} alt={logo_msg} />

			<div class="tags">
				{tags.join(' · ')}
			</div>

			<p class="description">{description}</p>

			<div class="actions">
				{#each buttons as button}
					<button type="button" onclick={button.onClick}>
						{#if button.icon}
							<button.icon width="20px" height="20px" />
						{/if}
						{button.text}
					</button>
				{/each}
			</div>
		</div>

		{@render mediaButton(mobile?.video_src, mobileEnded, replayMobile)}
	</div>
</div>

<style lang="scss">
	.overview-container {
		position: relative;
		width: 100%;
		min-height: clamp(520px, 70vh, 800px);
		overflow: hidden;
		background: #000;
		display: grid;
		grid-template-areas: 'layer';
	}

	.media-container {
		position: relative;
		display: grid;
		grid-template-areas: 'media';
		min-height: inherit;
	}

	.media-asset,
	.media-gradient,
	.media-info,
	.media-control-btn {
		grid-area: media;
	}

	.media-asset {
		z-index: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: center;
	}

	.media-gradient {
		z-index: 1;
		background:
			linear-gradient(90deg, rgba(0, 0, 0, 0.9) 0%, rgba(0, 0, 0, 0.45) 42%, transparent 75%),
			linear-gradient(0deg, #000 0%, transparent 45%);
	}

	.media-info {
		z-index: 2;
		align-self: end;
		max-width: 560px;
		padding: 0 5vw 5rem;
		color: #fff;
	}

	.logo {
		display: block;
		width: min(32rem, 70%);
		max-height: 11rem;
		margin-bottom: 1rem;
		object-fit: contain;
		object-position: left bottom;
	}

	.tags {
		margin-bottom: 1rem;
		color: #ddd;
		font-size: 0.9rem;
	}

	.description {
		margin: 0 0 1.5rem;
		color: #ddd;
		line-height: 1.5;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.actions button {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1.25rem;
		border: 0;
		border-radius: 0.25rem;
		cursor: pointer;
		font-weight: 700;
	}

	.media-control-btn {
		z-index: 3;
		align-self: end;
		justify-self: end;
		margin: 0 5vw 2rem 0;
	}

	.mobile-only {
		display: grid;
	}
	.desktop-only {
		display: none;
	}

	@media (min-width: 768px) {
		.mobile-only {
			display: none;
		}
		.desktop-only {
			display: grid;
		}
	}

	@media (max-width: 767px) {
		.overview-container,
		.media-container {
			min-height: 620px;
		}

		.media-gradient {
			background:
				linear-gradient(0deg, #000 8%, rgba(0, 0, 0, 0.35) 65%, transparent),
				linear-gradient(90deg, rgba(0, 0, 0, 0.25), transparent);
		}

		.media-info {
			align-self: end;
			padding: 0 1.25rem 4rem;
		}

		.logo {
			width: 85%;
			max-height: 8rem;
		}

		.description {
			font-size: 0.9rem;
		}

		.media-control-btn {
			margin: 0 1.25rem 1.5rem 0;
		}
	}
</style>
