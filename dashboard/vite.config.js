import { defineConfig } from "vite";

export default defineConfig({
  root: "src",
  build: {
    outDir: "../dist",
    rollupOptions: {
      output: {
        entryFileNames: "main.js",
        assetFileNames: "[name].[ext]",
      },
    },
    emptyOutDir: true,
  },
});
