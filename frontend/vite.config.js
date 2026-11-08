import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { gzipSync } from "node:zlib";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [
    svelte(),
    {
      name: "precompress-public-assets",
      apply: "build",
      writeBundle({ dir }, bundle) {
        if (!dir) throw new Error("Expected a frontend output directory");
        // Read final files after Vite rewrites lazy import maps and CSS.
        // Private API data never passes through this build hook.
        for (const fileName of Object.keys(bundle)) {
          if (!/\.(js|css)$/.test(fileName)) continue;
          const file = resolve(dir, fileName);
          writeFileSync(`${file}.gz`, gzipSync(readFileSync(file)));
        }
      },
    },
  ],
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": "http://127.0.0.1:3000",
      "/health": "http://127.0.0.1:3000",
    },
  },
});
