import metadata from "./examples.json";
import type { Capability, Coverage, Example } from "./types";
import accessibility from "./nx/accessibility.nx?raw";
import cells from "./nx/cells.nx?raw";
import images from "./nx/images.nx?raw";
import layouts from "./nx/layouts.nx?raw";
import looks from "./nx/looks.nx?raw";
import rootMenu from "./nx/root.nx?raw";
import shapes from "./nx/shapes.nx?raw";
import snapping from "./nx/snapping.nx?raw";
import svg from "./nx/svg.nx?raw";
import text from "./nx/text.nx?raw";
import transforms from "./nx/transforms.nx?raw";
import unevenCells from "./nx/uneven-cells.nx?raw";

/** Every example's NX, by the id its metadata gives it. */
const SOURCES: Record<string, string> = {
  accessibility,
  cells,
  images,
  layouts,
  looks,
  root: rootMenu,
  shapes,
  snapping,
  svg,
  text,
  transforms,
  "uneven-cells": unevenCells,
};

/**
 * The example set, in the order the DrawnUI demo site lists it, with the names it gives them.
 *
 * Every entry is NX compiled through the app's own pipeline — never the vendored TSX page rendered
 * natively. A gallery of originals would always look right while proving nothing; compiling the
 * ports means a gap in the catalog or the renderer shows up as a broken example, which is the
 * feedback this app exists to produce. For the same reason an entry with no NX behind it is a
 * build-time failure rather than a card that opens onto nothing.
 */
export const EXAMPLES: readonly Example[] = metadata.map((entry) => {
  const source = SOURCES[entry.id];
  if (source === undefined) {
    throw new Error(`Example '${entry.id}' has metadata but no NX source.`);
  }
  return {
    id: entry.id,
    name: entry.name,
    blurb: entry.blurb,
    coverage: entry.coverage as Coverage,
    capabilities: entry.capabilities as Capability[],
    demonstrates: "demonstrates" in entry ? (entry.demonstrates as string) : undefined,
    source,
  };
});

export function exampleById(id: string): Example | undefined {
  return EXAMPLES.find((example) => example.id === id);
}

export { coverageNote } from "./types";
export type { Capability, Coverage, Example } from "./types";
