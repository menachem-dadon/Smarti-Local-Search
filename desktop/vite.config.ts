import { defineConfig } from 'vite';
import type {} from 'vitest/config';
import react from '@vitejs/plugin-react';
import {readFileSync} from 'node:fs';
const version=JSON.parse(readFileSync(new URL('./package.json',import.meta.url),'utf8')).version;
// Direct Tabler imports keep the graph small. Avoid a Rollup property-path
// analysis regression observed with React 19 (CPU profile in build evidence).
export default defineConfig({ define:{__APP_VERSION__:JSON.stringify(version)},plugins: [react()], build:{rollupOptions:{treeshake:false}},clearScreen: false, server: { port: 1423, strictPort: true }, test: { include:['src/**/*.test.{ts,tsx}'],environment: 'jsdom', setupFiles: './src/test-setup.ts',maxWorkers:2 } });
