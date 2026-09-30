<!-- ChatPanel: streaming conversation panel (lives in the main window) -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { chatStore } from '$lib/stores/chat';
  import { petStore } from '$lib/stores/pet';
  import { sendMessage, setClickThrough } from '$lib/api/commands';
  import { on } from '$lib/api/events';
  import MessageList from './MessageList.svelte';
  import InputArea from './InputArea.svelte';

  let { open = $bindable(false) } = $props();

  const cleanups: (() => void)[] = [];

  onMount(async () => {
    cleanups.push(
      await on('chat:stream', (chunk) => {
        if (chunk.delta) chatStore.appendDelta(chunk.delta);
        if (chunk.done) {
          chatStore.endStream(chunk.emotion);
          petStore.playAnimation('idle');
        }
      }),
    );
    cleanups.push(
      await on('chat:status', (s) => {
        if (typeof s.offline === 'boolean') chatStore.setOffline(s.offline);
        if (typeof s.quota_remaining === 'number') chatStore.setQuota(s.quota_remaining);
      }),
    );
  });

  onDestroy(() => cleanups.forEach((fn) => fn()));

  // While the chat is open the pet must not swallow clicks meant for the panel.
  $effect(() => {
    setClickThrough(!open).catch(() => {});
  });

  async function submit(text: string) {
    if (!text.trim() || chatStore.isStreaming) return;

    chatStore.addUserMessage(text);
    chatStore.startStream();
    chatStore.setOffline(false);
    petStore.playAnimation('talk');

    try {
      // The reply arrives asynchronously over `chat:stream`.
      await sendMessage(text);
    } catch (e) {
      console.warn('send_message failed:', e);
      chatStore.endStream();
      chatStore.setOffline(true);
      petStore.playAnimation('idle');
    }
  }

  function close() { open = false; }
</script>

{#if open}
  <section class="chat-panel" aria-label="与 Echo 对话">
    <header class="chat-header">
      <h3>Echo</h3>
      <div class="status">
        {#if chatStore.isOffline}
          <span class="offline">离线模式</span>
        {:else if chatStore.quotaRemaining !== null}
          <span>今日剩余 {chatStore.quotaRemaining} 次</span>
        {:else}
          <span class="online">在线</span>
        {/if}
      </div>
      <button class="close-btn" onclick={close} aria-label="关闭对话">×</button>
    </header>

    <MessageList messages={chatStore.messages} />

    <InputArea
      value={chatStore.inputValue}
      oninput={(text) => chatStore.setInput(text)}
      onsend={submit}
      disabled={chatStore.isStreaming}
      placeholder={chatStore.isStreaming ? 'Echo 正在思考…' : '和 Echo 聊聊吧…'}
    />
  </section>
{/if}

<style>
  .chat-panel {
    position: fixed;
    right: 20px;
    bottom: 20px;
    width: min(380px, calc(100vw - 40px));
    max-height: min(600px, calc(100vh - 40px));
    background: var(--color-bg-panel);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-strong);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    z-index: 9998;
  }
  .chat-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-border);
  }
  .chat-header h3 { font-size: 0.95rem; font-weight: 600; }
  .status { margin-left: auto; font-size: 11px; color: var(--color-text-muted); }
  .online::before { content: '●'; color: #2ecc71; margin-right: 4px; }
  .offline { color: #e67e22; }
  .close-btn {
    background: none;
    border: none;
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    color: var(--color-text-muted);
    padding: 0 4px;
  }
  .close-btn:hover { color: var(--color-text); }
</style>
