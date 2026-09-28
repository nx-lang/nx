/**
 * One check over the whole example set.
 *
 * Every example must compile with no diagnostics, and what its `root` prints must equal the
 * `.out.nx` committed beside it. Evaluation goes through `evaluateSource` and the wasm module the
 * site ships, so what is checked is what a visitor sees. Every example must also name a docs page
 * the website builds, and the set must stay between 10 and 20 examples: enough to start from,
 * few enough to stay out of the way.
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
import { evaluateSource } from "../src/compile/evaluate.ts";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const docsRoot = resolve(appRoot, "../website/src/content/docs");
const update = process.argv.includes("--update");

const MIN_EXAMPLES = 10;
const MAX_EXAMPLES = 20;

const examples = JSON.parse(readFileSync(join(appRoot, "src/examples/examples.json"), "utf8"));
const failures = [];

if (examples.length < MIN_EXAMPLES || examples.length > MAX_EXAMPLES) {
  failures.push(`the set has ${examples.length} examples; keep it between ${MIN_EXAMPLES} and ${MAX_EXAMPLES}`);
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

  const result = evaluateSource(host, readFileSync(sourcePath, "utf8"));
  if (result.outcome === null) {
    for (const diagnostic of result.diagnostics) {
      failures.push(`${label}: ${where(diagnostic)}${diagnostic.message}`);
    }
  } else if (result.outcome.kind !== "value") {
    const problems =
      result.outcome.kind === "noRoot"
        ? ["it has no root"]
        : result.outcome.diagnostics.map((diagnostic) => `${where(diagnostic)}${diagnostic.message}`);
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
      failures.push(`${label}: no expected output at src/examples/nx/${example.id}.out.nx (run with --update)`);
    } else {
      const expected = readFileSync(outputPath, "utf8");
      if (expected.trimEnd() !== actual.trimEnd()) {
        failures.push(`${label}: its value changed\n--- expected (${example.id}.out.nx)\n${expected.trimEnd()}\n--- actual\n${actual.trimEnd()}`);
      }
    }
  }

  console.log(`${failures.length === failuresBefore ? "ok" : "not ok"} - ${example.title}`);
}

host.dispose();

if (failures.length > 0) {
  console.error(`\n${failures.length} problem(s):\n`);
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`\nAll ${examples.length} examples ${update ? "updated" : "match their expected output"}.`);

function where(diagnostic) {
  return diagnostic.span === null ? "" : `${diagnostic.span.startLine}:${diagnostic.span.startColumn} `;
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
  const file = [".md", ".mdx"].map((extension) => join(docsRoot, `${page}${extension}`)).find(existsSync);
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
