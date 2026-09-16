/*
 * Welcome.svelte.spec.ts - svelte
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

import { page } from "vitest/browser";
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-svelte";
import Welcome from "./Welcome.svelte";

describe("Welcome.svelte", () => {
	it("renders greetings for host and guest", async () => {
		render(Welcome, { host: "SvelteKit", guest: "Vitest" });

		await expect
			.element(page.getByRole("heading", { level: 1 }))
			.toHaveTextContent("Hello, SvelteKit!");
		await expect.element(page.getByText("Hello, Vitest!")).toBeInTheDocument();
	});
});
