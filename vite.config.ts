import { defineConfig } from "vite";
import { codecovVitePlugin } from "@codecov/vite-plugin";
import vue from "@vitejs/plugin-vue";
import vuetify from "vite-plugin-vuetify";
// @ts-expect-error type error without @types/node package
import { fileURLToPath } from "node:url";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// The chatui island mirrors the upstream dashboard's `@/` import paths.
const chatuiAlias = {
  "@": fileURLToPath(new URL("./src/chatui", import.meta.url)),
};

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [
    vue(),
    vuetify({
      autoImport: true,
    }),
    codecovVitePlugin({
      enableBundleAnalysis: process.env.CODECOV_TOKEN !== undefined,
      bundleName: "astrbot-go",
      uploadToken: process.env.CODECOV_TOKEN,
    }),
  ],

  resolve: {
    alias: chatuiAlias,
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        "tray-panel": "src/tray-panel.html",
      },
    },
  },
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
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
