// Typed wrappers over Tauri IPC commands (`#[tauri::command]` on the Rust side).
// Every call here maps 1:1 to a command registered in `src-tauri/src/main.rs`.
import { invoke } from '@tauri-apps/api/core';
import type {
  EvolutionStage,
  Event,
  EvolutionState,
  Profile,
  Settings,
  SystemInfo,
  TimelineItem,
  UpdateInfo,
  VectorHit,
  DateRange,
} from './types';

// --- windows / pet overlay -------------------------------------------------
export const showPetOverlay = (): Promise<void> => invoke('show_pet_overlay');
export const hidePetOverlay = (): Promise<void> => invoke('hide_pet_overlay');
/** `enabled = true` lets mouse clicks pass through the pet to whatever is beneath. */
export const setClickThrough = (enabled: boolean): Promise<void> =>
  invoke('set_click_through', { enabled });
export const getPetPosition = (): Promise<{ x: number; y: number } | null> =>
  invoke('get_pet_position');
export const setPetPosition = (x: number, y: number): Promise<void> =>
  invoke('set_pet_position', { x, y });
/** Resize + move the pet window in one atomic OS call (no blink in between). */
export const placePetWindow = (x: number, y: number, width: number, height: number): Promise<void> =>
  invoke('place_pet_window', { x, y, width, height });
/** Surface the main window and ask it to open the chat panel. */
export const openChat = (): Promise<void> => invoke('open_chat');
/**
 * Restrict where the transparent pet window accepts mouse input: physical-px
 * `[x, y, w, h]` rects (and an optional ellipse for the pet's silhouette)
 * relative to the window; input outside their union falls through. `null`
 * lifts the restriction (whole window hit-testable).
 */
export type PetShape = {
  rects?: [number, number, number, number][];
  ellipse?: [number, number, number, number];
};
export const setPetWindowShape = (shape: PetShape | null): Promise<void> =>
  invoke('set_pet_window_shape', { region: shape });

// --- chat ------------------------------------------------------------------
/** Fire-and-forget: replies stream back over the `chat:stream` event. */
export const sendMessage = (message: string): Promise<void> =>
  invoke('send_message', { message });
export const getHistory = (limit = 50, offset = 0) =>
  invoke<unknown[]>('get_history', { limit, offset });
export const getQuotaRemaining = (): Promise<number> => invoke('get_quota_remaining');

// --- memory ----------------------------------------------------------------
export const searchMemory = (query: string, topK = 8): Promise<VectorHit[]> =>
  invoke('search_memory', { query, topK });
export const addEvent = (event: Omit<Event, 'id'>): Promise<number> =>
  invoke('add_event', { event });
export const getTimeline = (range: DateRange, limit = 200): Promise<TimelineItem[]> =>
  invoke('get_timeline', { range, limit });

// --- evolution -------------------------------------------------------------
export const getEvolutionState = (): Promise<EvolutionState> => invoke('get_evolution_state');
export const forceEvolve = (stage: EvolutionStage): Promise<void> =>
  invoke('force_evolve', { stage });

// --- settings / backup -----------------------------------------------------
export const getSettings = (): Promise<Settings> => invoke('get_settings');
export const updateSettings = (patch: Partial<Settings>): Promise<void> =>
  invoke('update_settings', { patch });

// --- system ----------------------------------------------------------------
export const checkUpdates = (): Promise<UpdateInfo> => invoke('check_updates');
/** Download + install a pending update; the backend relaunches on success. */
export const installUpdate = (): Promise<void> => invoke('install_update');
export const getSystemInfo = (): Promise<SystemInfo> => invoke('get_system_info');

// --- onboarding ------------------------------------------------------------
export const completeOnboarding = (profile: Profile, settings: Settings): Promise<void> =>
  invoke('complete_onboarding', { profile, settings });
export const getOnboardingStatus = (): Promise<boolean> => invoke('get_onboarding_status');
