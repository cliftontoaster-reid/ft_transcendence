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

// Sets `event.locals.userId`, the user the gRPC calls act on behalf of.
// TODO: replace with the real session lookup (sessions table / OIDC login).
// Until then the id only comes from DEV_USER_ID, and only in dev mode, so a
// production build can never be tricked into acting as someone else.
const handleAuth: Handle = ({ event, resolve }) => {
	if (dev && env.DEV_USER_ID && isUuid(env.DEV_USER_ID)) {
		event.locals.userId = env.DEV_USER_ID;
	}
	return resolve(event);
};

export const handle: Handle = sequence(handleAuth, handleParaglide);
