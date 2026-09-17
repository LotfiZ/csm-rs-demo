import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  build: { outDir: 'dist', emptyOutDir: true },
  server: {
    // `npm run dev` proxies the API to the Rust server (`cargo run`).
    proxy: { '/api': 'http://127.0.0.1:7878' },
  },
});