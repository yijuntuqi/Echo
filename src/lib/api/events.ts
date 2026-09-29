// Tauri IPC Events - Type-safe event listeners
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { ChatChunk, EvolutionEvent, MoodEntry, PetAnimationState } from './types';

type EventMap = {
    'pet:state-change': PetAnimationState;
    'chat:stream': ChatChunk;
    'evolution:triggered': EvolutionEvent;
    'notification:show': { title: string; body: string; icon?: string };
    'mood:updated': MoodEntry;
    'quota:changed': number;
    'offline:changed': boolean;
};

export async function listen<K extends keyof EventMap>(
    event: K,
    handler: (payload: EventMap[K]) => void
): Promise<UnlistenFn> {
    return listen(event, (e: { payload: EventMap[K] }) => handler(e.payload));
}

export function emit<K extends keyof EventMap>(event: K, payload: EventMap[K]) {
    // Frontend->Rust events would use window.__TAURI__.event.emit
    // Currently only listening to Rust->Frontend events
}