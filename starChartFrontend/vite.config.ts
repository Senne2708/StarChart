import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
        proxy: {
      // HTTP login routes → Rust server (avoids CORS in dev)
      "/login": "http://127.0.0.1:3000",
      "/create_login": "http://127.0.0.1:3000",

      // WebSocket → Rust server. Browsers can't set headers on a WebSocket,
      // so we pass the token as ?token=... and the proxy turns it into
      // the "Authorization: Bearer ..." header the server expects. /
      // /claude used for this part
      "/ws": {
        target: "ws://127.0.0.1:3000",
        ws: true,
        configure: (proxy) => {
          proxy.on("proxyReqWs", (proxyReq : any, req : any) => {
            const url = new URL(req.url ?? "", "http://localhost");
            const token = url.searchParams.get("token");
            if (token) proxyReq.setHeader("Authorization", `Bearer ${token}`);
          });
        },
      },
    },
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
