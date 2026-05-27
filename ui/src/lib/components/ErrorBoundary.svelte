<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
  }

  const { children }: Props = $props();

  let error = $state<Error | null>(null);

  // Surface uncaught errors that bubble out of children. Svelte 5 throws
  // into the error boundary primitive; until that lands as stable, we lean
  // on window-level capture for routing-induced errors.
  if (typeof window !== 'undefined') {
    window.addEventListener('error', (event) => {
      error = event.error ?? new Error(event.message);
    });
    window.addEventListener('unhandledrejection', (event) => {
      const reason = event.reason;
      error = reason instanceof Error ? reason : new Error(String(reason));
    });
  }

  function reset() {
    error = null;
    if (typeof window !== 'undefined') {
      window.location.reload();
    }
  }
</script>

{#if error}
  <div class="error" role="alert">
    <h2>Something went wrong</h2>
    <p>{error.message}</p>
    <button type="button" onclick={reset}>Reload</button>
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .error {
    background: color-mix(in srgb, #ef4444 12%, transparent);
    border: 1px solid #ef4444;
    border-radius: 0.5rem;
    padding: 1.25rem;
    color: var(--color-text);
  }

  .error h2 {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
  }

  .error p {
    margin: 0 0 1rem;
    color: var(--color-muted);
    font-family: var(--font-mono);
    font-size: 0.875rem;
  }

  button {
    background: var(--color-accent);
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-family: inherit;
    cursor: pointer;
  }
</style>
