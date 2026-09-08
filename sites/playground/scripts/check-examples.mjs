/**
 * One check over the whole example set.
 *
 * Every example must compile with no diagnostics and evaluate, every entry must have NX behind it,
 * and every non-complete example must name at least one capability from the shared vocabulary —
 * the vocabulary being fixed is what lets the gallery be read as a coverage report rather than a
 * pile of disclaimers.
 *
 * Usage: npm run check-examples
 */
import { evaluateFunction, prepareNxIrProgram } from "@nx-lang/ir-runtime";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compile } from "../server/compile.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CAPABILITIES = new Set(["event-handlers", "animation", "component-state", "list-virtualization"]);
const COVERAGE = new Set(["complete", "static", "reduced"]);

const examples = JSON.parse(readFileSync(join(appRoot, "src/examples/examples.json"), "utf8"));
const failures = [];

for (const example of examples) {
  const label = example.id;
  const path = join(appRoot, "src/examples/nx", example.file);

  if (!existsSync(path)) {
    failures.push(`${label}: no NX source at ${example.file}`);
    continue;
  }
  if (!COVERAGE.has(example.coverage)) {
    failures.push(`${label}: unknown coverage state '${example.coverage}'`);
  }
  for (const capability of example.capabilities) {
    if (!CAPABILITIES.has(capability)) {
      failures.push(`${label}: '${capability}' is not in the capability vocabulary`);
    }
  }
  if (example.coverage !== "complete" && example.capabilities.length === 0) {
    failures.push(`${label}: ${example.coverage} examples must name at least one capability`);
  }
  if (example.coverage === "complete" && example.capabilities.length > 0) {
    failures.push(`${label}: a complete example names no missing capability`);
  }
  if (example.coverage === "reduced" && !example.demonstrates) {
    failures.push(`${label}: a reduced example says what the original demonstrates`);
  }

  const result = compile(readFileSync(path, "utf8"));
  if (result.ir === null) {
    for (const diagnostic of result.diagnostics) {
      failures.push(`${label}: ${diagnostic.origin} ${diagnostic.message}`);
    }
    continue;
  }
  for (const diagnostic of result.diagnostics) {
    failures.push(`${label}: unexpected diagnostic — ${diagnostic.message}`);
  }
  try {
    evaluateFunction(prepareNxIrProgram(result.ir), "root");
  } catch (error) {
    failures.push(`${label}: evaluation failed — ${error.message}`);
    continue;
  }
  const state = example.coverage === "complete" ? "complete" : `${example.coverage} (${example.capabilities.join(", ")})`;
  console.log(`ok - ${example.name}: compiles, evaluates, ${state}`);
}

if (failures.length > 0) {
  console.error(`\n${failures.length} problem(s):`);
  for (const failure of failures) {
    console.error(`  ${failure}`);
  }
  process.exit(1);
}
console.log(`\n${examples.length} examples checked.`);
