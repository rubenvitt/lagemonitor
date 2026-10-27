// Übernommen aus Suchlage d621418 (27.10.2026): frontend/vite.config.ts
/// <reference types="vitest/config" />
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';

const BACKEND = 'http://127.0.0.1:8080';

// rust-embed braucht frontend/dist zur Compile-Zeit; emptyOutDir löscht die .gitkeep, der Hook
// schreibt sie leer neu (Muster aus Lifeline).
const gitkeepBewahren = (): Plugin => {
  let pfad = '';
  return {
    name: 'lagemonitor-dist-gitkeep',
    apply: 'build',
    configResolved(config) {
      pfad = resolve(config.root, config.build.outDir, '.gitkeep');
    },
    closeBundle() {
      writeFileSync(pfad, '');
    },
  };
};

// Node 26 bricht den Vite-Dev-Server gelegentlich bei sehr großen Strings ab („Lazy deopt after a
// fast API call“, Lifeline LFH-659). Vorgebündelte Pakete daher ohne angehängte Sourcemap. Anders als
// Lifeline immer aktiv, weil `pnpm dev` der Hauptweg ist, die UI anzusehen.
const depsOhneSourcemap = (): Plugin => ({
  name: 'lagemonitor-deps-ohne-sourcemap',
  apply: 'serve',
  enforce: 'pre',
  transform: {
    filter: { id: /\/node_modules\/\.vite\/deps\// },
    handler: (code) => ({ code: `${code}\n// Sourcemap weggelassen (Node-26-Abbruch)\n`, map: { mappings: '' } }),
  },
});

export default defineConfig({
  plugins: [react(), gitkeepBewahren(), depsOhneSourcemap()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': { target: BACKEND, changeOrigin: false },
    },
  },
  build: { sourcemap: false, chunkSizeWarningLimit: 2500 },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./vitest.setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
  },
});
