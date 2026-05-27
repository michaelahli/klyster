<script lang="ts">
  interface Props {
    rows?: number;
    height?: string;
  }
  const { rows = 1, height = '6.5rem' }: Props = $props();
</script>

<div class="grid">
  {#each Array.from({ length: rows }) as _, idx (idx)}
    <div class="skeleton" style="height: {height}" aria-hidden="true"></div>
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }
  .skeleton {
    background: linear-gradient(
      100deg,
      var(--color-surface) 30%,
      color-mix(in srgb, var(--color-accent) 8%, var(--color-surface)) 50%,
      var(--color-surface) 70%
    );
    background-size: 200% 100%;
    border-radius: 0.75rem;
    border: 1px solid var(--color-border);
    animation: shimmer 1.4s ease-in-out infinite;
  }
  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
