<!-- MessageList - Virtual list of messages -->
<script lang="ts">
    import MessageBubble from './MessageBubble.svelte';
    import type { Message } from '$lib/stores/chat';
    
    export let messages: Message[] = [];
</script>

<div class="message-list">
    {#each messages as msg (msg.id)}
        <MessageBubble message={msg} />
    {/each}
    {#if messages.length === 0}
        <div class="empty-state">说点什么开始对话吧 💬</div>
    {/if}
</div>

<style>
    .message-list { flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 12px; }
    .empty-state { text-align: center; color: var(--color-text-muted); padding: 40px; font-size: 14px; }
</style>