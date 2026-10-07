/**
 * The harness's core, tested against the committed build: every step runs, a step the corpus
 * records an operation count for reports that count, two builds of the runtime run the same steps
 * apart, and the core's source loads no module and names nothing of Node, which is what lets a
 * host run it in its own engine. It also holds the generated catalog to its generator, and checks
 * the rule the comparison names a step by.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import * as runtime from "../dist/src/index.js";
import { buildSteps, stepName, storedStates } from "./core.mjs";
import { corpusRoot, loadCorpus } from "./corpus.mjs";
import { verdict } from "./verdict.mjs";

let failures = 0;
function check(label, run) {
  try {
    run();
    console.log(`ok - ${label}`);
  } catch (error) {
    failures += 1;
    console.log(`not ok - ${label}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

const corpus = loadCorpus();
const steps = buildSteps(runtime, corpus);

const phases = ["prepare", "link", "evaluate", "initialize", "resume", "evaluate with state", "dispatch", "input", "tool call"];
check("the steps cover every phase but loading, which only a driver can time", () => {
  const missing = phases.filter((phase) => !steps.some((step) => step.phase === phase));
  if (missing.length > 0) {
    throw new Error(`no step of ${missing.join(", ")}`);
  }
});

let recorded = 0;
for (const step of steps) {
  check(`${stepName(step)} (${step.variant}) runs`, () => {
    step.ready();
    const usage = step.run();
    if (step.variant === "unlimited") {
      if (usage.operations !== undefined || usage.inputSize !== undefined) {
        throw new Error(`a call with no limits reported ${JSON.stringify(usage)}`);
      }
      return;
    }
    if (!Number.isSafeInteger(usage.operations) || !Number.isSafeInteger(usage.inputSize)) {
      throw new Error(`the usage report gave ${JSON.stringify(usage)}`);
    }
    if (step.recorded !== undefined) {
      recorded += 1;
      if (usage.operations !== step.recorded) {
        throw new Error(`used ${usage.operations} operations, and the corpus records ${step.recorded}`);
      }
    }
  });
}
check("the corpus records a count for evaluation, initialization and dispatch", () => {
  if (recorded < 4) {
    throw new Error(`only ${recorded} steps had a recorded count`);
  }
});

// A stored state passed in gives the same steps the same counts as one the core dispatched for.
check("a stored state the driver supplies is used as it is", () => {
  const stored = storedStates(runtime, corpus);
  const supplied = buildSteps(runtime, corpus, JSON.parse(JSON.stringify(stored)));
  for (const [index, step] of supplied.entries()) {
    if (step.variant !== "limited" || (step.phase !== "resume" && step.phase !== "evaluate with state")) {
      continue;
    }
    const [own, other] = [steps[index].run(), step.run()];
    if (own.operations !== other.operations || own.inputSize !== other.inputSize) {
      throw new Error(`${stepName(step)}: ${JSON.stringify(own)} with its own state, ${JSON.stringify(other)} with the one supplied`);
    }
  }
});

// A dispatch step's first call alone is what its cold time is.
check("a dispatch step's first call runs alone", () => {
  const first = steps.filter((step) => step.first !== undefined);
  if (first.length === 0 || first.some((step) => step.phase !== "dispatch")) {
    throw new Error(`the steps with a first call are ${first.map(stepName).join(", ") || "none"}`);
  }
  for (const step of first) {
    step.first();
  }
});

// Two builds of the runtime get the same steps, and running one build's does nothing to the
// other's. The second build here is the same files loaded again, which is a module of its own
// with state of its own.
const second = await import("../dist/src/index.js?second");
check("two builds of the runtime run the same steps, and neither affects the other", () => {
  if (second.prepareNxIrModule === runtime.prepareNxIrModule) {
    throw new Error("the second import is the first module again");
  }
  const others = buildSteps(second, corpus);
  const describe = (list) => JSON.stringify(list.map((step) => [stepName(step), step.variant, step.divisor, step.recorded]));
  if (describe(others) !== describe(steps)) {
    throw new Error("the two builds were given different steps");
  }
  const limited = (list) => list.filter((step) => step.variant === "limited");
  const before = limited(steps).map((step) => JSON.stringify(step.run()));
  const theirs = limited(others).map((step) => JSON.stringify(step.run()));
  const after = limited(steps).map((step) => JSON.stringify(step.run()));
  if (JSON.stringify(theirs) !== JSON.stringify(before) || JSON.stringify(after) !== JSON.stringify(before)) {
    throw new Error("a step's usage differs between the two builds, or changed once the other build ran");
  }
});

// A program the corpus lacks has no steps, and the others keep theirs.
check("a corpus without a program has the other programs' steps", () => {
  const { "question-flow": _, ...rest } = corpus;
  const fewer = buildSteps(runtime, rest);
  if (fewer.length === 0 || fewer.some((step) => step.programs.includes("question-flow"))) {
    throw new Error("the steps of a missing program were built");
  }
});

// Comments aside, the core has no import, static or dynamic, and names no global a host's engine
// may lack: Node's, a clock, or a timer.
check("the core loads no module and names nothing of Node or of a clock", () => {
  const source = readFileSync(new URL("./core.mjs", import.meta.url), "utf8")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
  const found = source.match(/\b(import|require|process|Buffer|performance|Date|setTimeout|setInterval|globalThis|global|console)\b/g);
  if (found !== null) {
    throw new Error(`it names ${[...new Set(found)].join(", ")}`);
  }
});

// The comparison's rule: the median of a step's rounds beyond the threshold, with three quarters
// of them agreeing on the direction, so that neither a wild round nor a mild median names a step.
check("a step is named by the median of its rounds and their agreement", () => {
  const cases = [
    ["a steady 10%", [1.1, 1.09, 1.11, 1.1, 1.12, 1.08, 1.1], "slower"],
    ["10% with one round far the other way", [1.1, 1.09, 0.5, 1.1, 1.12, 1.08, 1.1], "slower"],
    // Six of seven have to agree, so two contrary rounds hide a slowdown: the limit of the rule.
    ["15% with two rounds the other way", [1.15, 1.15, 1.15, 1.15, 1.15, 0.5, 0.5], ""],
    ["two wild rounds on an unchanged step", [1.0, 2.5, 0.99, 1.01, 1.9, 1.0, 0.98], ""],
    ["a median just under the threshold", [1.06, 1.06, 1.06, 1.06, 1.06, 1.06, 1.06], ""],
    ["a median over it that three rounds contradict", [1.09, 1.09, 1.09, 1.09, 0.99, 1.0, 1.01], ""],
    ["a steady 10% faster", [0.9, 0.91, 0.9, 0.89, 0.9, 0.92, 0.9], "faster"],
    ["one round, which is its own median", [1.3], "slower"],
  ];
  for (const [what, ratios, expected] of cases) {
    const actual = verdict(ratios, 0.07);
    if (actual !== expected) {
      throw new Error(`${what}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
  }
});

// The catalog is generated: the committed file is what its script writes, so an edit made to the
// file and not to the script's tables is caught here and not lost at the next regeneration.
const { catalogText } = await import(new URL("large-catalog/generate-catalog.mjs", `${pathToFileURL(corpusRoot).href}/`).href);
check("large-catalog/catalog.nx is what generate-catalog.mjs writes", () => {
  if (readFileSync(join(corpusRoot, "large-catalog/catalog.nx"), "utf8") !== catalogText) {
    throw new Error("the file differs from the script's output; change the script's tables and run it");
  }
});

if (failures > 0) {
  throw new Error(`${failures} check(s) of the harness core failed`);
}
