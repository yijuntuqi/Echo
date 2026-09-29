<!-- Timeline - Memory timeline with events and moods -->
<script lang="ts">
    import type { TimelineItem } from '$lib/api/types';
    
    export let items: TimelineItem[] = [];
</script>

<div class="timeline">
    {#each items as item (item.date)}
        <div class="timeline-day">
            <div class="day-header">{new Date(item.date).toLocaleDateString('zh-CN', { month: 'long', day: 'numeric', weekday: 'short' })}</div>
            
            {#if item.mood}
                <div class="mood-indicator" style="--emoji: '{moodEmoji(item.mood.emotion)}';">
                    <span>{moodEmoji(item.mood.emotion)}</span>
                    <span>{item.mood.emotion}</span>
                </div>
            {/if}
            
            {#each item.events as event}
                <div class="event-item">
                    <span class="event-type">{event.type}</span>
                    <span class="event-desc">{event.description}</span>
                    <span class="event-importance">★{event.importance}</span>
                </div>
            {/if}
        </div>
    {/if}
    
    {#if items.length === 0}
        <div class="empty">暂无记录，开始对话创造回忆吧 ✨</div>
    {/if}
</div>

<script>
    function moodEmoji(emotion: string): string {
        const map: Record<string, string> = {
            happy: '😊', sad: '😢', anxious: '😰', calm: '😌',
            angry: '😠', excited: '🤩', bored: '😐', lonely: '🥺',
            grateful: '🙏', neutral: '😶',
        };
        return map[emotion] ?? '😶';
    }
</script>

<style>
    .timeline { display: flex; flex-direction: column; gap: 16px; }
    .timeline-day { padding: 12px 16px; background: var(--color-bg-input); border-radius: var(--radius-md); border: 1px solid var(--color-border); }
    .day-header { font-weight: 600; color: var(--color-accent); margin-bottom: 8px; font-size: 13px; }
    .mood-indicator { display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; background: var(--color-accent-alpha); border-radius: 100px; font-size: 12px; margin-bottom: 8px; }
    .event-item { display: flex; gap: 8px; padding: 8px 0; font-size: 13px; }
    .event-type { font-size: 10px; text-transform: uppercase; color: var(--color-text-muted); min-width: 60px; }
    .event-desc { flex: 1; color: var(--color-text); }
    .event-importance { color: var(--color-accent); font-weight: 600; }
    .empty { text-align: center; color: var(--color-text-muted); padding: 40px; }
</style>