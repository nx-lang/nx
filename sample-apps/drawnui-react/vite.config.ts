import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

/**
 * The vendored DrawnUI source is a Vite project: it imports the CanvasKit wasm binary with `?url`
 * and loads its fonts from `publicDir`. Both settings below mirror `samples/vite.shared.ts`
 * upstream so the vendored tree runs unmodified.
 */
export default defineConfig({
  plugins: [react()],
  build: { target: "esnext" },
  server: {
    // The NX TextMate grammar is imported from the repository rather than copied, so dev needs to
    // be allowed to read above the app root.
    fs: { allow: [".", "../.."] },
    proxy: {
      "/api": "http://localhost:5174",
    },
  },
});
