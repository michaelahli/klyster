<script lang="ts">
  import Router from 'svelte-spa-router';
  import { ROUTES } from '../routes';
  import Sidebar from './Sidebar.svelte';
  import TopBar from './TopBar.svelte';
  import ErrorBoundary from './ErrorBoundary.svelte';

  let sidebarOpen = $state(false);

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
  }

  function closeSidebar() {
    sidebarOpen = false;
  }
</script>

<div class="layout">
  <Sidebar open={sidebarOpen} onClose={closeSidebar} />
  <TopBar onToggleSidebar={toggleSidebar} />

  {#if sidebarOpen}
    <button type="button" class="scrim" aria-label="Close navigation" onclick={closeSidebar}
    ></button>
  {/if}

  <main class="content">
    <ErrorBoundary>
      <Router routes={ROUTES} />
    </ErrorBoundary>
  </main>
</div>

<style>
  .layout {
    display: grid;
    grid-template-areas:
      'sidebar topbar'
      'sidebar content';
    grid-template-columns: 240px 1fr;
    grid-template-rows: auto 1fr;
    min-height: 100vh;
  }

  .content {
    grid-area: content;
    padding: 2rem;
    overflow-x: auto;
  }

  .scrim {
    display: none;
  }

  @media (max-width: 768px) {
    .layout {
      grid-template-columns: 1fr;
      grid-template-areas:
        'topbar'
        'content';
    }

    .scrim {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      border: none;
      cursor: pointer;
      z-index: 15;
    }
  }
</style>
