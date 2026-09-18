import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** The extensions Vite adds to a bare relative import, tried in its order. */
const EXTENSIONS = [".ts", ".tsx", "/index.ts", "/index.tsx"];

export async function resolve(specifier, context, next) {
  if (specifier.endsWith("?url")) {
    return { url: `nx-url:${specifier}`, shortCircuit: true };
  }
  // A relative import without an extension: Vite resolves it, Node does not.
  if ((specifier.startsWith("./") || specifier.startsWith("../")) && !/\.[a-z]+$/i.test(specifier) && context.parentURL !== undefined) {
    const base = new URL(specifier, context.parentURL);
    for (const extension of EXTENSIONS) {
      const candidate = new URL(`${base.href}${extension}`);
      if (existsSync(fileURLToPath(candidate))) {
        return next(candidate.href, context);
      }
    }
  }
  return next(specifier, context);
}

export async function load(url, context, next) {
  if (url.startsWith("nx-url:")) {
    return { format: "module", source: `export default ${JSON.stringify(url.slice("nx-url:".length))};`, shortCircuit: true };
  }
  return next(url, context);
}
