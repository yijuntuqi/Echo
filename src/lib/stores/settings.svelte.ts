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
    settings = { ...DEFAULTS, ...next };
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
