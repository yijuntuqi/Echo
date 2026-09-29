// Settings Store - User preferences
import { browser } from '$app/environment';
import type { Settings } from '$lib/api/types';

const DEFAULT_SETTINGS: Settings = {
    nickname: '', birthday: '', theme: 'auto', notifications: true,
    auto_start: true, user_api_key: undefined, model_preference: 'auto',
    backup_password_hash: undefined,
};

function createSettingsStore() {
    let settings = $state<Settings>({ ...DEFAULT_SETTINGS });
    let loaded = $state(false);
    
    function load(s: Settings) { settings = { ...DEFAULT_SETTINGS, ...s }; loaded = true; }
    function update(patch: Partial<Settings>) { settings = { ...settings, ...patch }; }
    function reset() { settings = { ...DEFAULT_SETTINGS }; }
    
    return { get settings() { return settings; }, get loaded() { return loaded; }, load, update, reset };
}

export const settingsStore = createSettingsStore();