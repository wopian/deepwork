import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
const host = process.env.TAURI_DEV_HOST;
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  optimizeDeps: { include: ["pixi.js", "pixi.js/unsafe-eval"] },
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**"] },
  },
  build: { target: "es2022" },
});
