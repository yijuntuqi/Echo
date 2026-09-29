// Chat Store - Streaming conversation state
import { browser } from '$app/environment';
import type { Conversation, ChatChunk, Emotion } from '$lib/api/types';

interface Message {
    id: string;
    role: 'user' | 'assistant';
    content: string;
    streamingContent: string;
    isStreaming: boolean;
    emotion?: Emotion;
    emotionWeight?: number;
    timestamp: Date;
    topics?: string[];
}

function createChatStore() {
    let messages = $state<Message[]>([]);
    let isStreaming = $state(false);
    let currentMessageId = $state<string | null>(null);
    let inputValue = $state('');
    let quotaRemaining = $state<number | null>(null);
    let isOffline = $state(false);
    
    const visibleMessages = $derived(
        messages.filter(m => !m.isStreaming || m.streamingContent.length > 0)
    );
    
    function addUserMessage(content: string) {
        messages = [...messages, {
            id: crypto.randomUUID(),
            role: 'user',
            content,
            streamingContent: content,
            isStreaming: false,
            timestamp: new Date(),
        }];
    }
    
    function startStream() {
        isStreaming = true;
        const id = crypto.randomUUID();
        currentMessageId = id;
        messages = [...messages, {
            id, role: 'assistant', content: '', streamingContent: '',
            isStreaming: true, timestamp: new Date(),
        }];
    }
    
    function appendChunk(chunk: ChatChunk) {
        if (!currentMessageId) return;
        messages = messages.map(m => {
            if (m.id !== currentMessageId) return m;
            const delta = chunk.choices[0]?.delta?.content ?? '';
            return { ...m, content: m.content + delta, streamingContent: m.streamingContent + delta };
        });
    }
    
    function endStream(emotion?: Emotion, weight?: number, topics?: string[]) {
        if (!currentMessageId) return;
        messages = messages.map(m => {
            if (m.id !== currentMessageId) return m;
            return { ...m, isStreaming: false, emotion, emotionWeight: weight, topics };
        });
        isStreaming = false;
        currentMessageId = null;
    }
    
    function setQuota(remaining: number) { quotaRemaining = remaining; }
    function setOffline(v: boolean) { isOffline = v; }
    function setInput(v: string) { inputValue = v; }
    function clearInput() { inputValue = ''; }
    
    if (browser) {
        const saved = localStorage.getItem('chat:messages');
        if (saved) { try { messages = JSON.parse(saved).slice(-50); } catch {} }
        $effect(() => { localStorage.setItem('chat:messages', JSON.stringify(messages.slice(-50))); });
    }
    
    return {
        get messages() { return messages; },
        get visibleMessages() { return visibleMessages; },
        get isStreaming() { return isStreaming; },
        get inputValue() { return inputValue; },
        get quotaRemaining() { return quotaRemaining; },
        get isOffline() { return isOffline; },
        addUserMessage, startStream, appendChunk, endStream,
        setQuota, setOffline, setInput, clearInput,
    };
}

export const chatStore = createChatStore();