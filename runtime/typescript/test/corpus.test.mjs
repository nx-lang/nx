/**
 * The conformance corpus, from the runtime's side: every artifact in `specs/ir-conformance` is
 * prepared, linked where its module table requires, and evaluated, and each named entrypoint's
 * canonical value must equal the result the interpreter recorded. Each lifecycle is initialized
 * and its batches dispatched in order, and every rendered output, tokens included, and every
 * effect list must equal what the interpreter recorded. Every evaluation must cost exactly the
 * operations recorded for it, and stop where the recorded failures say a smaller budget stops it.
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  NxIrRuntimeError,
  dispatchComponentActions,
  evaluateFunction,
  initializeComponent,
  linkNxIrProgram,
  prepareNxIrModule,
  tryPrepareNxIrModule,
} from "../dist/src/index.js";

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
      const operations = JSON.parse(readFileSync(join(dir, "expected", "operations.json"), "utf8"));
      return { name, manifest, artifacts, strippedArtifacts, results, operations };
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

/**
 * Checks that an evaluation costs exactly `recorded` operations: under that budget it succeeds,
 * and under one less it fails on the budget.
 */
function checkCount(what, recorded, run) {
  if (!Number.isSafeInteger(recorded)) {
    throw new Error(`${what}: no operation count is recorded (found ${JSON.stringify(recorded)})`);
  }
  try {
    run({ maxOperations: recorded });
  } catch (error) {
    throw new Error(`${what} fails under its recorded count of ${recorded} operations: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (recorded === 0) {
    return;
  }
  try {
    run({ maxOperations: recorded - 1 });
  } catch (error) {
    if (error instanceof NxIrRuntimeError && error.diagnostics[0]?.limit?.name === "maxOperations") {
      return;
    }
    throw new Error(`${what} under ${recorded - 1} operations, one less than recorded, fails otherwise than on the budget: ${error instanceof Error ? error.message : String(error)}`);
  }
  throw new Error(`${what} succeeds under ${recorded - 1} operations, one less than recorded`);
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
    // A program's own emitted modules, and only those — no corpus program emits the prelude. So the
    // `ranges` program, whose image links `@nx/prelude.nx`, resolves that slot through the runtime's
    // built-in prelude on every run of this loop, in both variants: the fallback is covered here,
    // incidentally but genuinely. Adding the prelude to `modules` would remove that silently.
    const resolve = (identity) => modules.get(identity);
    for (const entrypoint of program.manifest.entrypoints) {
      const label = `${program.name} (${variant}) ${entrypoint.module}::${entrypoint.function}`;
      try {
        const module = modules.get(entrypoint.module);
        if (module === undefined) {
          throw new Error(`the corpus emits no artifact for ${entrypoint.module}`);
        }
        const linked = linkNxIrProgram(module, { resolve });
        const key = `${entrypoint.module}::${entrypoint.function}`;
        checkCount("the evaluation", program.operations.counts[key], (options) => evaluateFunction(linked, entrypoint.function, [], options));
        const actual = evaluateFunction(linked, entrypoint.function);
        const expected = program.results[key];
        if (stableJson(actual) !== stableJson(expected)) {
          throw new Error(`expected ${stableJson(expected)}, got ${stableJson(actual)}`);
        }
        console.log(`ok - ${label}`);
      } catch (error) {
        failures += 1;
        console.log(`not ok - ${label}: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
    for (const lifecycle of program.manifest.lifecycles ?? []) {
      const label = `${program.name} (${variant}) ${lifecycle.module}::${lifecycle.component} lifecycle`;
      try {
        const module = modules.get(lifecycle.module);
        if (module === undefined) {
          throw new Error(`the corpus emits no artifact for ${lifecycle.module}`);
        }
        const linked = linkNxIrProgram(module, { resolve });
        const key = `${lifecycle.module}::${lifecycle.component}`;
        const expected = program.results[key];
        const counts = program.operations.counts[key] ?? {};
        const expect = (what, actual, wanted) => {
          if (stableJson(actual) !== stableJson(wanted)) {
            throw new Error(`${what}: expected ${stableJson(wanted)}, got ${stableJson(actual)}`);
          }
        };
        checkCount("initialization", counts.initial, (options) => initializeComponent(linked, lifecycle.component, lifecycle.props ?? {}, options));
        const initialized = initializeComponent(linked, lifecycle.component, lifecycle.props ?? {});
        expect("initial rendered output", initialized.rendered, expected.initial);
        let instance = initialized.instance;
        lifecycle.batches.forEach((batch, index) => {
          checkCount(`batch ${index}`, counts.batches?.[index], (options) => dispatchComponentActions(linked, instance, batch, options));
          const dispatched = dispatchComponentActions(linked, instance, batch);
          expect(`batch ${index} rendered output`, dispatched.rendered, expected.batches[index].rendered);
          expect(`batch ${index} effects`, dispatched.effects, expected.batches[index].effects);
          instance = dispatched.instance;
        });
        console.log(`ok - ${label}`);
      } catch (error) {
        failures += 1;
        console.log(`not ok - ${label}: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
  }
}

// A budget below an evaluation's count stops it at the node the corpus records: the same
// declaration and, from the image with its debug section, the same span. Equal counts alone do not
// show that two runtimes charge in the same order; these do.
let recordedFailures = 0;
for (const program of loadCorpus()) {
  const modules = new Map([...program.artifacts].map(([identity, image]) => [identity, prepareNxIrModule(image)]));
  const resolve = (identity) => modules.get(identity);
  for (const [key, records] of Object.entries(program.operations.failures ?? {})) {
    const [module, name] = key.split("::");
    const linked = linkNxIrProgram(modules.get(module), { resolve });
    for (const record of records) {
      recordedFailures += 1;
      const label = `${program.name} ${key} under ${record.budget} operations`;
      try {
        evaluateFunction(linked, name, [], { maxOperations: record.budget });
        failures += 1;
        console.log(`not ok - ${label}: succeeds`);
      } catch (error) {
        const diagnostic = error instanceof NxIrRuntimeError ? error.diagnostics[0] : undefined;
        const actual = stableJson({ declaration: diagnostic?.declaration, limit: diagnostic?.limit, source: diagnostic?.source });
        const wanted = stableJson({ declaration: record.declaration, limit: { name: "maxOperations", value: record.budget }, source: record.source });
        if (actual === wanted) {
          console.log(`ok - ${label} stops at its recorded node`);
        } else {
          failures += 1;
          console.log(`not ok - ${label}: expected ${wanted}, got ${error instanceof NxIrRuntimeError ? actual : String(error)}`);
        }
      }
    }
  }
}
if (recordedFailures === 0) {
  failures += 1;
  console.log("not ok - the corpus records no failures");
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
