/**
 * The harness's Node driver: times every step of `core.mjs` against a build of the runtime and
 * prints the report, as a table and as JSON.
 *
 *   node bench/run.mjs [--runtime <dir>] [--corpus <dir>] [--samples <n>] [--isolates <n>] [--cold <n>]
 *                      [--json <path>]
 *
 * `--runtime` is the directory of a built runtime, `dist/src` of this package by default, and
 * `--corpus` a checkout of `specs/ir-conformance`, this repository's by default. `--samples` is
 * how many warm samples a step gets (200), `--isolates` how many isolates a step is warmed and
 * sampled in, of which the middle one is reported (3), and `--cold` how many cold samples it gets
 * (15; 0 takes none).
 * The JSON goes to `--json`, `bench/out/bench.json` by default; `--json -` prints it in place of
 * the table. `sample.mjs` says what cold and warm mean; every call is timed in an isolate of its
 * own.
 *
 * A step whose program the corpus lacks is not in the report, and one that calls a function the
 * runtime does not export is reported as unavailable, which is how a build from an earlier
 * revision is measured.
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { cpus } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { corpusRoot } from "./corpus.mjs";
import { loadSide, measureCold, measureWarm } from "./measure.mjs";
import { formatTime, textTable } from "./report.mjs";
import { median } from "./sample.mjs";

const here = fileURLToPath(new URL(".", import.meta.url));
const { values: options } = parseArgs({
  options: {
    runtime: { type: "string", default: join(here, "../dist/src") },
    corpus: { type: "string", default: corpusRoot },
    samples: { type: "string", default: "200" },
    isolates: { type: "string", default: "3" },
    cold: { type: "string", default: "15" },
    json: { type: "string", default: join(here, "out/bench.json") },
  },
});
const count = (name, least) => {
  const value = Number(options[name]);
  if (!Number.isSafeInteger(value) || value < least) {
    throw new Error(`--${name} takes a count of at least ${least}, not ${options[name]}`);
  }
  return value;
};
const samples = count("samples", 1);
const isolates = count("isolates", 1);
const coldSamples = count("cold", 0);

const side = await loadSide(options.runtime, options.corpus);
const loads = [];
const rows = [];
for (const { variants, name, ...row } of side.rows) {
  rows.push(row);
  if (row.unavailable !== undefined) {
    continue;
  }
  Object.assign(row, await measureWarm(side, { ...row, variants }, samples, isolates));
  if (coldSamples > 0) {
    const { loads: loaded, ...cold } = await measureCold(side, { variants }, coldSamples);
    loads.push(...loaded);
    row.cold = cold;
  }
}

const report = {
  node: process.version,
  platform: `${process.platform} ${process.arch}`,
  cpu: cpus()[0]?.model ?? "unknown",
  runtime: side.runtimeDir,
  samples,
  isolates,
  coldSamples,
  ...(loads.length === 0 ? {} : { load: { median: median(loads), max: Math.max(...loads), samples: loads.length } }),
  steps: rows,
};

const json = `${JSON.stringify(report, undefined, 2)}\n`;
if (options.json === "-") {
  process.stdout.write(json);
} else {
  const headings = ["workload", "phase", "subject", "cold", "cold max", "warm", "p95", "limited", "p95 ", "operations", "input", "ns/op"];
  const table = textTable(
    headings,
    report.steps.map((row) => [
      row.workload,
      row.phase,
      row.calls === 1 ? row.subject : `${row.subject}, each of ${row.calls}`,
      ...(row.unavailable !== undefined
        ? [row.unavailable, "", "", "", "", "", "", "", ""]
        : [
            formatTime(row.cold?.median),
            formatTime(row.cold?.max),
            formatTime(row.unlimited.median),
            formatTime(row.unlimited.p95),
            formatTime(row.limited?.median),
            formatTime(row.limited?.p95),
            row.operations === undefined
              ? ""
              : `${row.operations}${row.recorded !== undefined && row.recorded !== row.operations ? ` (recorded ${row.recorded})` : ""}`,
            row.inputSize === undefined ? "" : String(row.inputSize),
            row.nsPerOperation === undefined ? "" : row.nsPerOperation.toFixed(1),
          ]),
    ]),
    headings.slice(3),
  );
  console.log(`@nx-lang/ir-runtime, ${report.node} on ${report.platform}, ${report.cpu}`);
  console.log(`runtime ${side.runtimeDir}`);
  if (report.load !== undefined) {
    console.log(`load, cold: median ${formatTime(report.load.median)}, largest ${formatTime(report.load.max)} of ${report.load.samples}`);
  }
  console.log(
    `warm: ${samples} samples a step, the middle of ${isolates} isolates; cold: ${coldSamples}; times are for one call, operations and input for a whole run\n`,
  );
  console.log(table);
  mkdirSync(dirname(resolve(options.json)), { recursive: true });
  writeFileSync(resolve(options.json), json);
  console.log(`\nwritten to ${resolve(options.json)}`);
}
