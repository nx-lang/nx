/**
 * The catalog, prepared once per page from the image the build bundled.
 *
 * <para>`virtual:nx-catalog-artifact` is the catalog compiled at build time by the Vite plugin in
 * `vite.config.ts`, through the same wasm module and the same `emitCatalogArtifact` the example
 * check uses, carried as base64. A compile therefore never emits the catalog: the worker sends the
 * visitor's module alone, and the renderer links it against this preparation.</para>
 *
 * <para>Decoded and prepared on first use rather than at import, so an image the runtime refuses
 * surfaces as a drawing failure the visitor can read rather than a blank page.</para>
 */
import type { NxPreparedModule } from "@nx-lang/ir-runtime";
import encoded from "virtual:nx-catalog-artifact";

import { prepareCatalog } from "./evaluate";

let prepared: NxPreparedModule | undefined;

/** The prepared catalog every compiled snippet links against. */
export function catalogModule(): NxPreparedModule {
  prepared ??= prepareCatalog(decodeBase64(encoded));
  return prepared;
}

/** Base64 to bytes, through the engine's decoder where it has one. */
function decodeBase64(text: string): Uint8Array {
  const fromBase64 = (Uint8Array as unknown as { fromBase64?: (text: string) => Uint8Array }).fromBase64;
  if (fromBase64 !== undefined) {
    return fromBase64(text);
  }
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}
