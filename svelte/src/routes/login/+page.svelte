<script lang="ts">
	import { goto } from '$app/navigation';
	import { passwordLogin, passwordRegister, providerLoginUrl } from '$lib/auth';
	import * as m from '$lib/paraglide/messages.js';

	let mode = $state<'login' | 'register'>('login');
	let username = $state('');
	let email = $state('');
	let password = $state('');
	let passwordConfirmation = $state('');
	let error = $state('');
	let loading = $state(false);

	async function submit() {
		error = '';
		if (mode === 'register' && password !== passwordConfirmation) {
			error = m.login_error_password_mismatch();
			return;
		}
		loading = true;

		try {
			if (mode === 'register') {
				await passwordRegister(email, password, username);
			} else {
				await passwordLogin(email, password);
			}
			await goto('/');
		} catch (cause) {
			error = cause instanceof Error ? cause.message : m.login_error_failed();
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>{mode === 'login' ? m.login_title_sign_in() : m.login_title_create_account()} | Dionysus</title>
</svelte:head>

<main class="login-page">
	<section class="login-card" aria-labelledby="login-title">
		<p class="eyebrow">Dionysus</p>
		<h1 id="login-title">{mode === 'login' ? m.login_heading_welcome_back() : m.login_heading_create_account()}</h1>
		<p class="intro">
			{mode === 'login' ? m.login_intro_sign_in() : m.login_intro_register()}
		</p>

		<div class="social-login">
			<a class="social-button" href={providerLoginUrl('google')}>
				<span class="google-icon" aria-hidden="true">G</span>
				{m.login_continue_google()}
			</a>
			<a class="social-button" href={providerLoginUrl('discord')}>
				<span class="discord-icon" aria-hidden="true">◈</span>
				{m.login_continue_discord()}
			</a>
		</div>

		<div class="separator">
			<span>{mode === 'login' ? m.login_separator_sign_in() : m.login_separator_register()}</span>
		</div>

		<form
			onsubmit={(event) => {
				event.preventDefault();
				submit();
			}}
		>
			{#if mode === 'register'}
				<label for="username">{m.login_label_username()}</label>
				<input
					id="username"
					bind:value={username}
					type="text"
					autocomplete="username"
					minlength="3"
					maxlength="64"
					required
				/>
			{/if}

			<label for="email">{m.login_label_email()}</label>
			<input id="email" bind:value={email} type="email" autocomplete="email" required />

			<div class="password-label">
				<label for="password">{m.login_label_password()}</label>
				{#if mode === 'login'}<a href="/login">{m.login_forgot_password()}</a>{/if}
			</div>
			<input
				id="password"
				bind:value={password}
				type="password"
				autocomplete="current-password"
				minlength="8"
				required
			/>

			{#if mode === 'register'}
				<label for="password-confirmation">{m.login_label_password_confirm()}</label>
				<input
					id="password-confirmation"
					bind:value={passwordConfirmation}
					type="password"
					autocomplete="new-password"
					minlength="8"
					required
				/>
			{/if}

			{#if error}
				<p class="error" role="alert">{error}</p>
			{/if}

			<button class="submit-button" type="submit" disabled={loading}>
				{loading ? m.login_loading() : mode === 'login' ? m.login_submit_sign_in() : m.login_submit_create_account()}
			</button>
		</form>

		<p class="signup">
			{mode === 'login' ? m.login_switch_question_no_account() : m.login_switch_question_has_account()}
			<button
				type="button"
				onclick={() => {
					mode = mode === 'login' ? 'register' : 'login';
					error = '';
				}}
			>
				{mode === 'login' ? m.login_switch_create_account() : m.login_switch_sign_in()}
			</button>
		</p>
	</section>
</main>

<style>
	.login-page {
		min-height: calc(100vh - 8rem);
		display: grid;
		place-items: center;
		padding: 2rem 1rem;
	}

	.login-card {
		width: min(100%, 430px);
		padding: clamp(2rem, 5vw, 3.25rem);
		border: 1px solid rgb(255 138 29 / 30%);
		border-radius: 24px;
		background: rgb(15 12 17 / 96%);
		box-shadow: 0 24px 70px rgb(0 0 0 / 35%);
		color: #f9e6d3;
	}

	.eyebrow {
		margin: 0 0 0.45rem;
		color: #ffad5c;
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	h1 {
		margin: 0;
		color: #fff1e5;
		font-size: clamp(1.8rem, 5vw, 2.4rem);
	}

	.intro {
		margin: 0.75rem 0 1.75rem;
		color: #bca99b;
		line-height: 1.5;
	}

	.social-login {
		display: grid;
		gap: 0.7rem;
	}

	.social-button {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.7rem;
		padding: 0.85rem 1rem;
		border: 1px solid #49372b;
		border-radius: 10px;
		color: #f9e6d3;
		text-decoration: none;
		transition:
			border-color 0.2s,
			background 0.2s;
	}

	.social-button:hover,
	.social-button:focus-visible {
		border-color: #ff8a1d;
		background: #281d19;
	}

	.google-icon,
	.discord-icon {
		width: 1.25rem;
		color: #ffad5c;
		font-weight: 800;
		text-align: center;
	}

	.separator {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin: 1.5rem 0;
		color: #806f65;
		font-size: 0.78rem;
		text-align: center;
	}

	.separator::before,
	.separator::after {
		content: '';
		height: 1px;
		flex: 1;
		background: #3b2d26;
	}

	form {
		display: grid;
		gap: 0.55rem;
	}

	label,
	.password-label {
		color: #d3bdae;
		font-size: 0.85rem;
		font-weight: 600;
	}

	.password-label {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 0.45rem;
	}

	/* .password-label a,
	.signup a {
		color: #ffad5c;
		font-weight: 700;
		text-decoration: none;
	} */

	input {
		width: 100%;
		box-sizing: border-box;
		padding: 0.82rem 0.9rem;
		border: 1px solid #49372b;
		border-radius: 9px;
		background: #1d171a;
		color: #fff1e5;
		font: inherit;
	}

	input:focus {
		outline: 2px solid #ff8a1d;
		outline-offset: 1px;
		border-color: transparent;
	}

	.error {
		margin: 0.45rem 0 0;
		color: #ff9a8b;
		font-size: 0.86rem;
	}

	.submit-button {
		margin-top: 0.7rem;
		padding: 0.9rem 1rem;
		border: 0;
		border-radius: 10px;
		background: #ff8a1d;
		color: #170e08;
		font-weight: 800;
		cursor: pointer;
	}

	.submit-button:hover:not(:disabled),
	.submit-button:focus-visible {
		background: #ffad5c;
	}

	.submit-button:disabled {
		cursor: wait;
		opacity: 0.65;
	}

	.signup {
		display: flex;
		justify-content: center;
		gap: 0.3rem;
		margin: 1.5rem 0 0;
		color: #a89588;
		font-size: 0.86rem;
		text-align: center;
	}

	.signup button {
		padding: 0;
		border: 0;
		background: transparent;
		color: #ffad5c;
		font: inherit;
		font-weight: 700;
		cursor: pointer;
	}
</style>
