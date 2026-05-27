/**
 * Typed wrappers around the `/api/v1/metrics*` endpoints.
 *
 * The backend DTOs live in `svcs/web/src/dto/metrics.rs`. Keep these in
 * sync when fields are added/removed there.
 */

import { getJson, type FetchOptions } from './client';

export interface MetricNamesResponse {
  names: string[];
  count: number;
}

export interface LatestMetric {
  name: string;
  value: number;
  /** RFC3339 timestamp. */
  timestamp: string;
  source_id: number;
}

export interface LatestMetricsResponse {
  metrics: LatestMetric[];
  count: number;
}

export interface MetricDataPoint {
  id: number;
  source_id: number;
  name: string;
  value: number;
  timestamp: string;
}

export interface MetricQueryInfo {
  start: string;
  end: string;
  source_id: number | null;
  limit: number;
}

export interface MetricQueryResponse {
  name: string;
  data: MetricDataPoint[];
  count: number;
  query: MetricQueryInfo;
}

export interface MetricQueryParams {
  start: Date | string;
  end?: Date | string;
  sourceId?: number;
  limit?: number;
}

export function listMetricNames(options?: FetchOptions): Promise<MetricNamesResponse> {
  return getJson<MetricNamesResponse>('/api/v1/metrics', options);
}

export function listLatestMetrics(options?: FetchOptions): Promise<LatestMetricsResponse> {
  return getJson<LatestMetricsResponse>('/api/v1/metrics/latest', options);
}

export function queryMetric(
  name: string,
  params: MetricQueryParams,
  options?: FetchOptions,
): Promise<MetricQueryResponse> {
  const query = new URLSearchParams();
  query.set('start', toIso(params.start));
  if (params.end) query.set('end', toIso(params.end));
  if (params.sourceId !== undefined) query.set('source_id', String(params.sourceId));
  if (params.limit !== undefined) query.set('limit', String(params.limit));

  const path = `/api/v1/metrics/${encodeURIComponent(name)}?${query.toString()}`;
  return getJson<MetricQueryResponse>(path, options);
}

function toIso(value: Date | string): string {
  return value instanceof Date ? value.toISOString() : value;
}
