import { error, json } from '@sveltejs/kit';
import { Code, ConnectError } from '@connectrpc/connect';
import { userClient } from '$lib/server/grpc';
import type { RequestHandler } from './$types';

/** `GET /api/username-available?username=foo` -> `{ available: boolean }` */
export const GET: RequestHandler = async ({ url, locals }) => {
	if (!locals.userId) error(401, 'Not authenticated');

	const username = (url.searchParams.get('username') ?? '').trim();
	if (!username)
		return json({ available: false, message: 'username is required' }, { status: 400 });

	try {
		const { available } = await userClient.checkUsernameAvailability({ username });
		return json({ available });
	} catch (e) {
		if (e instanceof ConnectError) {
			if (e.code === Code.InvalidArgument) {
				return json({ available: false, message: e.rawMessage }, { status: 400 });
			}
			console.error('UserService.CheckUsernameAvailability failed', e);
			error(502, 'User service unavailable');
		}
		throw e;
	}
};
