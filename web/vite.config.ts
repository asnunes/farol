import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(__dirname, "./src") } },
  build: { outDir: "dist", emptyOutDir: true },
  server: {
    port: 5173,
    // In dev the Rust server proxies here, and the page talks back to it for
    // data — so API calls have to reach the backend, not Vite.
    proxy: { "/api": "http://127.0.0.1:4600" },
  },
  test: { environment: "jsdom", globals: true },
});
