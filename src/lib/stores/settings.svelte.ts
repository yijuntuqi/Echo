import type { Settings } from '$lib/api/types';

const DEFAULTS: Settings = {
  nickname: '',
  birthday: '',
  theme: 'auto',
  notifications: true,
  auto_start: true,
  user_api_key: undefined,
  user_base_url: undefined,
  model_preference: 'auto',
};

function createSettingsStore() {
  let settings = $state<Settings>({ ...DEFAULTS });
  let loaded = $state(false);

  function load(next: Partial<Settings>): void {
    // Backend may hand back null/undefined optional strings; input bindings
    // need real strings, so coalesce them here once and for all.
    settings = {
      ...DEFAULTS,
      ...next,
      user_api_key: next.user_api_key ?? '',
      user_base_url: next.user_base_url ?? '',
    };
    loaded = true;
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = settings.theme;
    }
  }

  function update(patch: Partial<Settings>): void {
    settings = { ...settings, ...patch };
  }

  function reset(): void {
    settings = { ...DEFAULTS };
    loaded = false;
  }

  return {
    get settings() { return settings; },
    get loaded() { return loaded; },
    load,
    update,
    reset,
  };
}

export const settingsStore = createSettingsStore();
