/*
 * hooks.server.ts - svelte
 * Copyright (C) 2026 Clifton Toaster Reid
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

import type { Handle } from "@sveltejs/kit";
import { sequence } from "@sveltejs/kit/hooks";
import { dev } from "$app/environment";
import { env } from "$env/dynamic/private";
import { env as publicEnv } from "$env/dynamic/public";
import { isUuid } from "$lib/uuid";
import { getTextDirection } from "$lib/paraglide/runtime";
import { paraglideMiddleware } from "$lib/paraglide/server";

const handleParaglide: Handle = ({ event, resolve }) =>
	paraglideMiddleware(event.request, ({ request, locale }) => {
		event.request = request;

		return resolve(event, {
			transformPageChunk: ({ html }) =>
				html
					.replace("%paraglide.lang%", locale)
					.replace("%paraglide.dir%", getTextDirection(locale)),
		});
	});

const handleAuth: Handle = async ({ event, resolve }) => {
	const sessionCookie = event.cookies.get('dionysus_session');
	if (sessionCookie) {
		const apiUrl = (publicEnv.PUBLIC_API_URL ?? 'http://localhost:8000').replace(/\/$/, '');
		const response = await event.fetch(`${apiUrl}/auth/session`, {
			headers: { cookie: `dionysus_session=${encodeURIComponent(sessionCookie)}` }
		});
		if (response.ok) {
			const body = (await response.json()) as { userId?: unknown };
			if (typeof body.userId !== 'string' || !isUuid(body.userId)) {
				throw new Error('Authentication service returned an invalid session.');
			}
			event.locals.userId = body.userId;
		} else if (response.status !== 401) {
			throw new Error(`Authentication service failed to validate the session (${response.status}).`);
		}
	}

	if (!event.locals.userId && dev && env.DEV_USER_ID && isUuid(env.DEV_USER_ID)) {
		event.locals.userId = env.DEV_USER_ID;
	}
	return resolve(event);
};

export const handle: Handle = sequence(handleAuth, handleParaglide);
