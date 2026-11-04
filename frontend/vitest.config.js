import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  test: {
    // Validation overlaps a Rust lane; bound CI frontend memory/CPU workers.
    maxWorkers: process.env.CI ? 2 : undefined,
    environment: "jsdom",
    setupFiles: ["./test/location.js"],
    include: ["src/**/*.test.js"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.svelte", "src/**/*.ts"],
      // main.ts is the DOM mount adapter; the browser suite exercises it.
      exclude: ["src/main.ts", "src/vite-env.d.ts"],
      thresholds: {
        lines: 90,
        functions: 90,
        statements: 90,
        branches: 85,
        perFile: true,
      },
    },
  },
});
