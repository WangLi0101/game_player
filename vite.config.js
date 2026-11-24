import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  build: { rollupOptions: { input: { main: 'index.html', achievementOverlay: 'achievement-overlay.html', trayMenu: 'tray-menu.html' } } },
  server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } }
});
