import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The front-end lives in ./ui; the Tauri shell in ./src-tauri.
export default defineConfig({
  root: "ui",
  plugins: [svelte({ configFile: "../svelte.config.js" })],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    // WebView2 is evergreen Chromium.
    target: "chrome120",
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // Two entry points: the always-on island stays tiny; the library window's
    // code is only loaded when that window is opened.
    rollupOptions: {
      input: { island: "ui/index.html", main: "ui/main.html" },
    },
  },
});
