/**
 * One isolate of the harness: loads the runtime module, builds the steps, and does the one task
 * it was started for, which `inFreshIsolate` of `sample.mjs` describes. A step is made ready
 * before its first call is timed: a host's first request also prepares and links before it
 * evaluates.
 */
import { parentPort, workerData } from "node:worker_threads";

import { buildSteps } from "./core.mjs";
import { sampleWarm } from "./sample.mjs";

const { task, runtimeUrl, corpus, stored } = workerData;

const loading = performance.now();
const runtime = await import(runtimeUrl);
const load = performance.now() - loading;
const steps = buildSteps(runtime, corpus, stored);

if (task.kind === "cold") {
  const step = steps[task.index];
  step.ready();
  const starting = performance.now();
  (step.first ?? step.run)();
  parentPort.postMessage({ load, first: performance.now() - starting });
} else {
  const picked = task.indices.map((index) => steps[index]);
  const timings = sampleWarm(picked, task.samples);
  parentPort.postMessage({ timings, usages: picked.map((step) => step.run()) });
}
