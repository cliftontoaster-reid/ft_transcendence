import { redirect } from '@sveltejs/kit';
import { userClient } from '$lib/server/grpc';
import { toProfile } from '$lib/server/profile';
import { uuidToBytes } from '$lib/uuid';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ locals, url }) => {
	if (!locals.userId) return { isLoggedIn: false, user: null };

	try {
		const { user } = await userClient.getProfile({
			userId: { value: { value: uuidToBytes(locals.userId) } }
		});
		const profile = user ? toProfile(user) : null;
		if (
			!profile?.username.trim() &&
			(url.pathname !== '/profile' || url.searchParams.get('setup') !== 'username')
		) {
			redirect(303, '/profile?setup=username');
		}
		return { isLoggedIn: true, user: profile };
	} catch (e) {
		if (e instanceof Response) throw e;
		// Session valide mais service profil indisponible : on reste "connecté"
		// pour ne pas afficher "Connexion" à tort, sans casser tout le site.
		console.error('Layout: impossible de charger le profil', e);
		return { isLoggedIn: true, user: null };
	}
};
