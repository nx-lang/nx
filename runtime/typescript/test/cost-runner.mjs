/**
 * The TypeScript runtime's side of the cost validation. The harness in
 * `crates/nx-codegen/tests/cost_differential.rs` compiles the probe library, generates the cases
 * and writes both to a file; this script runs every case through the TypeScript runtime and
 * writes its answers, which the harness compares with the Rust runtime's.
 *
 *   node test/cost-runner.mjs <cases.json> <answers.json>
 *
 * The input names the images of the probe library, the entry module, the mode and the cases. A
 * case holds one call at two scales, the second with its host value eight times the size and its
 * step repeated eight times as often. In the mode `answers`, the answer for a case is, at the
 * base scale, its result with no budget, its operation count and input size as the usage report
 * gives them, whether the result under a budget equal to the count is the result under none, and
 * where budgets below the count stop it; and at the larger scale its count and input size. A
 * case the runtime refuses with no budget has only the diagnostic's code and the argument it
 * names, which is recorded wherever a diagnostic is. In the mode `time`, the
 * answer is the least of five timings of each scale, with the count and the input size.
 *
 * It imports the committed `dist`, as a host would, and reads nothing else of the repository.
 */
import { readFileSync, writeFileSync } from "node:fs";

import {
  NxIrRuntimeError,
  applyComponentStatePatch,
  callFunction,
  constructComponentDescriptor,
  dispatchComponentActions,
  evaluateComponent,
  evaluateFunction,
  initializeComponent,
  linkNxIrProgram,
  normalizeComponentState,
  prepareNxIrModule,
} from "../dist/src/index.js";

const [, , inputPath, outputPath] = process.argv;
if (inputPath === undefined || outputPath === undefined) {
  throw new Error("usage: node test/cost-runner.mjs <cases.json> <answers.json>");
}
const input = JSON.parse(readFileSync(inputPath, "utf8"));
const modules = new Map(
  input.images.map(({ identity, path }) => [identity, prepareNxIrModule(new Uint8Array(readFileSync(path)))]),
);
const program = linkNxIrProgram(modules.get(input.entry), { resolve: (identity) => modules.get(identity) });

/** A budget and an input limit no case reaches: what a host sets to read what a call used and limit nothing. */
const ample = 2 ** 40;

/**
 * What a dispatch runs against: the instance its component initializes to, under no limit. It is
 * made once, outside what is measured, as a host that holds an instance would have.
 */
function prepare(call) {
  return call.kind === "dispatch" ? initializeComponent(program, call.component, call.props).instance : undefined;
}

/** Makes the call under `options` and returns its result in the form the Rust harness writes its own. */
function run(call, instance, options) {
  switch (call.kind) {
    case "function":
      return evaluateFunction(program, call.name, call.arguments, options);
    case "named":
      return callFunction(program, { $type: "Function", module: input.entry, name: call.name }, call.arguments, options);
    case "initialize": {
      const result = initializeComponent(program, call.component, call.props, call.state === undefined ? options : { ...options, state: call.state });
      return { rendered: result.rendered, effects: [], state: result.state };
    }
    case "evaluate":
      return evaluateComponent(program, call.component, call.props, call.state, options).rendered;
    case "descriptor":
      return constructComponentDescriptor(program, call.component, call.props, call.content, options);
    case "dispatch": {
      const result = dispatchComponentActions(program, instance, call.batch, options);
      return { rendered: result.rendered, effects: result.effects, state: result.state };
    }
    case "normalize":
      return normalizeComponentState(program, call.component, call.state, options);
    case "patch":
      return applyComponentStatePatch(program, call.component, call.state, call.patch, options);
    default:
      throw new Error(`no call is of kind ${call.kind}`);
  }
}

/**
 * The code a call was refused with and the argument the diagnostic names, `null` when it names
 * none, or what the call threw that is not the runtime's own error.
 */
function refusal(error) {
  if (error instanceof NxIrRuntimeError) {
    return { refused: error.diagnostics[0].code, argument: error.diagnostics[0].argument ?? null };
  }
  return { threw: `${error?.constructor?.name}: ${error instanceof Error ? error.message : String(error)}` };
}

/** The budgets below a count that failures are recorded at: all of them up to 200, and otherwise a quarter, a half and one less. */
function budgetsBelow(operations) {
  if (operations <= 200) {
    return Array.from({ length: operations }, (_, budget) => budget);
  }
  return [Math.floor(operations / 4), Math.floor(operations / 2), operations - 1];
}

/** The answer for one scale: the refusal, or the operations and the input size the usage report gives. */
function measured(call) {
  const usage = {};
  try {
    run(call, prepare(call), { maxOperations: ample, maxInputSize: ample, usage });
  } catch (error) {
    return refusal(error);
  }
  return { operations: usage.operations ?? null, inputSize: usage.inputSize ?? null };
}

function answer(testCase) {
  const call = testCase.base;
  const instance = prepare(call);
  let unlimited;
  try {
    unlimited = JSON.stringify(run(call, instance, {}));
  } catch (error) {
    return { base: refusal(error) };
  }
  const usage = {};
  try {
    run(call, instance, { maxOperations: ample, maxInputSize: ample, usage });
  } catch (error) {
    const refused = refusal(error);
    return { base: { refusedUnderAmpleLimits: refused.refused ?? refused.threw, argument: refused.argument ?? null } };
  }
  const operations = usage.operations ?? null;
  const inputSize = usage.inputSize ?? null;
  let resultUnderCount = false;
  try {
    resultUnderCount = JSON.stringify(run(call, instance, { maxOperations: operations ?? 0, usage })) === unlimited;
  } catch {
    resultUnderCount = false;
  }
  const failures = budgetsBelow(operations ?? 0).map((budget) => {
    try {
      run(call, instance, { maxOperations: budget, usage });
    } catch (error) {
      if (!(error instanceof NxIrRuntimeError)) {
        return { budget, ...refusal(error) };
      }
      const diagnostic = error.diagnostics[0];
      return {
        budget,
        code: diagnostic.code,
        argument: diagnostic.argument ?? null,
        limit: diagnostic.limit?.name ?? null,
        declaration: diagnostic.declaration ?? null,
        source: diagnostic.source ?? null,
        operations: usage.operations ?? null,
      };
    }
    return { budget, succeeded: true };
  });
  return {
    base: { operations, inputSize, result: JSON.parse(unlimited), resultUnderCount, failures },
    scaled: measured(testCase.scaled),
  };
}

/** The least of five timings of a call under limits it cannot reach, in milliseconds, with what it used. */
function timed(call) {
  const usage = {};
  let least = Infinity;
  for (let round = 0; round < 5; round += 1) {
    const instance = prepare(call);
    const options = { maxOperations: ample, maxInputSize: ample, usage };
    const start = performance.now();
    try {
      run(call, instance, options);
    } catch (error) {
      return refusal(error);
    }
    least = Math.min(least, performance.now() - start);
  }
  return { milliseconds: least, operations: usage.operations ?? null, inputSize: usage.inputSize ?? null };
}

const answers = input.cases.map((testCase) =>
  input.mode === "time" ? { base: timed(testCase.base), scaled: timed(testCase.scaled) } : answer(testCase),
);
writeFileSync(outputPath, JSON.stringify(answers));
