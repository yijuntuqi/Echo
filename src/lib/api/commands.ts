// Tauri IPC Commands - Type-safe invoke wrappers
import { invoke } from '@tauri-apps/api/core';
import type { Conversation, MoodEntry, Event, VectorHit, EvolutionEvent, Settings, Stage } from './types';

// Window / Pet Overlay
export const showPetOverlay = () => invoke<void>('show_pet_overlay');
export const hidePetOverlay = () => invoke<void>('hide_pet_overlay');
export const setClickThrough = (enabled: boolean) => invoke<void>('set_click_through', { enabled });
export const getPetPosition = () => invoke<{ x: number; y: number } | null>('get_pet_position');
export const setPetPosition = (x: number, y: number) => invoke<void>('set_pet_position', { x, y });

// Chat
export const sendMessage = (message: string) => 
    invoke<ReadableStream<ChatChunk>>('send_message', { message });

export const getHistory = (limit: number, offset: number) => 
    invoke<Conversation[]>('get_history', { limit, offset });

export const getQuotaRemaining = () => invoke<number>('get_quota_remaining');

// Memory / Search
export const searchMemory = (query: string, topK: number) => 
    invoke<VectorHit[]>('search_memory', { query, top_k: topK });

export const addEvent = (event: Omit<Event, 'id'>) => 
    invoke<number>('add_event', { event });

export const getTimeline = (range: { start: string; end: string }, limit: number) => 
    invoke<Event[]>('get_timeline', { range, limit });

// Evolution
export const getEvolutionState = () => invoke<{
    stage: Stage;
    personality: number[];
    history: EvolutionEvent[];
    progress: number;
}>('get_evolution_state');

export const forceEvolve = (stage: Stage) => invoke<void>('force_evolve', { stage });

// Settings / Backup
export const getSettings = () => invoke<Settings>('get_settings');
export const updateSettings = (patch: Partial<Settings>) => invoke<void>('update_settings', { patch });
export const exportBackup = (password: string, destPath: string) => invoke<string>('export_backup', { password, dest_path: destPath });
export const importBackup = (password: string, srcPath: string) => invoke<void>('import_backup', { password, src_path: srcPath });

// System
export const checkUpdates = () => invoke<{ available: boolean; version: string; notes: string }>('check_updates');
export const getSystemInfo = () => invoke<{ os: string; arch: string; version: string }>('get_system_info');

// Onboarding
export const completeOnboarding = (profile: any, settings: Settings) => invoke<void>('complete_onboarding', { profile, settings });
export const getOnboardingStatus = () => invoke<boolean>('get_onboarding_status');

// ChatChunk type for streaming
export interface ChatChunk {
    id: string;
    choices: Array<{ index: number; delta: { content?: string; role?: string }; finish_reason?: string }>;
    usage?: { prompt_tokens: number; completion_tokens: number; total_tokens: number };
}