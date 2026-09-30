<script lang="ts">
  import StreamRenderer from './StreamRenderer.svelte';
  import EmotionBadge from './EmotionBadge.svelte';
  import type { Message } from '$lib/stores/chat';

  let { message }: { message: Message } = $props();

  const isUser = $derived(message.role === 'user');
</script>

<div class="bubble {isUser ? 'user' : 'assistant'}">
  <div class="content">
    <StreamRenderer content={message.content} isStreaming={message.isStreaming} />
    {#if message.emotion && !message.isStreaming}
      <EmotionBadge emotion={message.emotion} weight={message.emotionWeight ?? 0.5} />
    {/if}
  </div>
  <time class="ts" datetime={message.timestamp.toISOString()}>
    {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
  </time>
</div>

<style>
  .bubble { display: flex; flex-direction: column; gap: 4px; max-width: 85%; }
  .bubble.user { align-self: flex-end; align-items: flex-end; }
  .bubble.assistant { align-self: flex-start; align-items: flex-start; }
  .content {
    padding: 10px 14px;
    border-radius: var(--radius-md);
    font-size: 14px;
    line-height: 1.55;
    overflow-wrap: anywhere;
  }
  .user .content { background: var(--color-accent); color: #fff; border-bottom-right-radius: 4px; }
  .assistant .content { background: var(--color-bg-input); color: var(--color-text); border-bottom-left-radius: 4px; }
  .ts { font-size: 10px; color: var(--color-text-muted); padding: 0 4px; }
</style>
