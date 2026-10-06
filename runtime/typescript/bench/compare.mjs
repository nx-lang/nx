/**
 * Compares the runtime of the working tree with the runtime of the revision it is based on, on
 * this machine, in one run. Each revision's committed build runs that revision's committed corpus
 * images. In each round every call is timed once for each side, one straight after the other in
 * an isolate of its own, and a step is named when the working tree is slower, or faster, by more
 * than a fraction in most rounds. No timing from
 * another machine is read or kept.
 *
 *   node bench/compare.mjs [--base <ref>] [--rounds <n>] [--samples <n>] [--threshold <fraction>]
 *                          [--base-runtime <dir>] [--head-runtime <dir>] [--json <path>] [--markdown <path>]
 *
 * `--base` is the revision to compare with, the merge base of `HEAD` and `origin/main` by
 * default. Its build (`runtime/typescript/dist/src`) and its corpus programs are read from Git
 * into a temporary directory; nothing is built. `--rounds` is how many times each side runs (7),
 * `--samples` how many warm samples a step gets in a round (40), and `--threshold` how far apart
 * two medians must be to count (0.1). `--base-runtime` and `--head-runtime` name a build
 * directory in place of a revision's; with `--base-runtime` and no `--base`, both sides run the
 * working tree's corpus, which is how two builds that are not revisions are compared.
 *
 * A step is not compared, and says why, when its program's sources or `program.json` differ
 * between the two sides, when the base has no such program, when the base's runtime does not
 * export a function the step calls, or when the step fails on either side. The steps themselves
 * are always the working tree's.
 *
 * The command exits with 1 when it names a step as slower, and with 2 when it could not compare
 * at all: an option it cannot read, a revision with no build.
 */
import { execFileSync } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { PROGRAMS } from "./core.mjs";
import { corpusRoot } from "./corpus.mjs";
import { loadSide, measureWarm } from "./measure.mjs";
import { formatTime, markdownTable, textTable } from "./report.mjs";
import { median } from "./sample.mjs";

const here = fileURLToPath(new URL(".", import.meta.url));
const repository = resolve(here, "../../..");
/** Ends the command as one that could not compare, which is not the same as one that found a step slower. */
function cannotCompare(message) {
  console.error(message);
  process.exit(2);
}
let options;
try {
  ({ values: options } = parseArgs({
    options: {
      base: { type: "string" },
      rounds: { type: "string", default: "7" },
      samples: { type: "string", default: "40" },
      threshold: { type: "string", default: "0.1" },
      "base-runtime": { type: "string" },
      "head-runtime": { type: "string", default: join(here, "../dist/src") },
      json: { type: "string", default: join(here, "out/compare.json") },
      markdown: { type: "string" },
    },
  }));
} catch (error) {
  cannotCompare(error instanceof Error ? error.message : String(error));
}
const rounds = Number(options.rounds);
const samples = Number(options.samples);
const threshold = Number(options.threshold);
if (!Number.isSafeInteger(rounds) || rounds < 1 || !Number.isSafeInteger(samples) || samples < 1 || !(threshold > 0)) {
  cannotCompare("--rounds and --samples each take a count of at least 1, and --threshold a fraction above 0");
}
/** How many rounds must agree for a step to be named: three quarters of them. */
const needed = Math.ceil(rounds * 0.75);

const git = (...args) => execFileSync("git", args, { cwd: repository, encoding: "utf8", maxBuffer: 2 ** 28 }).trim();

/** The paths of `paths` that exist at `ref`, read from Git into `into`. */
function extract(ref, paths, into) {
  const present = paths.filter((path) => {
    try {
      execFileSync("git", ["cat-file", "-e", `${ref}:${path}`], { cwd: repository, stdio: "ignore" });
      return true;
    } catch {
      return false;
    }
  });
  if (present.length > 0) {
    const archive = execFileSync("git", ["archive", "--format=tar", ref, "--", ...present], { cwd: repository, maxBuffer: 2 ** 28 });
    execFileSync("tar", ["-x", "-C", into], { input: archive });
  }
}

