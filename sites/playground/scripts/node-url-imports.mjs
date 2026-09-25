/**
 * Lets Node load modules written for Vite. An import ending in `?url` resolves to its own specifier
 * as a string, which is what Vite hands the module at build time. A relative import with no
 * extension gets the extension Vite would find. drawnui-react's `dist` does both: it imports
 * CanvasKit's wasm with `?url`, and its own modules without an extension. So a test that builds
 * DrawnUI controls under `node --test` needs this hook and nothing else. The controls themselves
 * run without a canvas until they are drawn.
 *
 * Usage: node --import ./scripts/node-url-imports.mjs --test ...
 */
import { register } from "node:module";

register("./node-url-imports.hooks.mjs", import.meta.url);
