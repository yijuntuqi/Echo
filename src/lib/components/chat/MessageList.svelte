<script lang="ts">
  import MessageBubble from './MessageBubble.svelte';

  let { messages = [] } = $props();
</script>

<div class="message-list">
  {#each messages as msg (msg.id)}
    <MessageBubble message={msg} />
  {:else}
    <p class="empty">说点什么开始对话吧 💬</p>
  {/each}
</div>

<style>
  .message-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .empty { margin: auto; color: var(--color-text-muted); font-size: 14px; }
</style>
