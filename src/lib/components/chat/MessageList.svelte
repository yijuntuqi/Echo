<script lang="ts">
  import { tick } from 'svelte';
  import MessageBubble from './MessageBubble.svelte';
  import type { Message } from '$lib/stores/chat';

  let { messages = [], streaming = false }: { messages?: Message[]; streaming?: boolean } = $props();

  let listEl = $state<HTMLDivElement>();
  // Auto-scroll only while the user is (still) near the bottom: scrolling up
  // to read history must not be hijacked by incoming stream chunks.
  let pinned = $state(true);

  async function follow() {
    if (!listEl || !pinned) return;
    await tick();
    if (listEl) listEl.scrollTop = listEl.scrollHeight;
  }

  function onScroll() {
    if (!listEl) return;
    pinned = listEl.scrollHeight - listEl.scrollTop - listEl.clientHeight < 32;
  }

  // New content (a message appended, a stream delta, the typing state)
  // re-anchors to the bottom while `pinned`. Reading the last content length
  // inside the effect is what makes every delta trigger it.
  $effect(() => {
    const last = messages[messages.length - 1];
    void messages.length;
    void last?.content.length;
    void streaming;
    void follow();
  });

  // Waiting for the first token of a fresh reply: an empty streaming
  // assistant bubble. Once text arrives the bubble itself takes over.
  const typing = $derived.by(() => {
    const last = messages[messages.length - 1];
    return streaming && (!last || (last.role === 'assistant' && last.content === ''));
  });
</script>

<div class="message-list" bind:this={listEl} onscroll={onScroll}>
  {#each messages as msg (msg.id)}
    <MessageBubble message={msg} />
  {:else}
    <p class="empty">说点什么开始对话吧 💬</p>
  {/each}
  {#if typing}
    <div class="typing" aria-label="Echo 正在输入">
      <span></span><span></span><span></span>
    </div>
  {/if}
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

  .typing {
    align-self: flex-start;
    display: flex;
    gap: 5px;
    padding: 13px 16px;
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    border-bottom-left-radius: 4px;
  }
  .typing span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-muted);
    animation: typing-bounce 1.2s ease-in-out infinite;
  }
  .typing span:nth-child(2) { animation-delay: 0.15s; }
  .typing span:nth-child(3) { animation-delay: 0.3s; }
  @keyframes typing-bounce {
    0%, 60%, 100% { transform: translateY(0); opacity: 0.45; }
    30% { transform: translateY(-4px); opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .typing span { animation: none; opacity: 0.6; }
  }
</style>
