<script lang="ts">
  import { link, location } from 'svelte-spa-router';
  import { NAV_ITEMS } from '../routes';

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  const { open, onClose }: Props = $props();
</script>

<aside class="sidebar" class:open aria-label="Primary navigation">
  <div class="brand">
    <span class="brand-mark" aria-hidden="true">K</span>
    <span class="brand-name">Klyster</span>
  </div>

  <nav>
    <ul>
      {#each NAV_ITEMS as item (item.path)}
        <li>
          <a href={item.path} use:link class:active={$location === item.path} onclick={onClose}>
            <span class="icon" aria-hidden="true">{item.icon}</span>
            <span>{item.label}</span>
          </a>
        </li>
      {/each}
    </ul>
  </nav>
</aside>

<style>
  .sidebar {
    width: 240px;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    padding: 1.5rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    grid-area: sidebar;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 0.5rem;
  }

  .brand-mark {
    width: 2rem;
    height: 2rem;
    border-radius: 0.5rem;
    background: var(--color-accent);
    color: white;
    display: grid;
    place-items: center;
    font-weight: 700;
  }

  .brand-name {
    font-weight: 600;
    font-size: 1.125rem;
  }

  nav ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  nav a {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.75rem;
    border-radius: 0.5rem;
    color: var(--color-text);
    text-decoration: none;
    font-size: 0.9375rem;
    transition:
      background 120ms ease,
      color 120ms ease;
  }

  nav a:hover {
    background: var(--color-bg);
  }

  nav a.active {
    background: color-mix(in srgb, var(--color-accent) 15%, transparent);
    color: var(--color-accent);
    font-weight: 600;
  }

  .icon {
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 0.375rem;
    background: var(--color-bg);
    color: var(--color-muted);
    display: grid;
    place-items: center;
    font-size: 0.75rem;
    font-weight: 700;
  }

  nav a.active .icon {
    background: var(--color-accent);
    color: white;
  }

  @media (max-width: 768px) {
    .sidebar {
      position: fixed;
      inset: 0 auto 0 0;
      transform: translateX(-100%);
      transition: transform 200ms ease;
      z-index: 20;
      box-shadow: 4px 0 12px rgba(0, 0, 0, 0.1);
    }

    .sidebar.open {
      transform: translateX(0);
    }
  }
</style>
