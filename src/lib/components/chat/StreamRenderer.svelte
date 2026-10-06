<script lang="ts">
  import { marked } from 'marked';
  import { markedHighlight } from 'marked-highlight';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/github-dark.css';

  let { content = '', isStreaming = false } = $props<{ content: string; isStreaming?: boolean }>();

  marked.use(
    markedHighlight({
      langPrefix: 'hljs language-',
      highlight(code, lang) {
        return hljs.getLanguage(lang) ? hljs.highlight(code, { language: lang }).value : code;
      },
    }),
  );

  // While streaming, escape and keep line breaks only: full Markdown parsing of a
  // half-received string flickers (unclosed code fences, half-written tables).
  const html = $derived.by(() => {
    if (isStreaming) {
      return content
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/\n/g, '<br>');
    }
    return marked.parse(content, { async: false }) as string;
  });
</script>

<div class="md">
  {@html html}
  {#if isStreaming}<span class="cursor" aria-hidden="true"></span>{/if}
</div>

<style>
  .md { font-size: 14px; }
  /* Blinking caret at the tail of a streaming reply: "more is coming". */
  .cursor {
    display: inline-block;
    width: 2px;
    height: 1em;
    margin-left: 2px;
    vertical-align: -0.15em;
    background: currentColor;
    animation: cursor-blink 0.9s steps(2, start) infinite;
  }
  @keyframes cursor-blink { 50% { opacity: 0; } }
  @media (prefers-reduced-motion: reduce) {
    .cursor { animation: none; }
  }
  .md :global(p) { margin: 0 0 8px; }
  .md :global(p:last-child) { margin-bottom: 0; }
  .md :global(pre) {
    background: #1e1e1e; color: #e6edf3;
    padding: 10px 12px; border-radius: var(--radius-sm);
    overflow-x: auto; margin: 8px 0; font-size: 12.5px;
  }
  .md :global(code) { font-family: var(--font-mono); font-size: 0.9em; }
  .md :global(:not(pre) > code) {
    background: rgba(127, 127, 127, 0.18); padding: 1px 5px; border-radius: 4px;
  }
  .md :global(blockquote) {
    border-left: 3px solid var(--color-accent);
    padding-left: 10px; margin: 8px 0; opacity: 0.85;
  }
  .md :global(ul), .md :global(ol) { padding-left: 20px; margin: 8px 0; }
  .md :global(li) { margin: 3px 0; }
</style>
