/**
 * Measuring one build of the runtime against one checkout of the corpus, in Node: what `run.mjs`
 * does for one side and `compare.mjs` for two.
 */
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { buildSteps, stepName, storedStates } from "./core.mjs";
import { loadCorpus } from "./corpus.mjs";
import { inFreshIsolate, median } from "./sample.mjs";

/**
 * A side to measure: the build in `runtimeDir` and the corpus in `corpusDir`, with the calls it
 * can make as `rows`. A row is one call, `{ name, workload, phase, subject, programs, calls }`,
 * whose variants are the steps of that name; it has `unavailable`, a reason, when this build does
 * not export a function the call needs. A program the corpus lacks has no rows.
 */
export async function loadSide(runtimeDir, corpusDir) {
  const runtimeUrl = pathToFileURL(join(resolve(runtimeDir), "index.js")).href;
  const runtime = await import(runtimeUrl);
  const corpus = loadCorpus(resolve(corpusDir));
  // The stored states are made once, here, and passed to every isolate: a host that resumes a
  // component read its state from storage and did not dispatch to make it.
  const resumes = ["initializeComponent", "dispatchComponentActions"].every((name) => typeof runtime[name] === "function");
  const stored = resumes ? storedStates(runtime, corpus) : undefined;
  const rows = new Map();
  for (const [index, step] of buildSteps(runtime, corpus, stored).entries()) {
    const name = stepName(step);
    if (!rows.has(name)) {
      rows.set(name, {
        name,
        workload: step.workload,
        phase: step.phase,
        subject: step.subject,
        programs: step.programs,
        calls: step.divisor,
        variants: [],
      });
    }
    const row = rows.get(name);
    row.variants.push({ index, variant: step.variant, recorded: step.recorded });
    const lacks = step.requires.find((required) => typeof runtime[required] !== "function");
    if (lacks !== undefined) {
      row.unavailable = `the runtime has no ${lacks}`;
    }
  }
  return { runtimeDir: resolve(runtimeDir), isolate: { runtimeUrl, corpus, stored }, rows: [...rows.values()] };
}

/**
 * The warm times of one row, its variants sampled together in an isolate of their own:
 * `{ unlimited, limited, operations, inputSize, recorded, nsPerOperation, load, first }`. The
 * four after `limited` are from the variant with limits, where the row has one and the corpus
 * records a count; `load` and `first` are that isolate's cold times, of loading the runtime module
 * and of the first call of the variant with no limits. With `isolates` above 1 the row is timed
 * in that many isolates, one after another, and the times are those of the isolate whose median
 * for the first variant is the middle one.
 */
export async function measureWarm(side, row, samples, isolates = 1) {
  const task = { kind: "warm", indices: row.variants.map(({ index }) => index), samples };
  const runs = [];
  for (let isolate = 0; isolate < isolates; isolate += 1) {
    runs.push(await inFreshIsolate(task, side.isolate));
  }
  runs.sort((left, right) => left.timings[0].median - right.timings[0].median);
  const { load, first, timings, usages } = runs[(runs.length - 1) >> 1];
  const measured = { load, first };
  for (const [position, { variant, recorded }] of row.variants.entries()) {
    measured[variant] = timings[position];
    if (variant === "limited") {
      const { operations, inputSize } = usages[position];
      Object.assign(measured, { operations, inputSize }, recorded === undefined ? {} : { recorded });
      if (operations > 0) {
        measured.nsPerOperation = (measured.unlimited.median * row.calls * 1e6) / operations;
      }
    }
  }
  return measured;
}

/**
 * The cold time of one row, the first call of its variant with no limits in each of `samples`
 * fresh isolates: `{ median, max, samples, loads }`, where `loads` are the times the runtime
 * module took to load in those isolates.
 */
export async function measureCold(side, row, samples) {
  const { index } = row.variants.find(({ variant }) => variant === "unlimited");
  const firsts = [];
  const loads = [];
  for (let sample = 0; sample < samples; sample += 1) {
    const { load, first } = await inFreshIsolate({ kind: "cold", index }, side.isolate);
    loads.push(load);
    firsts.push(first);
  }
  return { median: median(firsts), max: Math.max(...firsts), samples, loads };
}
