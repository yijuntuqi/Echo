<script lang="ts">
  import { onMount } from 'svelte';
  import { evolutionStore } from '$lib/stores/evolution';
  import { memoryStore } from '$lib/stores/memory';
  import { getEvolutionState, getTimeline, searchMemory } from '$lib/api/commands';
  import EvolutionTree from '$lib/components/dashboard/EvolutionTree.svelte';
  import MoodHeatmap from '$lib/components/dashboard/MoodHeatmap.svelte';
  import Timeline from '$lib/components/dashboard/Timeline.svelte';
  import MemoryGraph from '$lib/components/dashboard/MemoryGraph.svelte';

  let loading = $state(true);
  let query = $state('');

  async function runSearch(e: SubmitEvent) {
    e.preventDefault();
    const text = query.trim();
    if (!text) {
      memoryStore.setSearchResults([]);
      return;
    }
    memoryStore.setLoading(true);
    try {
      memoryStore.setSearchResults(await searchMemory(text, 12));
    } catch (err) {
      memoryStore.setError(`搜索失败：${err}`);
    } finally {
      memoryStore.setLoading(false);
    }
  }

  /** SQLite timestamps can be "YYYY-MM-DD HH:MM:SS" or RFC3339 or a bare
   * date — only the first parses reliably in every JS engine. */
  function prettyDate(raw: string): string {
    if (!raw) return '';
    const d = new Date(raw.includes(' ') && !raw.includes('T') ? raw.replace(' ', 'T') : raw);
    if (Number.isNaN(d.getTime())) return raw;
    const sameYear = d.getFullYear() === new Date().getFullYear();
    return d.toLocaleDateString('zh-CN', {
      month: 'short',
      day: 'numeric',
      ...(sameYear ? {} : { year: 'numeric' }),
    });
  }

  /** Map the 0..2 cosine distance onto a friendlier 0..100 percentage. */
  function relevance(distance: number): number {
    return Math.round(Math.max(0, Math.min(1, 1 - distance / 2)) * 100);
  }

  /** Keep results scannable: one line, the user's words first. */
  function snippet(text: string): string {
    const clean = text.replace(/\s+/g, ' ').trim();
    return clean.length > 64 ? clean.slice(0, 64) + '…' : clean;
  }

  /** Collapse whitespace for the expanded view too. */
  function fullText(text: string): string {
    return text.replace(/\s+/g, ' ').trim();
  }

  /** Click a result to unfold its full content (the list only shows one line). */
  let expandedHits = $state(new Set<string>());

  function toggleHit(key: string): void {
    const next = new Set(expandedHits);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    expandedHits = next;
  }

  onMount(async () => {
    try {
      const [evo, timeline] = await Promise.all([
        getEvolutionState(),
        getTimeline({ start: '1970-01-01', end: '2999-12-31' }, 200),
      ]);
      evolutionStore.loadFromBackend({
        stage: evo.stage,
        personality: evo.personality,
        history: evo.history,
        progress: evo.progress,
      });
      memoryStore.setTimeline(timeline);
    } catch (e) {
      console.warn('dashboard data unavailable:', e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="dashboard">
  <header class="top-bar">
    <h1>📊 记忆</h1>
    <nav><a href="#/">← 返回</a></nav>
  </header>

  {#if loading}
    <p class="loading">加载中…</p>
  {:else}
    <section class="card">
      <h2>进化树</h2>
      <EvolutionTree />
      <p class="stage-line">
        当前阶段：<strong>{evolutionStore.stageLabel}</strong>
        {#if evolutionStore.nextStageLabel}
          · 距离「{evolutionStore.nextStageLabel}」{Math.round(evolutionStore.progress * 100)}%
        {:else}
          · 已达最终形态
        {/if}
      </p>
    </section>

    <section class="card">
      <h2>心情热力图</h2>
      <MoodHeatmap />
    </section>

    <section class="card">
      <h2>🔍 语义搜索</h2>
      <form onsubmit={runSearch}>
        <input
          type="search"
          bind:value={query}
          placeholder="回想一下…（例如：上次我说的那个项目）"
          aria-label="搜索记忆"
        />
        <button type="submit" disabled={memoryStore.loading}>
          {memoryStore.loading ? '搜索中…' : '搜索'}
        </button>
      </form>
      {#if memoryStore.searchResults.length > 0}
        {#if memoryStore.searchResults.length > 1}
          <MemoryGraph hits={memoryStore.searchResults} />
        {/if}
        <ul class="hits">
          {#each memoryStore.searchResults as hit (hit.memory_id + '-' + hit.memory_type)}
            {@const key = hit.memory_id + '-' + hit.memory_type}
            <li>
              <button
                type="button"
                class="hit"
                class:expanded={expandedHits.has(key)}
                onclick={() => toggleHit(key)}
              >
                <span class="kind">{hit.memory_type === 'conversation' ? '对话' : '事件'}</span>
                <span class="when">{prettyDate(hit.created_at)}</span>
                <span class="score">相关度 {relevance(hit.distance)}%</span>
                <p class="snippet">{expandedHits.has(key) ? fullText(hit.content) : snippet(hit.content)}</p>
                <span class="unfold">{expandedHits.has(key) ? '收起 ▲' : '展开全文 ▼'}</span>
              </button>
            </li>
          {/each}
        </ul>
        <p class="hint">
          找到 {memoryStore.searchResults.length} 条相关记忆 · 点击可展开全文。
          对话是连续的一条流，按日期即可分辨每次聊天的时间。
        </p>
      {:else if query.trim() && !memoryStore.loading}
        <p class="hint">没有找到相关记忆</p>
      {/if}
      {#if memoryStore.error}
        <p class="error">{memoryStore.error}</p>
      {/if}
    </section>

    <section class="card">
      <h2>记忆时间线</h2>
      <Timeline items={memoryStore.timeline} />
    </section>
  {/if}
</div>

<style>
  .dashboard { padding: 20px; max-width: 800px; margin: 0 auto; display: flex; flex-direction: column; gap: 20px; }
  .top-bar { display: flex; justify-content: space-between; align-items: center; padding-bottom: 16px; border-bottom: 1px solid var(--color-border); }
  .top-bar h1 { font-size: 1.4rem; font-weight: 700; }
  .top-bar a { color: var(--color-text-muted); text-decoration: none; font-size: 14px; }
  .card { background: var(--color-bg-panel); border: 1px solid var(--color-border); border-radius: var(--radius-lg); padding: 24px; box-shadow: var(--shadow-soft); }
  .card h2 { font-size: 1.05rem; margin-bottom: 16px; color: var(--color-text); }
  .stage-line { text-align: center; color: var(--color-text-muted); font-size: 13px; margin-top: 12px; }
  .loading { text-align: center; color: var(--color-text-muted); padding: 40px; }

  form { display: flex; gap: 8px; margin-bottom: 16px; }
  form input {
    flex: 1; padding: 9px 12px; font: inherit; font-size: 14px;
    color: var(--color-text);
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  form input:focus { outline: none; border-color: var(--color-accent); box-shadow: 0 0 0 2px var(--color-accent-alpha); }
  form button {
    padding: 9px 16px; font: inherit; font-size: 14px; font-weight: 500;
    color: #fff; background: var(--color-accent);
    border: none; border-radius: var(--radius-sm); cursor: pointer;
  }
  form button:disabled { opacity: 0.45; cursor: not-allowed; }
  .hint { text-align: center; color: var(--color-text-muted); font-size: 12px; margin-top: 8px; }
  .error { color: #e74c3c; font-size: 13px; margin-top: 8px; }
  .hits { list-style: none; margin: 12px 0 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .hits li { list-style: none; }
  .hits .hit {
    display: flex; gap: 10px; align-items: baseline; flex-wrap: wrap;
    width: 100%; text-align: left; font: inherit;
    padding: 10px 12px; font-size: 13px;
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .hits .hit:hover, .hits .hit.expanded { border-color: var(--color-accent); }
  .hits .hit:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 1px; }
  .hits .kind { color: var(--color-accent); font-weight: 600; flex-shrink: 0; }
  .hits .when { color: var(--color-text-muted); flex-shrink: 0; }
  .hits .score { margin-left: auto; color: var(--color-text-muted); flex-shrink: 0; }
  .hits .snippet {
    flex-basis: 100%;
    margin: 0;
    color: var(--color-text);
    font-size: 12.5px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .hits .unfold {
    flex-basis: 100%;
    font-size: 11.5px;
    color: var(--color-text-muted);
  }
</style>
