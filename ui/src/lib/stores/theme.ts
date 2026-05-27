/**
 * Theme handling. Full implementation lands in CP-M5-011; this module exists
 * so the top bar's toggle has somewhere to wire into and so the choice is
 * persisted to localStorage from day one.
 */
import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark' | 'auto';

const STORAGE_KEY = 'klyster.theme';

function readInitial(): Theme {
  if (typeof localStorage === 'undefined') return 'auto';
  const value = localStorage.getItem(STORAGE_KEY);
  if (value === 'light' || value === 'dark' || value === 'auto') return value;
  return 'auto';
}

function applyTheme(theme: Theme) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.dataset.theme = theme;
}

export const theme = writable<Theme>(readInitial());

theme.subscribe((value) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, value);
  }
  applyTheme(value);
});

export function cycleTheme(current: Theme): Theme {
  const order: Theme[] = ['auto', 'light', 'dark'];
  const idx = order.indexOf(current);
  return order[(idx + 1) % order.length] ?? 'auto';
}
