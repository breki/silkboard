import { defineConfig } from "@playwright/test";

// Fixed dedicated loopback ports; refuse to reuse a developer's running app.
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  use: { baseURL: "http://127.0.0.1:38124" },
  webServer: {
    command: "cargo run --locked -p silkboard-web",
    cwd: "..",
    env: { SILKBOARD_LISTEN: "127.0.0.1:38124" },
    url: "http://127.0.0.1:38124/health",
    reuseExistingServer: false,
    timeout: 120000,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
