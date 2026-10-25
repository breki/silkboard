import { fixturePort } from "./e2e/ports.mjs";
import { defineConfig } from "@playwright/test";
import { mkdtempSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

process.env.SILKBOARD_E2E_DIR ??= mkdtempSync(
  join(tmpdir(), "silkboard-browser-"),
);

// Dedicated configurable port block; refuse to reuse a running app.
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  use: {
    baseURL: `https://127.0.0.1:${fixturePort(38124)}`,
    ignoreHTTPSErrors: true,
  },
  webServer: {
    command: "node e2e/server.mjs",
    url: `https://127.0.0.1:${fixturePort(38124)}/health`,
    ignoreHTTPSErrors: true,
    reuseExistingServer: false,
    gracefulShutdown: { signal: "SIGTERM", timeout: 10000 },
    timeout: 120000,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
