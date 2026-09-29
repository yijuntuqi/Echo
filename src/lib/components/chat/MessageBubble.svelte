<!-- MessageBubble - Single message with emotion badge -->
<script lang="ts">
    import StreamRenderer from './StreamRenderer.svelte';
    import EmotionBadge from './EmotionBadge.svelte';
    import type { Message } from '$lib/stores/chat';
    
    export let message: Message;
    
    const isUser = message.role === 'user';
    const alignClass = isUser ? 'user' : 'assistant';
</script>

<div class="message-bubble {alignClass}">
    <div class="bubble-content">
        <StreamRenderer content={message.streamingContent || message.content} isStreaming={message.isStreaming} />
        {#if message.emotion && !message.isStreaming}
            <EmotionBadge emotion={message.emotion} weight={message.emotionWeight} />
        {/if}
    </div>
    <span class="timestamp">{message.timestamp.toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'})}</span>
</div>

<style>
    .message-bubble { display: flex; flex-direction: column; gap: 4px; max-width: 85%; }
    .message-bubble.user { align-self: flex-end; }
    .message-bubble.assistant { align-self: flex-start; }
    .bubble-content { padding: 10px 14px; border-radius: var(--radius-md); font-size: 14px; line-height: 1.5; }
    .message-bubble.user .bubble-content { background: var(--color-accent); color: white; border-bottom-right-radius: 4px; }
    .message-bubble.assistant .bubble-content { background: var(--color-bg-input); color: var(--color-text); border-bottom-left-radius: 4px; }
    .timestamp { font-size: 10px; color: var(--color-text-muted); padding: 0 8px; }
    .message-bubble.user .timestamp { text-align: right; }
    .message-bubble.assistant .timestamp { text-align: left; }
</style>