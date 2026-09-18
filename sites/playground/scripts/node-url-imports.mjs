/**
 * Lets Node load modules written for Vite: an import ending in `?url` resolves to its own specifier
 * as a string, which is what Vite hands the module at build time. The vendored DrawnUI runtime
 * imports CanvasKit's wasm that way, so a test that builds DrawnUI controls under `node --test`
 * needs this hook and nothing else — the controls themselves run without a canvas until drawn.
 *
 * Usage: node --import ./scripts/node-url-imports.mjs --test ...
 */
import { register } from "node:module";

register("./node-url-imports.hooks.mjs", import.meta.url);
