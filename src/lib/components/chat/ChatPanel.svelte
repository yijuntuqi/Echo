<!-- ChatPanel - Streaming conversation panel -->
<script lang="ts">
    import { chatStore } from '$lib/stores/chat';
    import { petStore } from '$lib/stores/pet';
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '$lib/api/events';
    import MessageList from './MessageList.svelte';
    import InputArea from './InputArea.svelte';
    import { onMount, onDestroy } from 'svelte';
    
    let isOpen = $state(false);
    let unlisten: (() => void) | null = null;
    
    onMount(() => {
        unlisten = listen('chat:stream', (chunk: any) => {
            chatStore.appendChunk(chunk);
        });
    });
    
    onDestroy(() => unlisten?.());
    
    async function sendMessage() {
        const text = chatStore.inputValue.trim();
        if (!text || chatStore.isStreaming) return;
        
        chatStore.addUserMessage(text);
        chatStore.clearInput();
        chatStore.startStream();
        petStore.playAnimation('talk');
        
        try {
            const stream = await invoke<ReadableStream<any>>('send_message', { message: text });
            for await (const chunk of stream) {
                chatStore.appendChunk(chunk);
                if (chunk.choices[0]?.finish_reason === 'stop') {
                    const content = chunk.choices[0]?.delta?.content ?? '';
                    try {
                        const parsed = JSON.parse(content);
                        chatStore.endStream(parsed.emotion, parsed.emotion_weight, parsed.topics);
                    } catch {
                        chatStore.endStream();
                    }
                    petStore.playAnimation('idle');
                }
            }
        } catch (e) {
            chatStore.endStream();
            petStore.playAnimation('idle');
            chatStore.setOffline(true);
        }
    }
    
    function toggle() {
        isOpen = !isOpen;
        if (isOpen) petStore.setClickThrough(false);
    }
    
    function close() {
        isOpen = false;
        petStore.setClickThrough(true);
    }
</script>

{#if isOpen}
    <div class="chat-panel" animate:slide={{ x: -300, duration: 200 }}>
        <div class="chat-header">
            <div class="pet-mini" innerHTML={$petStore.svgPaths.idle} />
            <h3>Echo</h3>
            <div class="status">
                {#if chatStore.isOffline} 📴 离线模式
                {:else if chatStore.quotaRemaining !== null} 💬 剩余 {chatStore.quotaRemaining}
                {:else} 🟢 在线 {/if}
            </div>
            <button class="close-btn" on:click={close}>×</button>
        </div>
        
        <MessageList messages={chatStore.visibleMessages} />
        
        <InputArea 
            value={chatStore.inputValue}
            on:input={(e) => chatStore.setInput(e.target.value)}
            on:send={sendMessage}
            disabled={chatStore.isStreaming}
            placeholder={chatStore.isStreaming ? 'Echo 正在思考...' : '和 Echo 聊聊吧...'}
        />
    </div>
    
    <div class="chat-backdrop" on:click={close} />
{/if}

<style>
    .chat-panel {
        position: fixed;
        right: 20px;
        bottom: 200px;
        width: 380px;
        max-height: 600px;
        background: var(--color-bg-panel);
        border-radius: var(--radius-lg);
        box-shadow: var(--shadow-strong);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        z-index: 9998;
        border: 1px solid var(--color-border);
    }
    .chat-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.3); backdrop-filter: blur(2px); z-index: 9997; }
    .chat-header { padding: 12px 16px; display: flex; align-items: center; gap: 8px; border-bottom: 1px solid var(--color-border); }
    .pet-mini { width: 32px; height: 32px; }
    .status { margin-left: auto; font-size: 11px; color: var(--color-text-muted); }
    .close-btn { background: none; border: none; font-size: 20px; cursor: pointer; color: var(--color-text-muted); }
</style>