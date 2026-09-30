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
  <textarea
    bind:value
    {disabled}
    {placeholder}
    rows="1"
    onkeydown={onKeydown}
    oninput={() => oninput?.(value)}
    aria-label="输入消息"
  ></textarea>
  <button onclick={submit} disabled={disabled || !value.trim()}>发送</button>
</div>

<style>
  .input-area {
    display: flex; gap: 8px; align-items: flex-end;
    padding: 12px 16px; border-top: 1px solid var(--color-border);
  }
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
