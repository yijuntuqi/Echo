import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'node:path';

// Tauri expects a fixed port and does not want Vite to obscure Rust errors.
export default defineConfig({
  plugins: [svelte(), tailwindcss()],

  // Two HTML entry points: the main window and the transparent pet window.
  build: {
    target: 'es2022',
    minify: 'esbuild',
    sourcemap: true,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        pet: resolve(__dirname, 'pet.html'),
      },
    },
  },

  resolve: {
    // Rune-ful modules use the `.svelte.ts` / `.svelte.js` compound extension,
    // which must be tried before the bare `.ts` / `.js`.
    extensions: [
      '.svelte.ts',
      '.svelte.js',
      '.mjs',
      '.js',
      '.mts',
      '.ts',
      '.jsx',
      '.tsx',
      '.json',
      '.svelte',
    ],
    alias: {
      $lib: resolve(__dirname, 'src/lib'),
      $api: resolve(__dirname, 'src/lib/api'),
      $styles: resolve(__dirname, 'src/lib/styles'),
      $utils: resolve(__dirname, 'src/lib/utils'),
    },
  },

  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },

  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
});
