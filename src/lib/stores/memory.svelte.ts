// Memory Store - Timeline, vector search, events
import type { Event, VectorHit, TimelineItem, DateRange } from '$lib/api/types';

function createMemoryStore() {
    let timeline = $state<TimelineItem[]>([]);
    let searchResults = $state<VectorHit[]>([]);
    let loading = $state(false);
    
    function setTimeline(items: TimelineItem[]) { timeline = items; }
    function setSearchResults(hits: VectorHit[]) { searchResults = hits; }
    function setLoading(v: boolean) { loading = v; }
    
    return { get timeline() { return timeline; }, get searchResults() { return searchResults; }, get loading() { return loading; }, setTimeline, setSearchResults, setLoading };
}

export const memoryStore = createMemoryStore();