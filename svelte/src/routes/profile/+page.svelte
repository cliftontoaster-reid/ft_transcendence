<script lang="ts">
    import { enhance } from '$app/forms';
    import ProfileCard from '$lib/components/ProfileCard.svelte';
    import { ageBrackets, ageLabels } from '$lib/profile';
    import type { ActionData, PageData } from './$types';

    let { data, form }: { data: PageData; form: ActionData } = $props();

    // Shape comes from `user.proto` (User) and the `users` table of db.dbml.
    const profile = $derived(data.profile);
    let editing = $state(false);

    // Live username check against UserService.CheckUsernameAvailability. Purely a hint:
    // the server still rejects a taken username on submit.
    type UsernameStatus = 'idle' | 'checking' | 'available' | 'taken' | 'invalid';
    let usernameStatus = $state<UsernameStatus>('idle');
    let timer: ReturnType<typeof setTimeout> | undefined;
    let controller: AbortController | undefined;

    function cancelUsernameCheck() {
        clearTimeout(timer);
        controller?.abort();
    }

    function checkUsername(event: Event) {
        const value = (event.currentTarget as HTMLInputElement).value.trim();
        cancelUsernameCheck();
        // empty, or unchanged: nothing to check (your own username is not "taken")
        if (!value || value === profile.username) {
            usernameStatus = 'idle';
            return;
        }
        usernameStatus = 'checking';
        timer = setTimeout(async () => {
            controller = new AbortController();
            try {
                const res = await fetch(
                    `/api/username-available?username=${encodeURIComponent(value)}`,
                    { signal: controller.signal }
                );
                if (res.status === 400) usernameStatus = 'invalid';
                else if (!res.ok) usernameStatus = 'idle';
                else usernameStatus = (await res.json()).available ? 'available' : 'taken';
            } catch (e) {
                if ((e as Error).name !== 'AbortError') usernameStatus = 'idle';
            }
        }, 400);
    }

    function toggleEditing() {
        editing = !editing;
        cancelUsernameCheck();
        usernameStatus = 'idle';
    }

    $effect(() => cancelUsernameCheck);

    const usernameHints: Partial<Record<UsernameStatus, string>> = {
        checking: 'Checking…',
        available: 'Username is available.',
        taken: 'This username is already taken.',
        invalid: 'Username must contain between 1 and 64 characters.'
    };
</script>

<svelte:head>
    <title>{profile.displayName} | Profile</title>
</svelte:head>

<ProfileCard {profile}>
    {#snippet actions()}
        <button class="primary" type="button" onclick={toggleEditing}>
            {editing ? 'Cancel' : 'Edit'}
        </button>
    {/snippet}

    {#if editing}
        <form
            class="edit-form"
            method="POST"
            action="?/update"
            use:enhance={() =>
                async ({ result, update }) => {
                    await update({ reset: false });
                    if (result.type === 'success') editing = false;
                }}
        >
            {#if form?.message}
                <p class="error" role="alert">{form.message}</p>
            {/if}

            <label>
                Display name
                <input
                    name="display_name"
                    maxlength="256"
                    required
                    value={form?.values?.display_name ?? profile.displayName}
                />
            </label>
            <label>
                Username
                <input
                    name="username"
                    maxlength="64"
                    value={form?.values?.username ?? profile.username}
                    oninput={checkUsername}
                />
                {#if usernameHints[usernameStatus]}
                    <span
                        class="hint"
                        class:error={usernameStatus === 'taken' || usernameStatus === 'invalid'}
                        class:ok={usernameStatus === 'available'}
                        aria-live="polite"
                    >
                        {usernameHints[usernameStatus]}
                    </span>
                {/if}
            </label>
            <label>
                Avatar URL
                <input
                    name="avatar_url"
                    maxlength="2048"
                    value={form?.values?.avatar_url ?? profile.avatarUrl}
                />
            </label>
            <label>
                Bio
                <textarea name="bio" maxlength="500">{form?.values?.bio ?? profile.bio}</textarea>
            </label>
            <label>
                Age bracket
                <select name="age">
                    {#each ageBrackets as bracket (bracket)}
                        <option
                            value={bracket}
                            selected={bracket === (form?.values?.age ?? profile.age)}
                        >
                            {ageLabels[bracket]}
                        </option>
                    {/each}
                </select>
            </label>

            <div class="form-actions">
                <button class="primary" type="submit">Save</button>
            </div>
        </form>
    {/if}
</ProfileCard>

<style>
    * {
        box-sizing: border-box;
    }

button {
        border: none;
        border-radius: 999px;
        padding: 0.9rem 1.4rem;
        font-weight: 700;
        cursor: pointer;
        transition: transform 0.15s ease;
    }

button:hover {
        transform: translateY(-1px);
    }

.primary {
        background: #ff8a1d;
        color: #120d09;
    }

@media (max-width: 760px) {
button {
            width: 100%;
        }
}

.edit-form {
        display: grid;
        gap: 14px;
        padding: 0 20px 20px;
    }

.edit-form label {
        display: grid;
        gap: 6px;
        font-size: 0.85rem;
        opacity: 0.85;
    }

.edit-form input,
    .edit-form textarea,
    .edit-form select {
        font: inherit;
        color: inherit;
        background: #111;
        border: 1px solid rgba(255, 138, 29, 0.25);
        border-radius: 10px;
        padding: 10px 12px;
    }

.edit-form textarea {
        min-height: 96px;
        resize: vertical;
    }

.form-actions {
        display: flex;
        gap: 10px;
    }

.error {
        color: #ff6b6b;
        margin: 0;
    }

    .hint {
        font-size: 0.8rem;
        opacity: 0.85;
    }

    .hint.ok {
        color: #6bd68a;
    }
</style>
