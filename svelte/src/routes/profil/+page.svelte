<script lang="ts">
    type AgeBracket =
        | 'AGE_BRACKET_UNSPECIFIED'
        | 'AGE_BRACKET_PEGI_3'
        | 'AGE_BRACKET_PEGI_7'
        | 'AGE_BRACKET_PEGI_12'
        | 'AGE_BRACKET_PEGI_16'
        | 'AGE_BRACKET_PEGI_18';

    type Profile = {
        id: string;
        username: string;
        display_name: string;
        bio: string;
        avatar_url: string;
        age?: AgeBracket;
    };

    const profile: Profile = {
        id: 'user_001',
        username: 'utilisateur',
        display_name: 'Utilisateur',
        bio: 'Profil utilisateur.',
        avatar_url: '',
        age: 'AGE_BRACKET_PEGI_16'
    };

    const labels: Record<AgeBracket, string> = {
        AGE_BRACKET_UNSPECIFIED: 'Non précisé',
        AGE_BRACKET_PEGI_3: 'PEGI 3',
        AGE_BRACKET_PEGI_7: 'PEGI 7',
        AGE_BRACKET_PEGI_12: 'PEGI 12',
        AGE_BRACKET_PEGI_16: 'PEGI 16',
        AGE_BRACKET_PEGI_18: 'PEGI 18'
    };
</script>

<svelte:head>
    <title>{profile.display_name} | Profil</title>
</svelte:head>

<main class="page-shell">
    <section class="profile-card">
        <div class="cover"></div>

        <div class="profile-header">
            <img class="avatar" src={profile.avatar_url} alt={profile.display_name} />

            <div class="profile-meta">
                <div class="identity">
                    <span class="eyebrow">Compte</span>
                    <h1>{profile.display_name}</h1>
                    <p class="username">@{profile.username}</p>
                </div>

                <div class="actions">
                    <button class="primary" type="button">Modifier</button>
                </div>
            </div>
        </div>

        <div class="content-grid">
            <section class="panel full-width">
                <h2>Bio</h2>
                <p>{profile.bio}</p>
            </section>

            <section class="panel full-width">
                <h2>Informations</h2>
                <ul class="info-list">
                    <li>
                        <span>ID</span>
                        <strong>{profile.id}</strong>
                    </li>
                    <li>
                        <span>Nom d’utilisateur</span>
                        <strong>{profile.username}</strong>
                    </li>
                    <li>
                        <span>Classe d’âge</span>
                        <strong>{labels[profile.age ?? 'AGE_BRACKET_UNSPECIFIED']}</strong>
                    </li>
                </ul>
            </section>
        </div>
    </section>
</main>

<style>
    :global(body) {
        margin: 0;
        background: #050505;
        color: #ff8a1d;
        font-family: 'Segoe UI', sans-serif;
    }

    * {
        box-sizing: border-box;
    }

    .page-shell {
        min-height: 100vh;
        display: flex;
        align-items: stretch;
        justify-content: center;
        padding: 0;
        background: #050505;
    }

    .profile-card {
        width: 100%;
        min-height: 100vh;
        background: rgba(18, 18, 18, 0.96);
        border: 1px solid rgba(255, 138, 29, 0.2);
        border-radius: 0;
        overflow: hidden;
        box-shadow: none;
    }

    .cover {
        height: 180px;
        background: linear-gradient(135deg, #ff8a1d 0%, #6a2f00 100%);
    }

    .profile-header {
        display: flex;
        gap: 20px;
        padding: 0 20px 20px;
        margin-top: -42px;
        align-items: end;
    }

    .avatar {
        width: 128px;
        height: 128px;
        border-radius: 18px;
        object-fit: cover;
        border: 3px solid #050505;
        background: #111;
    }

    .profile-meta {
        flex: 1;
        display: flex;
        justify-content: space-between;
        align-items: end;
        gap: 20px;
    }

    .eyebrow {
        display: inline-block;
        font-size: 0.72rem;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: #ffb06b;
        margin-bottom: 8px;
    }

    .identity h1 {
        margin: 0;
        color: #ff9a3d;
        font-size: clamp(2rem, 4vw, 3rem);
        line-height: 1.1;
    }

    .username {
        margin: 8px 0 0;
        color: #ffb06b;
        font-size: 1rem;
    }

    .actions {
        display: flex;
        gap: 12px;
        flex-wrap: wrap;
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

    .content-grid {
        display: grid;
        grid-template-columns: 1fr;
        gap: 16px;
        padding: 0 20px 20px;
    }

    .full-width {
        width: 100%;
    }

    .panel {
        background: rgba(22, 22, 22, 0.92);
        border: 1px solid rgba(255, 138, 29, 0.18);
        border-radius: 16px;
        padding: 18px;
    }

    .panel h2 {
        margin: 0 0 16px;
        font-size: 1.1rem;
        color: #ff9a3d;
    }

    .panel p {
        margin: 0;
        line-height: 1.7;
        color: #f6d2af;
    }

    .info-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: grid;
        gap: 16px;
    }

    .info-list li {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 16px;
        padding-bottom: 10px;
        border-bottom: 1px solid rgba(255, 138, 29, 0.18);
    }

    .info-list li:last-child {
        border-bottom: none;
        padding-bottom: 0;
    }

    .info-list span {
        color: #ffb06b;
    }

    .info-list strong {
        color: #ffe3c8;
        font-weight: 600;
    }

    @media (max-width: 760px) {
        .cover {
            height: 120px;
        }

        .profile-header {
            display: flex;
            flex-direction: column;
            align-items: flex-start;
            gap: 12px;
            padding: 0 16px 16px;
            margin-top: -28px;
        }

        .avatar {
            width: 92px;
            height: 92px;
            border-radius: 14px;
        }

        .profile-meta {
            width: 100%;
            display: flex;
            flex-direction: column;
            align-items: flex-start;
            gap: 14px;
        }

        .actions {
            width: 100%;
        }

        button {
            width: 100%;
        }

        .content-grid {
            display: flex;
            flex-direction: column;
            gap: 12px;
            padding: 0 16px 16px;
        }

        .panel {
            border-radius: 12px;
            padding: 16px;
        }

        .info-list li {
            flex-direction: column;
            align-items: flex-start;
        }
    }
</style>