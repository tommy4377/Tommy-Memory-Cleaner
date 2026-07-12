import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig(({ command }) => ({
  plugins: [svelte()],
  esbuild: {
    // Strip console noise from production bundles; console.error is kept so
    // real failures remain diagnosable in the field. No-op for the dev server.
    pure: command === 'build' ? ['console.log', 'console.warn', 'console.debug', 'console.info'] : [],
    drop: command === 'build' ? ['debugger'] : []
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**']
    }
  },
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  build: { 
    target: 'es2020', 
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: './index.html',
        tray: './tray.html',
        setup: './setup.html'
      }
    }
  },
  publicDir: 'public'
}));