const temporary = mkdtempSync(join(tmpdir(), "nx-bench-"));
process.on("exit", () => rmSync(temporary, { recursive: true, force: true }));
let report;
try {
  // The base: a revision's build and corpus, or a build directory against this tree's corpus.
  const builds = options["base-runtime"] !== undefined && options.base === undefined;
  let base;
  if (builds) {
    base = { name: resolve(options["base-runtime"]), runtime: resolve(options["base-runtime"]), corpus: corpusRoot };
  } else {
    let ref = options.base;
    if (ref === undefined) {
      try {
        ref = git("merge-base", "HEAD", "origin/main");
      } catch {
        cannotCompare("HEAD has no merge base with origin/main; name the revision to compare with as --base <ref>");
      }
    }
    try {
      ref = git("rev-parse", "--verify", "--quiet", `${ref}^{commit}`);
    } catch {
      cannotCompare(`${ref} is not a revision of this repository`);
    }
    extract(ref, ["runtime/typescript/dist/src", ...PROGRAMS.map((name) => `specs/ir-conformance/${name}`)], temporary);
    // The build is ECMAScript modules, which Node reads as such only under a package that says so.
    writeFileSync(join(temporary, "package.json"), '{ "type": "module" }\n');
    mkdirSync(join(temporary, "specs/ir-conformance"), { recursive: true });
    base = {
      name: `${git("rev-parse", "--short", ref)}${options.base === undefined ? " (the merge base with origin/main)" : ` (${options.base})`}`,
      runtime: resolve(options["base-runtime"] ?? join(temporary, "runtime/typescript/dist/src")),
      corpus: join(temporary, "specs/ir-conformance"),
    };
  }
  if (!existsSync(join(base.runtime, "index.js"))) {
    cannotCompare(`${base.name} has no build of the runtime at ${builds || options["base-runtime"] !== undefined ? base.runtime : "runtime/typescript/dist/src"}`);
  }
  const head = {
    name: resolve(options["head-runtime"]) === resolve(here, "../dist/src") ? "the working tree" : resolve(options["head-runtime"]),
    runtime: resolve(options["head-runtime"]),
    corpus: corpusRoot,
  };

  /** A program's sources and `program.json`, by path: what must be equal for two sides to run one workload. */
  const workload = (root, name) => {
    const files = new Map();
    const walk = (dir, prefix) => {
      for (const entry of readdirSync(dir).sort()) {
        const path = join(dir, entry);
        if (statSync(path).isDirectory()) {
          if (entry !== "expected") {
            walk(path, `${prefix}${entry}/`);
          }
        } else if (entry.endsWith(".nx") || entry === "program.json") {
          files.set(`${prefix}${entry}`, readFileSync(path, "utf8"));
        }
      }
    };
    if (existsSync(join(root, name))) {
      walk(join(root, name), "");
    }
    return JSON.stringify([...files]);
  };
  const incomparable = new Map();
  for (const name of PROGRAMS) {
    if (!existsSync(join(base.corpus, name, "program.json"))) {
      incomparable.set(name, `the base has no ${name}`);
    } else if (workload(base.corpus, name) !== workload(head.corpus, name)) {
      incomparable.set(name, `${name} differs from the base's`);
    }
  }

  if (!existsSync(join(head.runtime, "index.js"))) {
    cannotCompare(`${head.name} has no build of the runtime at ${head.runtime}`);
  }
  const sides = { base: await loadSide(base.runtime, base.corpus), head: await loadSide(head.runtime, head.corpus) };
  // What a row is compared with, or why it is not: the steps are the working tree's.
  const rows = sides.head.rows.map((row) => {
    const other = sides.base.rows.find((candidate) => candidate.name === row.name);
    const reason =
      row.programs.map((name) => incomparable.get(name)).find((why) => why !== undefined) ??
      row.unavailable?.replace("the runtime", "the working tree's runtime") ??
      (other === undefined ? "the base has no such step" : other.unavailable?.replace("the runtime", "the base's runtime"));
    return { row, other, reason, base: [], head: [] };
  });
  for (let round = 0; round < rounds; round += 1) {
    for (const [position, compared] of rows.entries()) {
      // A row with nothing to compare with is still timed, once, where this build can run it.
      if (compared.row.unavailable !== undefined || (compared.reason !== undefined && round > 0)) {
        continue;
      }
      // The side that runs first alternates, by round and by row, so that neither always runs
      // on the warmer machine.
      const order = compared.reason !== undefined ? ["head"] : (round + position) % 2 === 0 ? ["base", "head"] : ["head", "base"];
      for (const side of order) {
        try {
          compared[side].push(await measureWarm(sides[side], side === "head" ? compared.row : compared.other, samples));
        } catch (error) {
          // A step one side cannot run is a step that cannot be compared, not a comparison that
          // cannot be made: the other steps still are.
          const message = (error instanceof Error ? error.message : String(error)).split("\n")[0];
          compared.reason = `it fails with ${side === "head" ? "the working tree's" : "the base's"} runtime: ${message}`;
          // When it was the base that failed first, the working tree is still timed, as a row
          // with nothing to compare with is.
          if (side === "head") {
            break;
          }
        }
      }
    }
    process.stderr.write(`round ${round + 1} of ${rounds}\n`);
  }

  const steps = [];
  for (const { row, reason, base: baseRuns, head: headRuns } of rows) {
    for (const { variant } of row.variants) {
      const step = { step: row.name, variant, calls: row.calls };
      if (headRuns.length > 0 && headRuns[0][variant] !== undefined) {
        step.head = median(headRuns.map((run) => run[variant].median));
        if (variant === "limited") {
          step.headOperations = headRuns[0].operations;
        }
      }
      if (reason !== undefined) {
        steps.push({ ...step, verdict: "not comparable", reason });
        continue;
      }
      const ratios = headRuns.map((run, round) => run[variant].median / baseRuns[round][variant].median);
      step.base = median(baseRuns.map((run) => run[variant].median));
      step.ratio = median(ratios);
      step.ratios = ratios;
      if (variant === "limited") {
        step.baseOperations = baseRuns[0].operations;
      }
      step.verdict =
        ratios.filter((ratio) => ratio > 1 + threshold).length >= needed
          ? "slower"
          : ratios.filter((ratio) => ratio < 1 / (1 + threshold)).length >= needed
            ? "faster"
            : "";
      steps.push(step);
    }
  }
  report = {
    base: base.name,
    head: head.name,
    node: process.version,
    platform: `${process.platform} ${process.arch}`,
    rounds,
    samples,
    threshold,
    steps,
  };
} catch (error) {
  cannotCompare(error instanceof Error ? (error.stack ?? error.message) : String(error));
}

