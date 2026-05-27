<script lang="ts">
  interface Props {
    label?: string;
  }

  const { label = 'Loading' }: Props = $props();
</script>

<div class="loading" role="status" aria-live="polite">
  <span class="spinner" aria-hidden="true"></span>
  <span>{label}</span>
</div>

<style>
  .loading {
    display: inline-flex;
    align-items: center;
    gap: 0.625rem;
    color: var(--color-muted);
    font-size: 0.9375rem;
  }

  .spinner {
    width: 1rem;
    height: 1rem;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 700ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
