<script lang="ts">
  let {
    value = $bindable(''),
    disabled = false,
    placeholder = '',
    onsend,
    oninput,
  } = $props<{
    value?: string;
    disabled?: boolean;
    placeholder?: string;
    onsend?: (text: string) => void;
    oninput?: (text: string) => void;
  }>();

  // Hard cap on one message: a huge paste would otherwise be sent verbatim
  // to the API. `maxlength` enforces it on typing and pasting alike.
  const MAX_INPUT = 2000;
  // The counter only appears once the user is close to the cap.
  const COUNT_FROM = MAX_INPUT - 200;
  const counting = $derived(value.length >= COUNT_FROM);
  const atLimit = $derived(value.length >= MAX_INPUT);

  function onKeydown(e: KeyboardEvent) {
    // Enter sends; Shift+Enter inserts a newline.
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function submit() {
    const text = value.trim();
    if (!text || disabled) return;
    onsend?.(text);
    value = '';
  }
</script>

<div class="input-area">
  {#if counting}
    <div class="count" class:limit={atLimit} aria-live="polite">{value.length} / {MAX_INPUT}</div>
  {/if}
  <div class="input-row">
    <textarea
      bind:value
      {disabled}
      {placeholder}
      rows="1"
      maxlength={MAX_INPUT}
      onkeydown={onKeydown}
      oninput={() => oninput?.(value)}
      aria-label="输入消息"
    ></textarea>
    <button onclick={submit} disabled={disabled || !value.trim()}>发送</button>
  </div>
</div>

<style>
  .input-area {
    display: flex; flex-direction: column; gap: 4px;
    padding: 12px 16px; border-top: 1px solid var(--color-border);
  }
  .count {
    align-self: flex-end;
    font-size: 10px; color: var(--color-text-muted);
  }
  .count.limit { color: #e74c3c; font-weight: 600; }
  .input-row { display: flex; gap: 8px; align-items: flex-end; }
  textarea {
    flex: 1; resize: none;
    min-height: 40px; max-height: 120px;
    padding: 9px 12px; font: inherit; font-size: 14px;
    color: var(--color-text);
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  textarea:focus { outline: none; border-color: var(--color-accent); box-shadow: 0 0 0 2px var(--color-accent-alpha); }
  textarea:disabled { opacity: 0.6; }
  button {
    padding: 9px 16px; font-size: 14px; font-weight: 500;
    color: #fff; background: var(--color-accent);
    border: none; border-radius: var(--radius-sm); cursor: pointer;
  }
  button:hover:not(:disabled) { background: var(--color-accent-hover); }
  button:disabled { opacity: 0.45; cursor: not-allowed; }
</style>
