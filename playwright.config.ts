import { defineConfig, devices } from "@playwright/test";

// End-to-end layout tests. They run the real Svelte UI against the built-in
// demo backends (no Tauri needed), so they work on any OS and in CI.
export default defineConfig({
  testDir: "tests/e2e",
  fullyParallel: true,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:4173",
    ...devices["Desktop Chrome"],
    deviceScaleFactor: 1,
  },
  webServer: {
    command: "pnpm build && pnpm exec vite preview --port 4173 --strictPort",
    url: "http://localhost:4173/main.html",
    // Always build fresh: a reused server can serve a stale bundle.
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
