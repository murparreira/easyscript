import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import { viteSingleFile } from "vite-plugin-singlefile";

// Two builds of one frontend: the desktop app's (`dist`), and with `--mode page` a single
// HTML file (`dist-page`) that `easyscript build` fills with a script's data.
export default defineConfig(({ mode }) => ({
  plugins: mode === "page" ? [svelte(), viteSingleFile()] : [svelte()],
  base: "./",
  // Tauri expects a fixed port and its own output kept on screen.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    outDir: mode === "page" ? "dist-page" : "dist",
  },
}));
