/**
 * Timing in Node: what a number in the harness's report means.
 *
 * - **Warm** is a step after it has run until its time stopped falling: the median and the 95th
 *   percentile of the samples taken after that. A step shorter than 0.1 ms is sampled in batches,
 *   so that a sample is long enough for the clock to measure. Each call is warmed and sampled in
 *   an isolate of its own, so that no step runs on what another step taught the engine and the
 *   order of the steps does not matter. A host's isolate runs a mix of calls, and its times are
 *   not below these. An isolate now and then settles on slower code for a call than the others
 *   do, so a report takes three and gives the one whose median is in the middle.
 * - **Cold** is the first execution of a step in a fresh isolate, after the runtime module was
 *   loaded there. It includes the engine compiling the runtime's code for that step. It
 *   approximates the first request an isolate serves; it is not another engine's cold start.
 *
 * An isolate is a `worker_threads` worker. Times are in milliseconds, for one call: a step that
 * makes several calls in a run is divided by its `divisor`.
 */
import { Worker } from "node:worker_threads";

/** The shortest a sample may be, in milliseconds. */
const SHORTEST_SAMPLE = 0.1;
/** The least a step is run before it is sampled, in milliseconds: long enough for the engine's last tier. */
const SHORTEST_WARM_UP = 100;
/** How many samples a warm-up window holds, and how many windows in a row must not improve. */
const WINDOW = 10;
const FLAT_WINDOWS = 3;
/** The most windows a warm-up takes once it has run that long, however long the step keeps getting faster. */
const MOST_WINDOWS = 60;
/** What counts as a window improving on the best one so far. */
const IMPROVEMENT = 0.97;

/** The middle of `values` in order: of an even number of them, halfway between the two middle ones. */
export function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = sorted.length >> 1;
  return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

/** The value a fraction `q` of the way through `values` in order, by the nearest rank. */
export function quantile(values, q) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil(q * sorted.length) - 1))];
}

function time(step, runs) {
  const start = performance.now();
  for (let run = 0; run < runs; run += 1) {
    step.run();
  }
  return performance.now() - start;
}

/** How many runs of `step` make a sample of at least {@link SHORTEST_SAMPLE}. */
function runsPerSample(step) {
  let runs = 1;
  while (time(step, runs) < SHORTEST_SAMPLE && runs < 2 ** 20) {
    runs *= 2;
  }
  return runs;
}

/**
 * The warm time of each of `steps`, in their order, in this isolate:
 * `{ median, p95, samples, runsPerSample }`, in milliseconds for one call. Steps given together
 * are sampled in turn, one sample of each, so that whatever drifts while they run (the engine's
 * tiers, the machine's load) falls on all of them alike: it is how the two variants of one call
 * are compared.
 */
export function sampleWarm(steps, samples) {
  for (const step of steps) {
    step.ready();
    const runs = runsPerSample(step);
    const started = performance.now();
    let best = Infinity;
    for (let window = 0, flat = 0; performance.now() - started < SHORTEST_WARM_UP || (window < MOST_WINDOWS && flat < FLAT_WINDOWS); window += 1) {
      const now = median(Array.from({ length: WINDOW }, () => time(step, runs)));
      flat = now < best * IMPROVEMENT ? 0 : flat + 1;
      best = Math.min(best, now);
    }
  }
  // A warm step is faster than the one the first count was taken from.
  const sampled = steps.map((step) => ({ step, runs: runsPerSample(step), times: [] }));
  for (let taken = 0; taken < samples; taken += 1) {
    for (const { step, runs, times } of sampled) {
      times.push(time(step, runs) / runs / step.divisor);
    }
  }
  return sampled.map(({ runs, times }) => ({ median: median(times), p95: quantile(times, 0.95), samples, runsPerSample: runs }));
}

/**
 * Runs `task` in a fresh isolate that has loaded the runtime at `runtimeUrl` and built the steps
 * from `corpus` and `stored`. A task is one of:
 *
 * - `{ kind: "cold", index }`: the first call of the step at `index`. Resolves to
 *   `{ load, first }`, the time to load the runtime module and the time of that call.
 * - `{ kind: "warm", indices, samples }`: the steps at `indices`, warmed and sampled together.
 *   Resolves to `{ load, first, timings, usages }`: the time to load the runtime module, the time
 *   of the first call of the first step, made before anything was warm, and, each in the order
 *   of `indices`, what {@link sampleWarm} returned and what one more run of each step reported.
 */
export function inFreshIsolate(task, { runtimeUrl, corpus, stored }) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./worker.mjs", import.meta.url), { workerData: { task, runtimeUrl, corpus, stored } });
    worker.once("message", resolve);
    worker.once("error", reject);
    worker.once("exit", (code) => {
      if (code !== 0) {
        reject(new Error(`the worker exited with code ${code}`));
      }
    });
  });
}
