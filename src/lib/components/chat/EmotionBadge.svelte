<!-- EmotionBadge - Visual emotion indicator -->
<script lang="ts">
    import type { Emotion } from '$lib/api/types';
    
    export let emotion: Emotion = 'neutral';
    export let weight: number = 0.5;
    
    const emojiMap: Record<Emotion, string> = {
        happy: '😊', sad: '😢', anxious: '😰', calm: '😌',
        angry: '😠', excited: '🤩', bored: '😐', lonely: '🥺',
        grateful: '🙏', neutral: '😶',
    };
    
    const labelMap: Record<Emotion, string> = {
        happy: '开心', sad: '难过', anxious: '焦虑', calm: '平静',
        angry: '生气', excited: '兴奋', bored: '无聊', lonely: '孤独',
        grateful: '感激', neutral: '平常',
    };
</script>

<span class="emotion-badge" title={labelMap[emotion]} style="--emoji: '{emojiMap[emotion]}';">
    <span class="emoji">{emojiMap[emotion]}</span>
    <span class="label">{labelMap[emotion]}</span>
    <span class="weight-bar"><span class="weight-fill" style="width: {weight * 100}%"></span></span>
</span>

<style>
    .emotion-badge { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; color: var(--color-text-muted); margin-top: 8px; padding: 2px 8px; background: var(--color-bg-input); border-radius: 100px; }
    .emoji { font-size: 12px; }
    .label { text-transform: capitalize; }
    .weight-bar { width: 40px; height: 3px; background: var(--color-border); border-radius: 2px; overflow: hidden; }
    .weight-fill { display: block; height: 100%; background: var(--color-accent); border-radius: 2px; transition: width 0.3s; }
</style>