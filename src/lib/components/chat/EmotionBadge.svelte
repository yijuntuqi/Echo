<script lang="ts">
  import type { Emotion } from '$lib/api/types';

  let { emotion = 'neutral', weight = 0.5 }: { emotion?: Emotion; weight?: number } = $props();

  const META: Record<Emotion, { emoji: string; label: string }> = {
    happy:    { emoji: '😊', label: '开心' },
    sad:      { emoji: '😢', label: '难过' },
    anxious:  { emoji: '😰', label: '焦虑' },
    calm:     { emoji: '😌', label: '平静' },
    angry:    { emoji: '😠', label: '生气' },
    excited:  { emoji: '🤩', label: '兴奋' },
    bored:    { emoji: '😐', label: '无聊' },
    lonely:   { emoji: '🥺', label: '孤独' },
    grateful: { emoji: '🙏', label: '感激' },
    neutral:  { emoji: '😶', label: '平常' },
  };

  const meta = $derived(META[emotion] ?? META.neutral);
  const pct = $derived(Math.round(Math.min(1, Math.max(0, weight)) * 100));
</script>

<span class="badge" title={`${meta.label} · 强度 ${pct}%`}>
  <span aria-hidden="true">{meta.emoji}</span>
  <span class="label">{meta.label}</span>
  <span class="meter"><span class="fill" style:width="{pct}%"></span></span>
</span>

<style>
  .badge {
    display: inline-flex; align-items: center; gap: 5px;
    margin-top: 8px; padding: 2px 8px;
    background: var(--color-bg-app); border: 1px solid var(--color-border);
    border-radius: 999px; font-size: 11px; color: var(--color-text-muted);
  }
  .meter { width: 36px; height: 3px; background: var(--color-border); border-radius: 2px; overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--color-accent); transition: width 0.3s; }
</style>
