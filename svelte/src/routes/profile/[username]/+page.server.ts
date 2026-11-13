import { error, redirect } from '@sveltejs/kit';
import { Code, ConnectError } from '@connectrpc/connect';
import { userClient } from '$lib/server/grpc';
import { toProfile } from '$lib/server/profile';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
	if (!locals.userId) error(401, 'Not authenticated');

	try {
		const { user } = await userClient.getProfileByUsername({ username: params.username });
		if (!user) error(404, 'User not found');

		const profile = toProfile(user);
		// your own profile lives at /profile, where it can be edited
		if (profile.id === locals.userId) redirect(302, '/profile');
		return { profile };
	} catch (e) {
		if (e instanceof ConnectError) {
			// an over-long username is rejected as invalid: it cannot exist, so it is a 404
			if (e.code === Code.NotFound || e.code === Code.InvalidArgument) {
				error(404, 'User not found');
			}
			console.error('UserService.GetProfileByUsername failed', e);
			error(502, 'User service unavailable');
		}
		throw e;
	}
};
