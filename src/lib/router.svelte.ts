// Lightweight hash-based router (replaces SvelteKit's file-based routing)
// Routes are declared as flat components and swapped by the App shell.

import { isBrowser } from '$lib/utils/env';

export type RouteName = '/' | '/dashboard' | '/settings' | '/onboarding';

const ROUTES: RouteName[] = ['/', '/dashboard', '/settings', '/onboarding'];

function normalize(hash: string): RouteName {
  const path = hash.replace(/^#/, '') || '/';
  return (ROUTES as string[]).includes(path) ? (path as RouteName) : '/';
}

// Reactive current route (Svelte 5 runes)
let current = $state<RouteName>('/');

if (isBrowser) {
  current = normalize(window.location.hash);
  window.addEventListener('hashchange', () => {
    current = normalize(window.location.hash);
  });
}

/** Navigate to a route (updates hash, which updates `current`). */
export function goto(path: RouteName): void {
  if (!isBrowser) return;
  if (normalize(window.location.hash) === path) return;
  window.location.hash = path;
}

export const router = {
  get current(): RouteName { return current; },
  goto,
  back(): void { if (isBrowser) window.history.back(); },
};
