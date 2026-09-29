<!-- InputArea - Chat input with send button -->
<script lang="ts">
    import { Textarea } from '@bits-ui/components/textarea';
    import { Button } from '@bits-ui/components/button';
    import { createEventDispatcher } from 'svelte';
    
    export let value: string = '';
    export let disabled: boolean = false;
    export let placeholder: string = '';
    
    const dispatch = createEventDispatcher<{ send: string; input: string }>();
    
    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            if (value.trim() && !disabled) dispatch('send', value.trim());
        }
    }
    
    function handleInput(e: Event & { target: HTMLTextAreaElement }) {
        value = e.target.value;
        dispatch('input', value);
    }
</script>

<div class="input-area">
    <Textarea.Root bind:value {disabled} {placeholder} on:keydown={handleKeydown} on:input={handleInput} 
        class="input-field" rows={1} style="resize: none; min-height: 44px; max-height: 120px;" />
    <Button.Root variant="primary" disabled={disabled || !value.trim()} on:click={() => dispatch('send', value.trim())}>
        发送
    </Button.Root>
</div>

<style>
    .input-area { display: flex; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--color-border); }
    .input-field { flex: 1; background: var(--color-bg-input); border: 1px solid var(--color-border); border-radius: var(--radius-sm); padding: 8px 12px; font-size: 14px; color: var(--color-text); font-family: inherit; }
    .input-field:focus { outline: none; border-color: var(--color-accent); box-shadow: 0 0 0 2px var(--color-accent-alpha); }
</style>