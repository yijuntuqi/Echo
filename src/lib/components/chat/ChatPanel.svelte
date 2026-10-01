<!-- ChatPanel: streaming conversation panel. Hosted by the pet window's
     panel slot (docked, follows the pet) or by the main window (floating). -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { chatStore } from '$lib/stores/chat';
  import { petStore } from '$lib/stores/pet';
  import { systemStore } from '$lib/stores/system';
  import { sendMessage } from '$lib/api/commands';
  import { on } from '$lib/api/events';
  import MessageList from './MessageList.svelte';
  import InputArea from './InputArea.svelte';

  // `docked` flattens the panel to fill its container: the pet window docks
  // it beside the pet, where floating-corner positioning makes no sense.
  let { open = $bindable(false), docked = false } = $props();

  const cleanups: (() => void)[] = [];

  onMount(async () => {
    cleanups.push(
      await on('chat:stream', (chunk) => {
        if (chunk.delta) chatStore.appendDelta(chunk.delta);
        if (chunk.done) {
          chatStore.endStream(chunk.emotion);
          chatStore.setLastError(null);
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
    cleanups.push(
      await on('model:progress', (p) => systemStore.setModelProgress(p)),
    );
    cleanups.push(
      await on('model:done', (d) => systemStore.setModelDone(d)),
    );
  });

  onDestroy(() => cleanups.forEach((fn) => fn()));

  async function submit(text: string) {
    if (!text.trim() || chatStore.isStreaming) return;

    chatStore.addUserMessage(text);
    chatStore.startStream();
    chatStore.setOffline(false);
    chatStore.setLastError(null);
    petStore.playAnimation('talk');

    try {
      // The reply arrives asynchronously over `chat:stream`.
      await sendMessage(text);
    } catch (e) {
      console.warn('send_message failed:', e);
      chatStore.endStream();
      chatStore.setOffline(true);
      // Surface why nothing came back: offline badge alone hides the cause.
      chatStore.setLastError(typeof e === 'string' ? e : String(e));
      petStore.playAnimation('idle');
    }
  }

  // Progress of the currently downloading model file; `null` while unknown.
  const modelPercent = $derived.by(() => {
    const p = systemStore.modelProgress;
    if (!p || p.total <= 0) return null;
    return Math.min(100, Math.round((p.downloaded / p.total) * 100));
  });

  function close() { open = false; }
</script>

{#if open}
  <section class="chat-panel" class:docked aria-label="与 Echo 对话">
    {#if systemStore.modelState === 'downloading' && systemStore.modelProgress}
      <div class="model-download">
        <span class="model-file">{systemStore.modelProgress.file}</span>
        {#if modelPercent !== null}
          <div class="model-bar" role="progressbar" aria-valuenow={modelPercent} aria-valuemin={0} aria-valuemax={100}>
            <div class="model-fill" style="width: {modelPercent}%"></div>
          </div>
          <span class="model-pct">{modelPercent}%</span>
        {:else}
          <span class="model-pct">下载中…</span>
        {/if}
      </div>
    {:else if systemStore.modelState === 'failed'}
      <button
        class="model-failed"
        onclick={() => systemStore.dismissModelError()}
        title={systemStore.modelError || ''}
      >
        模型下载失败，下次对话将重试（点击关闭）
      </button>
    {/if}
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

    {#if chatStore.lastError}
      <p class="chat-error" role="alert">{chatStore.lastError}</p>
    {/if}

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
  /* Docked inside the pet window's panel slot: fill it edge to edge. */
  .chat-panel.docked {
    position: absolute;
    inset: 0;
    width: 100%;
    max-width: none;
    max-height: none;
    border-radius: 0;
    border-left: 1px solid var(--color-border);
    box-shadow: none;
  }
  .chat-error {
    padding: 6px 16px;
    color: #e74c3c;
    font-size: 12px;
    background: rgba(231, 76, 60, 0.08);
    overflow-wrap: anywhere;
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
  .model-download {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--color-border);
    font-size: 11px;
    color: var(--color-text-muted);
  }
  .model-file {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .model-bar {
    flex: 1;
    height: 3px;
    border-radius: 2px;
    background: var(--color-border);
    overflow: hidden;
  }
  .model-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.2s ease;
  }
  .model-pct { flex-shrink: 0; }
  .model-failed {
    display: block;
    width: 100%;
    padding: 6px 16px;
    border: none;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-accent-alpha);
    color: var(--color-text);
    font-size: 11px;
    text-align: left;
    cursor: pointer;
  }
  .model-failed:hover { background: var(--color-accent); color: #fff; }
</style>
