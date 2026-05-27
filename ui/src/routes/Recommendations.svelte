<script lang="ts">
  import LoadingIndicator from '../lib/components/LoadingIndicator.svelte';
</script>

<section>
  <header>
    <h1>Recommendations</h1>
    <p>Pending scaling recommendations and approval flow arrive with CP-M5-006.</p>
  </header>
  <div class="placeholder">
    <LoadingIndicator label="Awaiting CP-M5-006" />
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  header h1 {
    margin: 0 0 0.5rem;
    font-size: 1.5rem;
  }
  header p {
    margin: 0;
    color: var(--color-muted);
  }
  .placeholder {
    background: var(--color-surface);
    border: 1px dashed var(--color-border);
    border-radius: 0.75rem;
    padding: 2rem;
    display: grid;
    place-items: center;
  }
</style>
