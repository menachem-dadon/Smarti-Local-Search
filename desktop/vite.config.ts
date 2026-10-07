import { defineConfig } from 'vite';
import type {} from 'vitest/config';
import react from '@vitejs/plugin-react';
// Direct Tabler imports keep the graph small. Avoid a Rollup property-path
// analysis regression observed with React 19 (CPU profile in build evidence).
export default defineConfig({ plugins: [react()], build:{rollupOptions:{treeshake:false}},clearScreen: false, server: { port: 1423, strictPort: true }, test: { include:['src/**/*.test.{ts,tsx}'],environment: 'jsdom', setupFiles: './src/test-setup.ts',maxWorkers:2 } });
