// Typed wrappers over Tauri's event system.
//
// `listen` is re-exported under a distinct name so call sites read as domain
// events (`onPetStateChange`) rather than generic IPC plumbing.
import { listen as tauriListen, emit as tauriEmit, type UnlistenFn } from '@tauri-apps/api/event';
import type { Emotion } from './types';

export type EmotionEvent = {
  emotion: Emotion;
  weight: number;
  topics?: string[];
};

export type StreamChunk = {
  delta: string;
  done: boolean;
  emotion?: EmotionEvent;
};

export type StatusEvent = {
  offline?: boolean;
  quota_remaining?: number;
};

export type EvolutionEvent = {
  from_stage: string;
  to_stage: string;
  trigger: string;
};

interface EventMap {
  'chat:stream': StreamChunk;
  'chat:status': StatusEvent;
  'chat:open': void;
  'pet:state-change': string;
  'evolution:triggered': EvolutionEvent;
  'mood:updated': { date: string; emotion: Emotion; weight: number };
  'notification:show': { title: string; body: string };
}

type Handler<K extends keyof EventMap> = (payload: EventMap[K]) => void;

/** Subscribe to a Rust-emitted event. Returns an unsubscribe function. */
export function on<K extends keyof EventMap>(
  event: K,
  handler: Handler<K>,
): Promise<UnlistenFn> {
  return tauriListen(event, (e) => handler(e.payload as EventMap[K]));
}

/** Fire an event towards the backend (rare: most traffic is Rust -> frontend). */
export function send<K extends keyof EventMap>(event: K, payload: EventMap[K]): Promise<void> {
  return tauriEmit(event, payload);
}
