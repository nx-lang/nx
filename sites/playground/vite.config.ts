import react from "@vitejs/plugin-react";
import { type Plugin, defineConfig } from "vite";
import { BASE_HREF } from "./base.mjs";

/**
 * Puts the site's `<base href>` into the shell from the same constant Vite's `base` comes from.
 *
 * Editor addresses are nested (`/playground/shapes`), and DrawnUI loads the images the examples
 * name by relative path; without a `<base>` they would resolve against the route. Injecting it
 * here rather than writing it in `index.html` keeps `base.mjs` the only place the prefix is spelled.
 */
function siteBase(): Plugin {
  return {
    name: "site-base",
    transformIndexHtml: () => [{ tag: "base", attrs: { href: BASE_HREF }, injectTo: "head-prepend" }],
  };
}

/**
 * The vendored DrawnUI source is a Vite project: it imports the CanvasKit wasm binary with `?url`
 * and loads its fonts from `publicDir`. The `build` and `fs` settings below mirror
 * `samples/vite.shared.ts` upstream so the vendored tree runs unmodified.
 *
 * `base` is the site's prefix: every asset URL Vite emits and `import.meta.env.BASE_URL` in the
 * client carry it, and the dev server serves the app at the same address production does, so a
 * path written against the origin's root breaks locally before it ships.
 *
 * There is no API proxy. The compiler and the language service are a WebAssembly module the client
 * loads into a worker, so `pnpm run dev` alone is the whole development setup.
 */
export default defineConfig({
  base: BASE_HREF,
  plugins: [react(), siteBase()],
  build: { target: "esnext" },
  server: {
    // The shared packages are workspace links into the repository, so dev needs to be allowed to
    // read above the app root.
    fs: { allow: [".", "../.."] },
  },
});
