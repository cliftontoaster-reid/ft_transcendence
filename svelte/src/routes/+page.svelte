<!--
  +page.svelte - svelte
  Copyright (C) 2026 Clifton Toaster Reid
  
  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU Affero General Public License as
  published by the Free Software Foundation, either version 3 of the
  License, or (at your option) any later version.
  
  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
  GNU Affero General Public License for more details.
  
  You should have received a copy of the GNU Affero General Public License
  along with this program. If not, see <https://www.gnu.org/licenses/>.
 -->

 <script>
  import Overview from '$lib/components/Overview.svelte';
  import Chat from '$lib/components/Chat.svelte';
</script>

<div class="home-layout">
  <section class="overview-section">
    <Overview
      title="Titre"
      imageUrl="https://picsum.photos/1280/720"
      playHref="/watch_room"
    />
  </section>

  <aside class="chat-sidebar">
    <Chat />
  </aside>
</div>

<style>
  .home-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 340px);
    gap: 1.5rem;
    align-items: start;
  }

  .overview-section {
    width: 100%;
    padding: 2rem 0;
  }

  .chat-sidebar {
    justify-self: end;
    width: 100%;
    max-width: 340px;
    padding-top: 2rem;
  }

  @media (max-width: 800px) {
    .home-layout {
      grid-template-columns: minmax(0, 1fr);
    }

    .chat-sidebar {
      padding-top: 0;
    }
  }
</style>