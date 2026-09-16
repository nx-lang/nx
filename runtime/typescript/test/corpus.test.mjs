/**
 * The conformance corpus, from the runtime's side: every artifact in `specs/ir-conformance` is
 * prepared, linked where its module table requires, and evaluated, and each named entrypoint's
 * canonical value must equal the result the interpreter recorded.
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { NxIrRuntimeError, evaluateFunction, linkNxIrProgram, prepareNxIrModule, tryPrepareNxIrModule } from "../dist/src/index.js";

const testRoot = fileURLToPath(new URL(".", import.meta.url));
const corpusRoot = resolve(testRoot, "../../../specs/ir-conformance");

function loadCorpus() {
  return readdirSync(corpusRoot)
    .filter((name) => statSync(join(corpusRoot, name)).isDirectory())
    .sort()
    .map((name) => {
      const dir = join(corpusRoot, name);
      const manifest = JSON.parse(readFileSync(join(dir, "program.json"), "utf8"));
      const artifacts = new Map();
      const strippedArtifacts = new Map();
      for (const identity of manifest.emit) {
        const file = identity.replaceAll("/", "__");
        artifacts.set(identity, new Uint8Array(readFileSync(join(dir, "expected", `${file}.nxir`))));
        strippedArtifacts.set(identity, new Uint8Array(readFileSync(join(dir, "expected", `${file}.stripped.nxir`))));
      }
      const results = JSON.parse(readFileSync(join(dir, "expected", "results.json"), "utf8"));
      return { name, manifest, artifacts, strippedArtifacts, results };
    });
}

function stableJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  if (value !== null && typeof value === "object") {
    const entries = Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value);
}

let failures = 0;

for (const program of loadCorpus()) {
  for (const [variant, sources] of [
    ["with debug", program.artifacts],
    ["stripped", program.strippedArtifacts],
  ]) {
    const modules = new Map();
    for (const [identity, image] of sources) {
      modules.set(identity, prepareNxIrModule(image));
    }
    const resolve = (identity) => modules.get(identity);
    for (const entrypoint of program.manifest.entrypoints) {
      const label = `${program.name} (${variant}) ${entrypoint.module}::${entrypoint.function}`;
      try {
        const module = modules.get(entrypoint.module);
        if (module === undefined) {
          throw new Error(`the corpus emits no artifact for ${entrypoint.module}`);
        }
        const linked = linkNxIrProgram(module, { resolve });
        const actual = evaluateFunction(linked, entrypoint.function);
        const expected = program.results[`${entrypoint.module}::${entrypoint.function}`];
        if (stableJson(actual) !== stableJson(expected)) {
          throw new Error(`expected ${stableJson(expected)}, got ${stableJson(actual)}`);
        }
        console.log(`ok - ${label}`);
      } catch (error) {
        failures += 1;
        console.log(`not ok - ${label}: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
  }
}

// Damage: every corpus image cut at every four-byte boundary, and every cell of the smallest one
// with a function entrypoint overwritten, must be refused with a diagnostic or read as a valid
// image, never thrown. A damaged image that prepared is then linked and each of its entrypoints
// evaluated, and anything that throws must be the runtime's own error, since a stranger's share
// is evaluated, not just opened.
const corpus = loadCorpus();
const images = corpus.flatMap((program) =>
  [...program.artifacts, ...program.strippedArtifacts].map(([identity, image]) => ({ program, identity, image })),
);
let refusedTruncations = 0;
for (const { image } of images) {
  for (let end = 0; end < image.byteLength; end += 4) {
    const result = tryPrepareNxIrModule(image.subarray(0, end));
    if (result.ok) {
      failures += 1;
      console.log(`not ok - an image cut at ${end} of ${image.byteLength} bytes prepared`);
    } else {
      refusedTruncations += 1;
    }
  }
}
console.log(`ok - ${refusedTruncations} truncated images refused`);
const subject = images
  .filter(({ program, identity }) => program.manifest.entrypoints.some((entrypoint) => entrypoint.module === identity))
  .reduce((best, candidate) => (candidate.image.byteLength < best.image.byteLength ? candidate : best));
const subjectEntrypoints = subject.program.manifest.entrypoints
  .filter((entrypoint) => entrypoint.module === subject.identity)
  .map((entrypoint) => entrypoint.function);
const intactModules = new Map([...subject.program.strippedArtifacts].map(([identity, image]) => [identity, prepareNxIrModule(image)]));
let opened = 0;
let refused = 0;
let evaluated = 0;
let evaluationsFailed = 0;
for (let offset = 0; offset < subject.image.byteLength; offset += 4) {
  for (const value of [0, 1, 0xffffffff, 0x7ffffff0]) {
    const damaged = subject.image.slice();
    new DataView(damaged.buffer).setUint32(offset, value, true);
    let result;
    try {
      result = tryPrepareNxIrModule(damaged);
    } catch (error) {
      failures += 1;
      console.log(`not ok - cell ${offset / 4} = ${value} threw: ${error instanceof Error ? error.message : String(error)}`);
      continue;
    }
    if (!result.ok) {
      refused += 1;
      continue;
    }
    opened += 1;
    for (const name of subjectEntrypoints) {
      try {
        const linked = linkNxIrProgram(result.value, { resolve: (identity) => intactModules.get(identity) });
        evaluateFunction(linked, name);
        evaluated += 1;
      } catch (error) {
        if (error instanceof NxIrRuntimeError) {
          evaluationsFailed += 1;
        } else {
          failures += 1;
          console.log(`not ok - cell ${offset / 4} = ${value}, ${name}() threw a ${error?.constructor?.name}: ${error instanceof Error ? error.message : String(error)}`);
        }
      }
    }
  }
}
console.log(`ok - ${subject.program.name}/${subject.identity}: ${opened} damaged images opened, ${refused} refused, none threw`);
console.log(`ok - ${evaluated} entrypoint evaluations of damaged images succeeded, ${evaluationsFailed} failed with a runtime error, none threw anything else`);

if (failures > 0) {
  throw new Error(`${failures} corpus entrypoint(s) failed`);
}
