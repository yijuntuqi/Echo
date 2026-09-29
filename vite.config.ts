import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

export default defineConfig({
  plugins: [sveltekit(), tailwindcss()],
  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
      $api: path.resolve('./src/lib/api'),
      $components: path.resolve('./src/lib/components'),
      $stores: path.resolve('./src/lib/stores'),
      $styles: path.resolve('./src/lib/styles'),
      $utils: path.resolve('./src/lib/utils'),
    },
  },
  build: {
    target: 'es2022',
    minify: 'esbuild',
    sourcemap: true,
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
});