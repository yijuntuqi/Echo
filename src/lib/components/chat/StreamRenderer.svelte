<!-- StreamRenderer - Streaming markdown renderer -->
<script lang="ts">
    import { marked } from 'marked';
    import { markedHighlight } from 'marked-highlight';
    import hljs from 'highlight.js';
    import 'highlight.js/styles/github-dark.min.css';
    
    export let content: string = '';
    export let isStreaming: boolean = false;
    
    marked.use(markedHighlight({
        langPrefix: 'hljs language-',
        highlight(code, lang) {
            if (hljs.getLanguage(lang)) return hljs.highlight(code, { language: lang }).value;
            return code;
        }
    }));
    
    let renderedHtml = $derived.by(() => {
        if (isStreaming) {
            return content
                .replace(/&/g, '&')
                .replace(/</g, '<')
                .replace(/>/g, '>')
                .replace(/\n/g, '<br>');
        }
        return marked.parse(content, { async: false }) as string;
    });
</script>

<div class="message-content" innerHTML={renderedHtml} />

<style>
    .message-content :global(pre) { background: #1e1e1e; padding: 12px; border-radius: 8px; overflow: auto; margin: 8px 0; }
    .message-content :global(code) { font-family: var(--font-mono); font-size: 0.9em; }
    .message-content :global(pre code) { background: none; padding: 0; color: inherit; }
    .message-content :global(blockquote) { border-left: 3px solid var(--color-accent); padding-left: 12px; color: var(--color-text-muted); margin: 8px 0; }
    .message-content :global(a) { color: var(--color-accent); text-decoration: underline; }
    .message-content :global(ul, ol) { padding-left: 20px; margin: 8px 0; }
    .message-content :global(li) { margin: 4px 0; }
</style>