/**
 * One check over the whole example set.
 *
 * Every example must compile with no diagnostics, and what its `root` prints must equal the
 * `.out.nx` committed beside it. Evaluation goes through the shared IR `PreviewWorkbench` and the wasm module the
 * site ships, so what is checked is what a visitor sees. Every example must also name a docs page
 * the website builds, and the short examples must stay between 10 and 20: enough to start from,
 * few enough to stay out of the way. The larger examples, programs of a realistic size listed last
 * under their own topic, and the test cases after them, are not counted.
 *
 * Usage: pnpm run check-examples [--update]
 *
 * `--update` rewrites every `.out.nx` from the current compiler instead of comparing, for a
 * deliberate language change. Review the diff it leaves.
 */
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { PreviewWorkbench } from "../src/preview/workbench.ts";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const docsRoot = resolve(appRoot, "../website/src/content/docs");
const update = process.argv.includes("--update");

const MIN_EXAMPLES = 10;
const MAX_EXAMPLES = 20;
const UNCOUNTED_TOPICS = new Set(["Larger examples", "Test cases"]);

const examples = JSON.parse(
  readFileSync(join(appRoot, "src/examples/examples.json"), "utf8"),
);
const failures = [];

const short = examples.filter(
  (example) => !UNCOUNTED_TOPICS.has(example.topic),
).length;
if (short < MIN_EXAMPLES || short > MAX_EXAMPLES) {
  failures.push(
    `the set has ${short} short examples; keep it between ${MIN_EXAMPLES} and ${MAX_EXAMPLES}`,
  );
}

const ids = new Set();
for (const example of examples) {
  if (ids.has(example.id)) {
    failures.push(`${example.id}: the id is used twice`);
  }
  ids.add(example.id);
}

const host = createNxHost(await loadNxModule());

for (const example of examples) {
  const label = example.id;
  const failuresBefore = failures.length;
  const sourcePath = join(appRoot, "src/examples/nx", `${example.id}.nx`);
  const outputPath = join(appRoot, "src/examples/nx", `${example.id}.out.nx`);

  const docsProblem = checkDocs(example.docs);
  if (docsProblem !== null) {
    failures.push(`${label}: ${docsProblem}`);
  }

  if (!existsSync(sourcePath)) {
    failures.push(`${label}: no NX source at src/examples/nx/${example.id}.nx`);
    console.log(`not ok - ${example.title}`);
    continue;
  }

  const source = readFileSync(sourcePath, "utf8");
  const result = evaluateExample(source);
  // Why a program does not compile, or the warnings of one that does: an example has neither.
  for (const diagnostic of result.diagnostics) {
    failures.push(`${label}: ${where(diagnostic)}${diagnostic.message}`);
  }
  if (result.outcome === null) {
    failures.push(`${label}: evaluation produced no output`);
    console.log(`not ok - ${example.title}`);
    continue;
  }
  if (result.outcome.kind !== "value") {
    const problems =
      result.outcome.kind === "noRoot"
        ? ["it has no root"]
        : result.outcome.diagnostics.map(
            (diagnostic) => `${where(diagnostic)}${diagnostic.message}`,
          );
    for (const problem of problems) {
      failures.push(`${label}: ${problem}`);
    }
  } else if (result.outcome.truncated) {
    failures.push(`${label}: its value is longer than the output pane shows`);
  } else {
    const actual = `${result.outcome.value.text}\n`;
    if (update) {
      writeFileSync(outputPath, actual);
    } else if (!existsSync(outputPath)) {
      failures.push(
        `${label}: no expected output at src/examples/nx/${example.id}.out.nx (run with --update)`,
      );
    } else {
      const expected = readFileSync(outputPath, "utf8");
      if (expected.trimEnd() !== actual.trimEnd()) {
        failures.push(
          `${label}: its value changed\n--- expected (${example.id}.out.nx)\n${expected.trimEnd()}\n--- actual\n${actual.trimEnd()}`,
        );
      }
    }
  }

  console.log(
    `${failures.length === failuresBefore ? "ok" : "not ok"} - ${example.title}`,
  );
}

host.dispose();

if (failures.length > 0) {
  console.error(`\n${failures.length} problem(s):\n`);
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(
  `\nAll ${examples.length} examples ${update ? "updated" : "match their expected output"}.`,
);

function where(diagnostic) {
  return diagnostic.span === null
    ? ""
    : `${diagnostic.span.startLine}:${diagnostic.span.startColumn} `;
}

/** The same IR workbench supplies the browser's raw Output and current component render. */
function evaluateExample(source) {
  const workbench = new PreviewWorkbench((source) =>
    host.buildProgramArtifact(source, { fileName: "playground.nx" }),
  );
  try {
    const state = workbench.answer({ op: "start", source });
    const rendered = state.session ?? state.structured;
    return {
      diagnostics: [
        ...(state.diagnostics ?? []),
        ...(state.problem != null && "diagnostics" in state.problem
          ? state.problem.diagnostics
          : []),
      ],
      outcome:
        rendered == null
          ? state.hasRoot === false || state.problem?.kind === "noComponent"
            ? { kind: "noRoot" }
            : null
          : "diagnostics" in rendered.output
            ? { kind: "error", diagnostics: rendered.output.diagnostics }
            : { kind: "value", ...rendered.output },
    };
  } finally {
    workbench.dispose();
  }
}

/**
 * Checks that `docs`, `page` or `page#anchor`, names a page the website builds and, with an anchor,
 * a heading on it. Returns what is wrong, or null.
 */
function checkDocs(docs) {
  if (typeof docs !== "string" || docs === "") {
    return "it names no docs page";
  }
  const [page, anchor] = docs.split("#");
  const file = [".md", ".mdx"]
    .map((extension) => join(docsRoot, `${page}${extension}`))
    .find(existsSync);
  if (file === undefined) {
    return `its docs page '${page}' is not a page under sites/website/src/content/docs`;
  }
  if (anchor !== undefined) {
    const headings = readFileSync(file, "utf8")
      .split("\n")
      .filter((line) => /^#{2,6} /.test(line))
      .map((line) => slug(line.replace(/^#+ /, "")));
    if (!headings.includes(anchor)) {
      return `its docs page '${page}' has no heading '#${anchor}'`;
    }
  }
  return null;
}

/** A heading's anchor, as Starlight makes it with github-slugger. */
function slug(heading) {
  return heading
    .replace(/`/g, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{N}\p{Pc} -]/gu, "")
    .replace(/ /g, "-");
}
