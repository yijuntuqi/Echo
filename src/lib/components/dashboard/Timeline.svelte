<script lang="ts">
  import type { TimelineItem, Emotion } from '$lib/api/types';

  let { items = [] }: { items?: TimelineItem[] } = $props();

  const MOOD: Record<Emotion, string> = {
    happy: '😊', sad: '😢', anxious: '😰', calm: '😌', angry: '😠',
    excited: '🤩', bored: '😐', lonely: '🥺', grateful: '🙏', neutral: '😶',
  };

  const TYPE_LABEL: Record<string, string> = {
    birthday: '生日', anniversary: '周年', custom: '自定义', auto_extracted: '自动提取',
  };

  function pretty(iso: string): string {
    const d = new Date(iso);
    return `${d.getMonth() + 1}月${d.getDate()}日 ${d.toLocaleDateString('zh-CN', { weekday: 'short' })}`;
  }
</script>

<div class="timeline">
  {#each items as item (item.date)}
    <article class="day">
      <header>{pretty(item.date)}</header>

      {#if item.mood}
        <div class="mood">
          <span aria-hidden="true">{MOOD[item.mood.emotion] ?? '😶'}</span>
          <span>{item.mood.emotion}</span>
          <span class="w">强度 {(item.mood.weight * 100).toFixed(0)}%</span>
        </div>
      {/if}

      {#each item.events as ev (ev.id)}
        <div class="event">
          <span class="type">{TYPE_LABEL[ev.type] ?? ev.type}</span>
          <span class="desc">{ev.description}</span>
          <span class="imp" title={`重要度 ${ev.importance}/5`}>{'★'.repeat(ev.importance)}</span>
        </div>
      {/each}
    </article>
  {:else}
    <p class="empty">暂无记录，开始对话创造回忆吧 ✨</p>
  {/each}
</div>

<style>
  .timeline { display: flex; flex-direction: column; gap: 12px; }
  .day {
    padding: 12px 14px;
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .day header { font-size: 13px; font-weight: 600; color: var(--color-accent); margin-bottom: 8px; }
  .mood {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 3px 10px; margin-bottom: 8px;
    background: var(--color-accent-alpha); border-radius: 999px;
    font-size: 12px; color: var(--color-text-muted);
  }
  .mood .w { opacity: 0.75; }
  .event { display: flex; gap: 8px; align-items: baseline; padding: 6px 0; font-size: 13px; }
  .event + .event { border-top: 1px dashed var(--color-border); }
  .type { flex-shrink: 0; min-width: 56px; font-size: 10px; text-transform: uppercase; color: var(--color-text-muted); }
  .desc { flex: 1; color: var(--color-text); }
  .imp { color: var(--color-accent); font-size: 10px; letter-spacing: -1px; }
  .empty { text-align: center; color: var(--color-text-muted); padding: 32px; font-size: 14px; }
</style>
