// Mood Store - Mood entries & heatmap data
import { isBrowser as browser } from '$lib/utils/env';
import type { MoodEntry } from '$lib/api/types';

function createMoodStore() {
    let entries = $state<MoodEntry[]>([]);
    
    function upsert(entry: MoodEntry) {
        const idx = entries.findIndex(e => e.date === entry.date);
        if (idx >= 0) entries[idx] = entry; else entries = [...entries, entry].sort((a, b) => a.date.localeCompare(b.date));
    }
    
    function getRange(start: string, end: string) {
        return entries.filter(e => e.date >= start && e.date <= end);
    }
    
    if (browser) {
        const saved = localStorage.getItem('mood:entries');
        if (saved) { try { entries = JSON.parse(saved); } catch {} }
        // Module-scope store: no component owns effects here, hence the root
        // (see chat.svelte.ts for the longer note).
        $effect.root(() => {
            $effect(() => { localStorage.setItem('mood:entries', JSON.stringify(entries)); });
        });
    }
    
    return { get entries() { return entries; }, upsert, getRange };
}

export const moodStore = createMoodStore();
