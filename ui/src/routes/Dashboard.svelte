<script lang="ts">
  import { onMount } from 'svelte';
  import LoadingIndicator from '../lib/components/LoadingIndicator.svelte';
  import MetricCard from '../lib/components/MetricCard.svelte';
  import Skeleton from '../lib/components/Skeleton.svelte';
  import TimeSeriesChart from '../lib/components/TimeSeriesChart.svelte';
  import { ApiError } from '../lib/api/client';
  import {
    listLatestMetrics,
    queryMetric,
    type LatestMetric,
    type MetricDataPoint,
  } from '../lib/api/metrics';

  /** Auto-refresh cadence; matches the ticket spec. */
  const REFRESH_MS = 30_000;

  /** Names that render as percentage gauges; non-matching names use raw value. */
  const PERCENTAGE_METRICS = new Set(['cpu_usage', 'memory_usage', 'disk_usage', 'load_average']);

  /** Metric chosen for the historical time-series panel below the cards. */
  const FOCUS_METRIC = 'cpu_usage';

  let latest = $state<LatestMetric[]>([]);
  let history = $state<MetricDataPoint[]>([]);
  let loadingLatest = $state(true);
  let loadingHistory = $state(true);
  let error = $state<string | null>(null);
  let lastUpdated = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;
  let abortInFlight: AbortController | null = null;

  async function refresh() {
    abortInFlight?.abort();
    const controller = new AbortController();
    abortInFlight = controller;

    try {
      const since = new Date(Date.now() - 24 * 60 * 60 * 1000);
      const [latestResp, historyResp] = await Promise.all([
        listLatestMetrics({ signal: controller.signal }),
        queryMetric(
          FOCUS_METRIC,
          { start: since, limit: 720 },
          { signal: controller.signal },
        ).catch((err) => {
          if (err instanceof ApiError && err.status === 404) {
            return { data: [] as MetricDataPoint[] };
          }
          throw err;
        }),
      ]);

      latest = sortLatest(latestResp.metrics);
      history = historyResp.data;
      error = null;
      lastUpdated = new Date().toISOString();
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') return;
      error = err instanceof Error ? err.message : 'Failed to load metrics';
    } finally {
      loadingLatest = false;
      loadingHistory = false;
    }
  }

  function sortLatest(metrics: LatestMetric[]): LatestMetric[] {
    // Stable, predictable order: percentage metrics first, then alphabetical.
    return [...metrics].sort((a, b) => {
      const aPct = PERCENTAGE_METRICS.has(a.name) ? 0 : 1;
      const bPct = PERCENTAGE_METRICS.has(b.name) ? 0 : 1;
      if (aPct !== bPct) return aPct - bPct;
      return a.name.localeCompare(b.name);
    });
  }

  function chartSeries() {
    return history.map((point) => ({
      timestamp: point.timestamp,
      value: point.value,
    }));
  }

  function isPercentage(metric: LatestMetric): boolean {
    return PERCENTAGE_METRICS.has(metric.name);
  }

  function unitFor(metric: LatestMetric): string | undefined {
    if (PERCENTAGE_METRICS.has(metric.name)) return undefined;
    if (metric.name.endsWith('_bytes')) return 'B';
    if (metric.name.endsWith('_seconds')) return 's';
    return undefined;
  }

  onMount(() => {
    void refresh();
    timer = setInterval(refresh, REFRESH_MS);
    return () => {
      if (timer) clearInterval(timer);
      abortInFlight?.abort();
    };
  });
</script>

<section>
  <header class="page-header">
    <div>
      <h1>Dashboard</h1>
      <p>Live resource utilization. Auto-refreshes every 30 seconds.</p>
    </div>
    <div class="meta">
      {#if lastUpdated && !error}
        <span class="updated">
          Updated <time datetime={lastUpdated}>{new Date(lastUpdated).toLocaleTimeString()}</time>
        </span>
      {/if}
      <button type="button" onclick={refresh} disabled={loadingLatest && latest.length === 0}>
        Refresh
      </button>
    </div>
  </header>

  {#if error}
    <div class="error" role="alert">
      <p>{error}</p>
      <button type="button" onclick={refresh}>Retry</button>
    </div>
  {/if}

  <h2 class="section-title">Current utilization</h2>
  {#if loadingLatest && latest.length === 0}
    <Skeleton rows={4} />
  {:else if latest.length === 0}
    <p class="empty">No metrics reported yet. Configure a source under Settings.</p>
  {:else}
    <div class="cards">
      {#each latest as metric (metric.name)}
        <MetricCard
          name={metric.name}
          value={metric.value}
          unit={unitFor(metric)}
          timestamp={metric.timestamp}
          asPercentage={isPercentage(metric)}
        />
      {/each}
    </div>
  {/if}

  <h2 class="section-title">CPU usage (24h)</h2>
  {#if loadingHistory && history.length === 0}
    <Skeleton rows={1} height="280px" />
  {:else if history.length === 0}
    <div class="empty-chart">
      <LoadingIndicator label={`No history yet for ${FOCUS_METRIC}`} />
    </div>
  {:else}
    <TimeSeriesChart title="cpu_usage" series={chartSeries()} seriesName="cpu_usage" asPercentage />
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .page-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .page-header h1 {
    margin: 0 0 0.25rem;
    font-size: 1.5rem;
  }

  .page-header p {
    margin: 0;
    color: var(--color-muted);
    font-size: 0.9375rem;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    color: var(--color-muted);
    font-size: 0.8125rem;
  }

  .meta button {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    padding: 0.4rem 0.75rem;
    font-family: inherit;
    color: var(--color-text);
    cursor: pointer;
  }

  .meta button:hover:not(:disabled) {
    border-color: var(--color-accent);
  }

  .meta button:disabled {
    opacity: 0.6;
    cursor: progress;
  }

  .section-title {
    margin: 0;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .error {
    background: color-mix(in srgb, #ef4444 10%, var(--color-surface));
    border: 1px solid #ef4444;
    border-radius: 0.5rem;
    padding: 1rem 1.25rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .error p {
    margin: 0;
  }

  .error button {
    background: #ef4444;
    color: white;
    border: none;
    padding: 0.4rem 0.9rem;
    border-radius: 0.375rem;
    cursor: pointer;
    font-family: inherit;
  }

  .empty,
  .empty-chart {
    background: var(--color-surface);
    border: 1px dashed var(--color-border);
    border-radius: 0.75rem;
    padding: 2rem;
    color: var(--color-muted);
    display: grid;
    place-items: center;
  }
</style>
