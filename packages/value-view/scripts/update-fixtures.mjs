/**
 * Writes `test/fixtures/values.json`: each source in `test/fixtures/sources.json` evaluated by
 * `evaluateNx()` in `@nx-lang/sdk-wasm`. The element's tests read these, so they exercise the text
 * and nodes the compiler really produces rather than a copy of its layout, and a test fails when
 * the compiler's output drifts from what is committed.
 *
 * Usage: node --no-warnings scripts/update-fixtures.mjs
 */
import { readFileSync, writeFileSync } from "node:fs";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";

const sourcesUrl = new URL("../test/fixtures/sources.json", import.meta.url);
const valuesUrl = new URL("../test/fixtures/values.json", import.meta.url);

/** Evaluates every source, by name. */
export async function evaluateFixtures() {
  const sources = JSON.parse(readFileSync(sourcesUrl, "utf8"));
  const host = createNxHost(await loadNxModule());
  try {
    return Object.fromEntries(
      Object.entries(sources).map(([name, source]) => {
        const artifact = host.buildProgramArtifact(source);
        try {
          return [name, artifact.evaluateNx()];
        } finally {
          artifact.dispose();
        }
      })
    );
  } finally {
    host.dispose();
  }
}

if (process.argv[1] === new URL(import.meta.url).pathname) {
  writeFileSync(valuesUrl, `${JSON.stringify(await evaluateFixtures(), null, 2)}\n`);
  console.log(`Wrote ${valuesUrl.pathname}`);
}
