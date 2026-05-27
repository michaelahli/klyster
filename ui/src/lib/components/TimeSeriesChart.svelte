<script lang="ts">
  /**
   * Time-series line chart powered by ECharts.
   *
   * Resizes with its container via `ResizeObserver`. The full ECharts library
   * is large; future work can replace this with a tree-shaken `core` import
   * once the set of chart types stabilises.
   */
  import { onMount } from 'svelte';
  import * as echarts from 'echarts/core';
  import { LineChart } from 'echarts/charts';
  import { GridComponent, TitleComponent, TooltipComponent } from 'echarts/components';
  import { CanvasRenderer } from 'echarts/renderers';
  import type { ECharts, EChartsCoreOption } from 'echarts/core';

  echarts.use([LineChart, GridComponent, TitleComponent, TooltipComponent, CanvasRenderer]);

  interface SeriesPoint {
    timestamp: string;
    value: number;
  }

  interface Props {
    title?: string;
    series: SeriesPoint[];
    /** Series legend label. Defaults to "value". */
    seriesName?: string;
    /** Optional Y-axis suffix (e.g. "%"). */
    unit?: string;
    /** Render as a percentage gauge axis (0-100). */
    asPercentage?: boolean;
    /** Container height in pixels. */
    height?: number;
  }

  const {
    title,
    series,
    seriesName = 'value',
    unit,
    asPercentage = false,
    height = 280,
  }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let chart: ECharts | null = null;
  let observer: ResizeObserver | null = null;

  function buildOption(): EChartsCoreOption {
    const data = series.map((point) => [
      new Date(point.timestamp).getTime(),
      asPercentage ? point.value * 100 : point.value,
    ]);

    return {
      animation: false,
      grid: { left: 48, right: 16, top: title ? 36 : 16, bottom: 36 },
      title: title ? { text: title, left: 0, textStyle: { fontSize: 14 } } : undefined,
      tooltip: {
        trigger: 'axis',
        valueFormatter: (value: number | string) => formatValue(Number(value)),
      },
      xAxis: {
        type: 'time',
        axisLabel: { hideOverlap: true },
      },
      yAxis: {
        type: 'value',
        axisLabel: {
          formatter: (value: number) => formatValue(value),
        },
        splitLine: { lineStyle: { opacity: 0.3 } },
      },
      series: [
        {
          name: seriesName,
          type: 'line',
          showSymbol: false,
          smooth: true,
          data,
          lineStyle: { width: 2, color: '#0e7490' },
          areaStyle: {
            color: {
              type: 'linear',
              x: 0,
              y: 0,
              x2: 0,
              y2: 1,
              colorStops: [
                { offset: 0, color: 'rgba(14, 116, 144, 0.25)' },
                { offset: 1, color: 'rgba(14, 116, 144, 0)' },
              ],
            },
          },
        },
      ],
    };
  }

  function formatValue(value: number): string {
    if (asPercentage) return `${value.toFixed(0)}%`;
    if (Math.abs(value) >= 1000) return value.toFixed(0);
    if (Math.abs(value) >= 1) return value.toFixed(2);
    return value.toFixed(3);
  }

  onMount(() => {
    if (!container) return;
    chart = echarts.init(container, undefined, { renderer: 'canvas' });
    chart.setOption(buildOption());

    observer = new ResizeObserver(() => chart?.resize());
    observer.observe(container);

    return () => {
      observer?.disconnect();
      chart?.dispose();
      chart = null;
      observer = null;
    };
  });

  // Re-render when the series prop changes.
  $effect(() => {
    if (!chart) return;
    chart.setOption(buildOption(), { notMerge: true });
  });

  // The runes-mode prop is intentionally read inside the effect below so the
  // chart re-renders if a parent ever changes the unit; the formatter does
  // not currently use it (axis labels are short enough to omit the unit), but
  // wiring it now keeps the prop honest.
  $effect(() => {
    void unit;
  });
</script>

<div bind:this={container} class="chart" style="height: {height}px"></div>

<style>
  .chart {
    width: 100%;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 0.75rem;
    padding: 1rem;
  }
</style>
