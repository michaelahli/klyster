<script lang="ts">
  // Placeholder shell. Layout, routing, and pages land in CP-M5-003.
  const version = '0.1.0';
</script>

<main>
  <header>
    <h1>Klyster</h1>
    <p class="tagline">Capacity planning for Kubernetes and VM workloads</p>
  </header>

  <section class="status">
    <p>UI scaffold ready. Dashboard pages arrive with CP-M5-003.</p>
    <p class="version">v{version}</p>
  </section>
</main>

<style>
  main {
    max-width: 720px;
    margin: 0 auto;
    padding: 4rem 1.5rem;
    text-align: center;
  }

  header h1 {
    margin: 0;
    font-size: 2.5rem;
    letter-spacing: -0.02em;
  }

  .tagline {
    color: var(--color-muted);
    margin-top: 0.5rem;
  }

  .status {
    margin-top: 3rem;
    padding: 1.5rem;
    border-radius: 0.75rem;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
  }

  .version {
    margin-top: 1rem;
    color: var(--color-muted);
    font-family: var(--font-mono);
    font-size: 0.875rem;
  }
</style>