const named = (verdict) => report.steps.filter((step) => step.verdict === verdict);
const operationsChanged = report.steps.filter((step) => step.baseOperations !== undefined && step.baseOperations !== step.headOperations);
const lines = [
  `${report.head[0].toUpperCase()}${report.head.slice(1)} against ${report.base}: ${report.rounds} rounds of ${report.samples} samples on ${report.node}, ${report.platform}.`,
  `A step is named when it is beyond ${(threshold * 100).toFixed(0)}% in ${needed} of ${report.rounds} rounds.`,
  "",
];
const list = (title, steps, describe) => {
  if (steps.length > 0) {
    lines.push(title, ...steps.map((step) => `- ${step.step} (${step.variant}): ${describe(step)}`), "");
  }
};
const times = (step) => `${formatTime(step.base)} at the base, ${formatTime(step.head)} now, ${step.ratio.toFixed(2)}x`;
list("Slower:", named("slower"), times);
list("Faster:", named("faster"), times);
list("Operations changed:", operationsChanged, (step) => `${step.baseOperations} at the base, ${step.headOperations} now`);
if (named("slower").length + named("faster").length + operationsChanged.length === 0) {
  lines.push("No step is slower or faster, and none costs a different number of operations.", "");
}
const reasons = [...new Set(named("not comparable").map((step) => step.reason))];
if (reasons.length > 0) {
  lines.push(`Not compared: ${reasons.join("; ")}.`, "");
}

const headings = ["step", "variant", "base", "now", "ratio", "least", "most", "operations", "verdict"];
const right = ["base", "now", "ratio", "least", "most", "operations"];
const cells = report.steps.map((step) => [
  step.calls === 1 ? step.step : `${step.step}, each of ${step.calls}`,
  step.variant,
  formatTime(step.base),
  formatTime(step.head),
  step.ratio === undefined ? "" : step.ratio.toFixed(2),
  step.ratios === undefined ? "" : Math.min(...step.ratios).toFixed(2),
  step.ratios === undefined ? "" : Math.max(...step.ratios).toFixed(2),
  step.headOperations === undefined
    ? ""
    : step.baseOperations === undefined || step.baseOperations === step.headOperations
      ? String(step.headOperations)
      : `${step.baseOperations} -> ${step.headOperations}`,
  step.verdict === "not comparable" ? `not comparable: ${step.reason}` : step.verdict,
]);
console.log(`${lines.join("\n")}\n${textTable(headings, cells, right)}`);
if (options.markdown !== undefined) {
  appendFileSync(resolve(options.markdown), `## Runtime performance\n\n${lines.join("\n")}\n${markdownTable(headings, cells, right)}\n`);
}
mkdirSync(dirname(resolve(options.json)), { recursive: true });
writeFileSync(resolve(options.json), `${JSON.stringify(report, undefined, 2)}\n`);
console.log(`\nwritten to ${resolve(options.json)}`);
if (named("slower").length > 0) {
  process.exitCode = 1;
}
