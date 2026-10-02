// Shared types mirrored from the Rust side (`src-tauri/src/**`).
// Keep field names in snake_case: they arrive verbatim over IPC.

export type Stage = 'egg' | 'child' | 'teen' | 'adult' | 'ultimate';

/** Alias used by the evolution commands to avoid clashing with the DOM `Stage`. */
export type EvolutionStage = Stage;

export type PetAnimationState = 'idle' | 'walk' | 'sleep' | 'talk' | 'react' | 'evolve';

export type Emotion =
  | 'happy'
  | 'sad'
  | 'anxious'
  | 'calm'
  | 'angry'
  | 'excited'
  | 'bored'
  | 'lonely'
  | 'grateful'
  | 'neutral';

export type EventType = 'birthday' | 'anniversary' | 'custom' | 'auto_extracted';

export interface Profile {
  nickname: string;
  birthday: string; // YYYY-MM-DD
  install_date?: string;
}

export interface Settings {
  nickname: string;
  birthday: string;
  theme: 'light' | 'dark' | 'auto';
  notifications: boolean;
  auto_start: boolean;
  /** Empty string = use the built-in shared key. Required (not optional):
   * these feed a `$bindable` with a fallback in TextField, and Svelte 5
   * throws `props_invalid_value` when `undefined` is bound to one. */
  user_api_key: string;
  /** OpenAI-compatible endpoint paired with a user key; empty = default. */
  user_base_url: string;
  model_preference: 'daily' | 'premium' | 'auto';
}

export interface MoodEntry {
  id?: number;
  date: string; // YYYY-MM-DD
  emotion: Emotion;
  weight: number; // 0..2
  note?: string;
  source?: 'auto' | 'manual';
}

export interface Event {
  id?: number;
  date: string;
  description: string;
  type: EventType;
  importance: number; // 1..5
  tags?: string[];
}

export interface TimelineItem {
  date: string;
  events: Event[];
  mood?: MoodEntry;
}

export interface DateRange {
  start: string;
  end: string;
}

export interface VectorHit {
  memory_id: number;
  memory_type: 'conversation' | 'event';
  /** RFC3339 (conversations) or YYYY-MM-DD (events). */
  created_at: string;
  /** Short text excerpt of the memory, for the results list. */
  content: string;
  distance: number; // cosine distance, 0 (identical) .. 2 (opposite)
}

export interface EvolutionRecord {
  id?: number;
  timestamp: string;
  from_stage: Stage;
  to_stage: Stage;
  trigger_type: string;
  personality_vector: number[];
  score: number;
}

export interface EvolutionState {
  stage: Stage;
  personality: number[]; // length 32, each in [-1, 1]
  history: EvolutionRecord[];
  progress: number; // 0..1 toward the next stage
}

export interface UpdateInfo {
  available: boolean;
  version: string;
  notes: string;
}

export interface SystemInfo {
  os: string;
  arch: string;
  version: string;
}
