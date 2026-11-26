import { env } from '$env/dynamic/public';

const authUrl = (env.PUBLIC_AUTH_URL ?? '').replace(/\/$/, '');

export function providerLoginUrl(provider: 'google' | 'discord'): string {
	const callback = env.PUBLIC_AUTH_CALLBACK_URL ?? 'http://localhost:5173/auth/callback';
	const params = new URLSearchParams({ redirect_uri: callback });
	return `${authUrl}/auth/login/${provider}?${params.toString()}`;
}

export async function passwordLogin(email: string, password: string): Promise<void> {
	const response = await fetch(`${authUrl}/auth/login`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ email, password })
	});

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as { message?: string } | null;
		throw new Error(body?.message ?? 'Adresse email ou mot de passe incorrect.');
	}
}

export async function passwordRegister(
	email: string,
	password: string,
	username: string
): Promise<void> {
	const response = await fetch(`${authUrl}/auth/register`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ email, password, username })
	});

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as { message?: string } | null;
		throw new Error(body?.message ?? 'La création du compte a échoué.');
	}
}
