/**
 * Writes `test/fixtures/values.json`: each source in `test/fixtures/sources.json` evaluated by
 * the IR runtime and formatted by `@nx-lang/sdk-wasm`. The element's tests read these, so they exercise the text
 * and nodes the compiler really produces rather than a copy of its layout, and a test fails when
 * the compiler's output drifts from what is committed.
 *
 * These are canonical-host values formatted at the declared root type; unlike the Rust CLI,
 * JavaScript has no rich result view for recovering constant-case identity at object sites.
 *
 * Usage: node --no-warnings scripts/update-fixtures.mjs
 */
import { readFileSync, writeFileSync } from "node:fs";
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";
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
          const entry = artifact.generateNxIr()[0];
          const modules = new Map(artifact.generateNxIr({ modules: [] }).map(image =>
            [image.identity, prepareNxIrModule(image.bytes)]));
          const program = linkNxIrProgram(modules.get(entry.identity), {
            resolve: identity => modules.get(identity)
          });
          const value = evaluateFunction(program, "root");
          const formatted = artifact.formatValue(value, {
            type: artifact.functionSchema({ name: "root" }).resultType
          });
          return [name, formatted];
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
