<script lang="ts">
  /**
   * Metric tile with an optional gauge for percentage values.
   *
   * Display rules:
   * - Values in [0, 1]: rendered as "{n}%" with a horizontal gauge.
   *   Color tracks: green < 70%, yellow < 85%, red >= 85%.
   * - Other values: rendered as a formatted number with the supplied unit.
   *
   * Updated timestamps are shown as relative ("12s ago") so an idle dashboard
   * makes it obvious when data went stale.
   */
  interface Props {
    name: string;
    value: number | null;
    /** Optional unit suffix when not displaying as a percentage. */
    unit?: string;
    /** RFC3339 timestamp of the latest sample. */
    timestamp?: string;
    /** When `true`, render a percentage gauge regardless of value range. */
    asPercentage?: boolean;
  }

  const { name, value, unit, timestamp, asPercentage }: Props = $props();

  const isPercentage = $derived(asPercentage || (value !== null && value >= 0 && value <= 1));

  const percent = $derived(value === null ? null : Math.round(value * 100));

  const status = $derived.by(() => {
    if (percent === null) return 'unknown';
    if (percent < 70) return 'ok';
    if (percent < 85) return 'warn';
    return 'crit';
  });

  const display = $derived.by(() => {
    if (value === null) return '—';
    if (isPercentage) return `${percent}%`;
    const fixed = Math.abs(value) >= 100 ? value.toFixed(0) : value.toFixed(2);
    return unit ? `${fixed} ${unit}` : fixed;
  });

  const relative = $derived.by(() => {
    if (!timestamp) return null;
    const diff = Date.now() - new Date(timestamp).getTime();
    if (Number.isNaN(diff) || diff < 0) return null;
    if (diff < 1000) return 'just now';
    if (diff < 60_000) return `${Math.round(diff / 1000)}s ago`;
    if (diff < 3_600_000) return `${Math.round(diff / 60_000)}m ago`;
    return `${Math.round(diff / 3_600_000)}h ago`;
  });
</script>

<article class="card status-{status}">
  <header>
    <h3>{name}</h3>
    {#if relative}
      <span class="updated" title={timestamp}>{relative}</span>
    {/if}
  </header>

  <div class="value">{display}</div>

  {#if isPercentage && percent !== null}
    <div
      class="gauge"
      role="progressbar"
      aria-valuenow={percent}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-label={`${name} utilization`}
    >
      <div class="gauge-fill" style="width: {Math.min(percent, 100)}%"></div>
    </div>
  {/if}
</article>

<style>
  .card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 0.75rem;
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 7rem;
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }

  h3 {
    margin: 0;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .updated {
    font-size: 0.6875rem;
    color: var(--color-muted);
    font-family: var(--font-mono);
  }

  .value {
    font-size: 1.875rem;
    font-weight: 600;
    line-height: 1.1;
  }

  .gauge {
    height: 0.375rem;
    background: var(--color-bg);
    border-radius: 999px;
    overflow: hidden;
  }

  .gauge-fill {
    height: 100%;
    border-radius: inherit;
    transition: width 200ms ease;
  }

  .status-ok .gauge-fill {
    background: #16a34a;
  }
  .status-warn .gauge-fill {
    background: #f59e0b;
  }
  .status-crit .gauge-fill {
    background: #dc2626;
  }
  .status-unknown .gauge-fill {
    background: var(--color-muted);
  }

  .status-warn .value {
    color: #c2630a;
  }
  .status-crit .value {
    color: #b91c1c;
  }
</style>
