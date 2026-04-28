import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri expects a fixed dev port and disables HMR overlays in production builds.
export default defineConfig(async () => ({
  plugins: [svelte()],

  // Tauri uses Chromium on Windows/Linux and WebKit on macOS. Targeting
  // ES2021 keeps build output compatible with both without unnecessary
  // polyfills.
  build: {
    target: ["es2021", "chrome105", "safari15"],
    minify: "esbuild",
    sourcemap: false,
  },

  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
    // Don't shell out to the OS; Tauri owns the window.
    open: false,
  },

  // Tauri injects env vars under TAURI_ at build time.
  envPrefix: ["VITE_", "TAURI_"],
}));
