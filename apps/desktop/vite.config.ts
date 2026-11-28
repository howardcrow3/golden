import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  // Tests may read the repo docs (terms.test.ts checks docs/legal/terms.md); the dev server may not.
  server: { port: 1420, strictPort: true, ...(process.env.VITEST ? { fs: { allow: ["../.."] } } : {}) },
  test: { environment: "jsdom", include: ["src/**/*.test.ts"] },
  resolve: process.env.VITEST ? { conditions: ["browser"] } : undefined,
});
