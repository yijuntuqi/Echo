import type { TimelineItem, VectorHit } from '$lib/api/types';

function createMemoryStore() {
  let timeline = $state<TimelineItem[]>([]);
  let searchResults = $state<VectorHit[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  function setTimeline(items: TimelineItem[]): void {
    timeline = items;
  }

  function setSearchResults(hits: VectorHit[]): void {
    searchResults = hits;
  }

  function setLoading(v: boolean): void {
    loading = v;
    if (!v) error = null;
  }

  function setError(message: string | null): void {
    error = message;
    loading = false;
  }

  return {
    get timeline() { return timeline; },
    get searchResults() { return searchResults; },
    get loading() { return loading; },
    get error() { return error; },
    get isEmpty() { return timeline.length === 0; },
    setTimeline,
    setSearchResults,
    setLoading,
    setError,
  };
}

export const memoryStore = createMemoryStore();
