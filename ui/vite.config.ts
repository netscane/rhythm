import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  // 设置 base path 为 /app/，与后端挂载路径一致
  base: "/app/",
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 3000,
    proxy: {
      // 代理 API 请求到后端服务器
      "/rest": {
        target: "http://127.0.0.1:5533",
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: "dist",
  },
});
