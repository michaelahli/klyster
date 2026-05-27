<script lang="ts">
  import { link } from 'svelte-spa-router';
</script>

<section class="not-found">
  <h1>404</h1>
  <p>The page you were looking for does not exist.</p>
  <a href="/" use:link>Back to dashboard</a>
</section>

<style>
  .not-found {
    text-align: center;
    padding: 4rem 2rem;
  }
  h1 {
    font-size: 4rem;
    margin: 0 0 1rem;
    color: var(--color-accent);
  }
  p {
    margin: 0 0 1.5rem;
    color: var(--color-muted);
  }
  a {
    color: var(--color-accent);
    text-decoration: none;
    font-weight: 600;
  }
  a:hover {
    text-decoration: underline;
  }
</style>
