/**
 * Answers hover, completion, diagnostics and symbol queries for the source pane.
 *
 * The catalog is the language route's prelude, for the same reason it is the compiler's prefix:
 * an imported external component loses its defaults and inherited props (NXE12/NXE13), so the
 * visitor's text and the catalog are analyzed as one module. The handler shifts every position in
 * and every range out, so the visitor only ever sees their own lines and columns.
 */
import { createNxLanguageHandler, toNodeListener } from "@nx-lang/language-http";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { API_PREFIX } from "../base.mjs";
import { MAX_SOURCE_BYTES } from "./compile.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const catalog = readFileSync(join(appRoot, "catalog/skia.nx"), "utf8");

/** The path prefix the route answers under; the query name is the segment after it. */
export const LANGUAGE_ROUTE = `${API_PREFIX}/language/`;

/** The Fetch-shaped handler, for tests that want to ask it directly. */
export const languageHandler = createNxLanguageHandler({
  prelude: { source: catalog },
  // A query carries the source once, plus a little JSON around it.
  maxBodyBytes: MAX_SOURCE_BYTES + 4096,
  onError: (error, { query }) => console.error(`language ${query} failed:`, error),
});

/** The same handler with Node's listener signature, for `server/index.mjs`. */
export const languageListener = toNodeListener(languageHandler);
