<script lang="ts">
  import { theme, cycleTheme, type Theme } from '../stores/theme';

  interface Props {
    onToggleSidebar: () => void;
  }

  const { onToggleSidebar }: Props = $props();

  const themeLabel: Record<Theme, string> = {
    auto: 'Auto',
    light: 'Light',
    dark: 'Dark',
  };

  function onCycleTheme() {
    theme.update(cycleTheme);
  }
</script>

<header class="topbar">
  <button type="button" class="hamburger" aria-label="Toggle navigation" onclick={onToggleSidebar}>
    <span></span>
    <span></span>
    <span></span>
  </button>

  <h1>Capacity Planning</h1>

  <div class="actions">
    <button
      type="button"
      class="theme-toggle"
      aria-label="Switch theme"
      title={`Theme: ${themeLabel[$theme]}`}
      onclick={onCycleTheme}
    >
      {themeLabel[$theme]}
    </button>
    <span class="user-placeholder" aria-label="User menu placeholder">—</span>
  </div>
</header>

<style>
  .topbar {
    grid-area: topbar;
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1.5rem;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
  }

  h1 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-muted);
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .theme-toggle,
  .hamburger,
  .user-placeholder {
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    color: var(--color-text);
    padding: 0.4rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.8125rem;
    cursor: pointer;
    font-family: inherit;
  }

  .theme-toggle:hover,
  .hamburger:hover {
    border-color: var(--color-accent);
  }

  .user-placeholder {
    cursor: default;
    color: var(--color-muted);
    min-width: 2rem;
    text-align: center;
  }

  .hamburger {
    display: none;
    flex-direction: column;
    gap: 3px;
    width: 36px;
    padding: 8px 6px;
  }

  .hamburger span {
    display: block;
    height: 2px;
    background: var(--color-text);
    border-radius: 1px;
  }

  @media (max-width: 768px) {
    .hamburger {
      display: flex;
    }
  }
</style>
