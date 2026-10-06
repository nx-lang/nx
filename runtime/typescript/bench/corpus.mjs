/**
 * Reads the corpus programs the harness runs from a checkout of `specs/ir-conformance`: each
 * program's `program.json`, its images without their debug section, which is what a host ships,
 * and its recorded operation counts, where it has them. Nothing is compiled.
 */
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { PROGRAMS } from "./core.mjs";

/** The corpus of the checkout this file is in. */
export const corpusRoot = fileURLToPath(new URL("../../../specs/ir-conformance", import.meta.url));

/** The programs of `PROGRAMS` that `root` holds, by name, in the form `buildSteps` takes. */
export function loadCorpus(root = corpusRoot) {
  const corpus = {};
  for (const name of PROGRAMS) {
    const dir = join(root, name);
    if (!existsSync(join(dir, "program.json"))) {
      continue;
    }
    const manifest = JSON.parse(readFileSync(join(dir, "program.json"), "utf8"));
    const images = {};
    for (const identity of manifest.emit) {
      images[identity] = new Uint8Array(readFileSync(join(dir, "expected", `${identity.replaceAll("/", "__")}.stripped.nxir`)));
    }
    // A revision from before the corpus recorded counts has none; the steps then have none either.
    const recorded = join(dir, "expected", "operations.json");
    const operations = existsSync(recorded) ? JSON.parse(readFileSync(recorded, "utf8")) : undefined;
    corpus[name] = { manifest, images, operations };
  }
  return corpus;
}
