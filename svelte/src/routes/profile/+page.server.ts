import { error, fail } from '@sveltejs/kit';
import { Code, ConnectError } from '@connectrpc/connect';
import { userClient } from '$lib/server/grpc';
import { AgeBracket } from '$lib/proto/dionysus/user/v1/user_pb';
import { toProfile } from '$lib/server/profile';
import { uuidToBytes } from '$lib/uuid';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
	if (!locals.userId) error(401, 'Not authenticated');

	try {
		const { user } = await userClient.getProfile({
			userId: { value: { value: uuidToBytes(locals.userId) } }
		});
		if (!user) error(404, 'User not found');
		return { profile: toProfile(user) };
	} catch (e) {
		if (e instanceof ConnectError) {
			if (e.code === Code.NotFound) error(404, 'User not found');
			if (e.code === Code.Unauthenticated) error(401, 'Not authenticated');
			console.error('UserService.GetProfile failed', e);
			error(502, 'User service unavailable');
		}
		throw e;
	}
};

export const actions: Actions = {
	update: async ({ request, locals }) => {
		if (!locals.userId) error(401, 'Not authenticated');

		const data = await request.formData();
		const text = (key: string) => String(data.get(key) ?? '').trim();
		const values = {
			username: text('username'),
			display_name: text('display_name'),
			bio: text('bio'),
			avatar_url: text('avatar_url'),
			age: Number(data.get('age') ?? AgeBracket.UNSPECIFIED)
		};

		try {
			const { user } = await userClient.updateProfile(
				{
					// the server rejects empty username / display name, so only send them when filled
					username: values.username || undefined,
					displayName: values.display_name || undefined,
					bio: values.bio,
					avatarUrl: values.avatar_url,
					age: values.age
				},
				// the user is identified by metadata, never by a field the client controls
				{ headers: { 'user-id': locals.userId } }
			);
			return { success: true, profile: user ? toProfile(user) : undefined };
		} catch (e) {
			if (e instanceof ConnectError) {
				if (e.code === Code.AlreadyExists) {
					return fail(409, { message: 'This username is already taken.', values });
				}
				if (e.code === Code.InvalidArgument) {
					return fail(400, { message: e.rawMessage, values });
				}
				console.error('UserService.UpdateProfile failed', e);
				error(502, 'User service unavailable');
			}
			throw e;
		}
	}
};
