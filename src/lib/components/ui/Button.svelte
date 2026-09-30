<script lang="ts">
  let {
    variant = 'primary',
    type = 'button',
    disabled = false,
    onclick,
    children,
  } = $props<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    type?: 'button' | 'submit';
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    children?: import('svelte').Snippet;
  }>();
</script>

<button {type} {disabled} {onclick} class={variant}>
  {@render children?.()}
</button>

<style>
  button {
    padding: 9px 16px;
    font: inherit;
    font-size: 14px;
    font-weight: 500;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }
  button:disabled { opacity: 0.45; cursor: not-allowed; }

  .primary { background: var(--color-accent); color: #fff; }
  .primary:hover:not(:disabled) { background: var(--color-accent-hover); }

  .secondary { background: var(--color-bg-input); color: var(--color-text); border-color: var(--color-border); }
  .secondary:hover:not(:disabled) { border-color: var(--color-accent); color: var(--color-accent); }

  .ghost { background: transparent; color: var(--color-text-muted); }
  .ghost:hover:not(:disabled) { color: var(--color-text); background: var(--color-bg-input); }

  .danger { background: #e74c3c; color: #fff; }
  .danger:hover:not(:disabled) { background: #c0392b; }
</style>
