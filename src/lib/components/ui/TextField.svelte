<script lang="ts">
  let {
    value = $bindable(''),
    type = 'text',
    placeholder = '',
    disabled = false,
    label,
  } = $props<{
    value?: string;
    type?: 'text' | 'password' | 'date';
    placeholder?: string;
    disabled?: boolean;
    label?: string;
  }>();

  const id = $props.id();
</script>

<label class="field" for={id}>
  {#if label}<span>{label}</span>{/if}
  <input {id} {type} bind:value {placeholder} {disabled} />
</label>

<style>
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 13px; color: var(--color-text-muted); }
  input {
    width: 100%;
    padding: 9px 12px;
    font: inherit;
    font-size: 14px;
    color: var(--color-text);
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  input:focus {
    outline: none;
    border-color: var(--color-accent);
    box-shadow: 0 0 0 2px var(--color-accent-alpha);
  }
  input:disabled { opacity: 0.6; }
  /* Make the native date picker icon visible on dark backgrounds. */
  input[type='date']::-webkit-calendar-picker-indicator { filter: invert(var(--picker-invert, 0)); cursor: pointer; }
  :global(:root[data-theme='dark']) input[type='date']::-webkit-calendar-picker-indicator { --picker-invert: 1; }
</style>
