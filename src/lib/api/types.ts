// Shared TypeScript Types - Must match Rust structs exactly

export type Stage = 'egg' | 'child' | 'teen' | 'adult' | 'ultimate';

export interface Vec2 { x: number; y: number; }

export type PetAnimationState = 'idle' | 'walk' | 'sleep' | 'talk' | 'react' | 'evolve';

export type Emotion = 'happy' | 'sad' | 'anxious' | 'calm' | 'angry' | 'excited' | 'bored' | 'lonely' | 'grateful' | 'neutral';

export interface Conversation {
    id: number;
    timestamp: string;
    user_message: string;
    ai_reply: string;
    emotion?: Emotion;
    emotion_weight?: number;
    topics?: string[];
    tokens_used?: number;
    model_used?: string;
}

export interface MoodEntry {
    id: number;
    date: string;
    emotion: Emotion;
    weight: number;
    note?: string;
    source: 'auto' | 'manual';
}

export interface Event {
    id: number;
    date: string;
    description: string;
    type: 'birthday' | 'anniversary' | 'custom' | 'auto_extracted';
    importance: 1 | 2 | 3 | 4 | 5;
    tags: string[];
}

export interface VectorHit {
    memory_id: number;
    memory_type: 'conversation' | 'event';
    created_at: string;
    distance: number;
}

export interface EvolutionEvent {
    id: number;
    timestamp: string;
    from_stage: Stage;
    to_stage: Stage;
    trigger_type: 'mood_positive' | 'mood_negative' | 'special_event' | 'time_elapsed' | 'interaction_milestone' | 'memory_milestone' | 'manual';
    trigger_ref?: number;
    personality_vector: number[];
    score: number;
    metadata?: Record<string, unknown>;
}

export interface PersonalityVector {
    dims: Float32Array;
}

export interface Settings {
    nickname: string;
    birthday: string;
    theme: 'light' | 'dark' | 'auto';
    notifications: boolean;
    auto_start: boolean;
    user_api_key?: string;
    model_preference: 'daily' | 'premium' | 'auto';
    backup_password_hash?: string;
}

export interface DateRange {
    start: string;
    end: string;
}

export interface TimelineItem {
    date: string;
    events: Event[];
    mood?: MoodEntry;
}