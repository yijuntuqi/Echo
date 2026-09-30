<script lang="ts">
  let {
    checked = $bindable(false),
    label,
    disabled = false,
  } = $props<{ checked?: boolean; label?: string; disabled?: boolean }>();

  const id = $props.id();
</script>

<label class="toggle" for={id}>
  {#if label}<span>{label}</span>{/if}
  <input {id} type="checkbox" role="switch" bind:checked {disabled} />
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
</label>

<style>
  .toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    cursor: pointer;
    font-size: 14px;
  }
  .toggle input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .track {
    flex-shrink: 0;
    width: 40px;
    height: 22px;
    padding: 2px;
    background: var(--color-border);
    border-radius: 999px;
    transition: background 0.2s;
  }
  .thumb {
    display: block;
    width: 18px;
    height: 18px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.2s;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
  }
  .toggle input:checked + .track { background: var(--color-accent); }
  .toggle input:checked + .track .thumb { transform: translateX(18px); }
  .toggle input:focus-visible + .track { box-shadow: 0 0 0 3px var(--color-accent-alpha); }
  .toggle input:disabled + .track { opacity: 0.5; }
</style>
