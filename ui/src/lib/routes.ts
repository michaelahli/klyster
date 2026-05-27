import type { Component } from 'svelte';
import Dashboard from '../routes/Dashboard.svelte';
import Resources from '../routes/Resources.svelte';
import Forecasts from '../routes/Forecasts.svelte';
import Recommendations from '../routes/Recommendations.svelte';
import Analytics from '../routes/Analytics.svelte';
import Settings from '../routes/Settings.svelte';
import NotFound from '../routes/NotFound.svelte';

/**
 * Top-level navigation entries shown in the sidebar.
 *
 * `path` matches the hash route under svelte-spa-router (no leading `#`).
 */
export interface NavItem {
  path: string;
  label: string;
  icon: string;
}

export const NAV_ITEMS: readonly NavItem[] = [
  { path: '/', label: 'Dashboard', icon: 'D' },
  { path: '/resources', label: 'Resources', icon: 'R' },
  { path: '/forecasts', label: 'Forecasts', icon: 'F' },
  { path: '/recommendations', label: 'Recommendations', icon: 'P' },
  { path: '/analytics', label: 'Analytics', icon: 'A' },
  { path: '/settings', label: 'Settings', icon: 'S' },
] as const;

/**
 * Route table consumed by svelte-spa-router's `<Router routes={...} />`.
 *
 * `*` is the catch-all 404 entry and must come last. The router accepts
 * any Svelte component constructor; we use `Component` so the table works
 * with both Svelte 5 runes-mode and legacy components.
 */
export const ROUTES: Record<string, Component> = {
  '/': Dashboard,
  '/resources': Resources,
  '/forecasts': Forecasts,
  '/recommendations': Recommendations,
  '/analytics': Analytics,
  '/settings': Settings,
  '*': NotFound,
};
