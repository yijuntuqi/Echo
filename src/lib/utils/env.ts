// Environment detection (replaces SvelteKit's `$app/environment`)
export const isBrowser: boolean = typeof window !== 'undefined';
export const isTauri: boolean =
  isBrowser && ('__TAURI__' in window || '__TAURI_INTERNALS__' in window);

// Storage-safe helpers
export function safeLocalStorage(): Storage | null {
  if (!isBrowser) return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}
