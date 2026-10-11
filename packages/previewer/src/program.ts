import {
  NxIrRuntimeError,
  linkNxIrProgram,
  prepareNxIrModule,
  type NxPreparedModule,
  type NxPreparedProgram,
} from "@nx-lang/ir-runtime";

import type { PreviewImage } from "./types.js";

/**
 * Prepares each image and links them into a program whose entry module is `entry`: the steps every
 * host takes between `generateNxIr` and a session. Each image is resolved by the identity it is
 * listed under.
 *
 * @throws NxIrRuntimeError when an image does not prepare, two images share an identity, no image
 * is the entry, or the program does not link.
 */
export function programFromImages(images: Iterable<PreviewImage>, entry: string): NxPreparedProgram {
  const prepared = new Map<string, NxPreparedModule>();
  for (const image of images) {
    if (prepared.has(image.identity)) {
      throw programError(`Two images are listed for '${image.identity}'.`);
    }
    prepared.set(image.identity, prepareNxIrModule(image.bytes));
  }
  const module = prepared.get(entry);
  if (module === undefined) {
    throw programError(`No image is listed for the entry module '${entry}'.`);
  }
  return linkNxIrProgram(module, { resolve: (identity) => prepared.get(identity) });
}

function programError(message: string): NxIrRuntimeError {
  return new NxIrRuntimeError([{ severity: "error", code: "nx-preview-program", message }]);
}
