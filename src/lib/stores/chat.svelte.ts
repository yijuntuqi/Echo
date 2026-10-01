import { isBrowser as browser } from '$lib/utils/env';
import type { Emotion } from '$lib/api/types';

export interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  isStreaming: boolean;
  emotion?: Emotion;
  emotionWeight?: number;
  topics?: string[];
  timestamp: Date;
}

const HISTORY_KEY = 'chat:messages';
const MAX_CACHED = 50;

function createChatStore() {
  let messages = $state<Message[]>([]);
  let isStreaming = $state(false);
  let activeId = $state<string | null>(null);
  let inputValue = $state('');
  let quotaRemaining = $state<number | null>(null);
  let isOffline = $state(false);

  const visibleMessages = $derived(messages);
  const canSend = $derived(!isStreaming && inputValue.trim().length > 0);

  function addUserMessage(content: string): void {
    messages = [
      ...messages,
      { id: crypto.randomUUID(), role: 'user', content, isStreaming: false, timestamp: new Date() },
    ];
  }

  function startStream(): string {
    const id = crypto.randomUUID();
    activeId = id;
    isStreaming = true;
    messages = [
      ...messages,
      { id, role: 'assistant', content: '', isStreaming: true, timestamp: new Date() },
    ];
    return id;
  }

  /** Append streamed text to whichever assistant message is currently open. */
  function appendDelta(delta: string): void {
    if (!activeId || !delta) return;
    messages = messages.map((m) =>
      m.id === activeId ? { ...m, content: m.content + delta } : m,
    );
  }

  function endStream(meta?: { emotion?: Emotion; weight?: number; topics?: string[] }): void {
    if (!activeId) return;
    const id = activeId;
    messages = messages.map((m) =>
      m.id === id
        ? {
            ...m,
            isStreaming: false,
            emotion: meta?.emotion,
            emotionWeight: meta?.weight,
            topics: meta?.topics,
          }
        : m,
    );
    activeId = null;
    isStreaming = false;
  }

  function setInput(v: string): void { inputValue = v; }
  function clearInput(): void { inputValue = ''; }
  function setQuota(n: number): void { quotaRemaining = n; }
  function setOffline(v: boolean): void { isOffline = v; }
  function reset(): void { messages = []; activeId = null; isStreaming = false; }

  if (browser) {
    try {
      const raw = localStorage.getItem(HISTORY_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as (Omit<Message, 'timestamp'> & { timestamp: string })[];
        messages = parsed.slice(-MAX_CACHED).map((m) => ({ ...m, timestamp: new Date(m.timestamp) }));
      }
    } catch {
      // Corrupt cache: start clean rather than blocking the app.
    }

    // Module-scope store: there is no component to own this effect, so it
    // needs an explicit root ($effect alone throws effect_orphan here). The
    // store lives as long as the app does, so the root's stop function is
    // never called.
    $effect.root(() => {
      $effect(() => {
        const snapshot = messages.slice(-MAX_CACHED).map((m) => ({ ...m, timestamp: m.timestamp.toISOString() }));
        try {
          localStorage.setItem(HISTORY_KEY, JSON.stringify(snapshot));
        } catch {
          // Quota exceeded: drop the cache, keep the in-memory session.
        }
      });
    });
  }

  return {
    get messages() { return messages; },
    get visibleMessages() { return visibleMessages; },
    get isStreaming() { return isStreaming; },
    get inputValue() { return inputValue; },
    get canSend() { return canSend; },
    get quotaRemaining() { return quotaRemaining; },
    get isOffline() { return isOffline; },
    addUserMessage,
    startStream,
    appendDelta,
    endStream,
    setInput,
    clearInput,
    setQuota,
    setOffline,
    reset,
  };
}

export const chatStore = createChatStore();
