/**
 * Checks every `nx` code block in the site's content against the current NX compiler.
 *
 * Each block is compiled as a module of its own. How it passes depends on the words after `nx` in
 * its fence:
 *
 * - none: the block compiles with no error diagnostics;
 * - `fragment`: the block has no syntax diagnostics, so it may use names declared elsewhere;
 * - `invalid`: the block produces at least one error, because it shows a form NX rejects.
 *
 * Failures print as `file:line:column: message`, and any failure exits nonzero.
 *
 * Usage: node --no-warnings scripts/check-code-blocks.mjs [file-or-directory ...]
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { fromMarkdown } from "mdast-util-from-markdown";
import { mdxFromMarkdown } from "mdast-util-mdx";
import { mdxjs } from "micromark-extension-mdxjs";
import { createNxHost, loadNxModule, NxEvaluationError } from "@nx-lang/sdk-wasm";

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(siteRoot, "../..");

export const KINDS = ["fragment", "invalid"];

/**
 * Diagnostic codes the parser and syntax validation report, read from the syntax crate so the list
 * can't drift from the compiler. A `fragment` passes when none of these is reported.
 */
export const SYNTAX_CODES = readSyntaxCodes(join(repoRoot, "crates/nx-syntax/src"));

function readSyntaxCodes(directory) {
  const codes = new Set();
  for (const name of readdirSync(directory)) {
    if (extname(name) !== ".rs") {
      continue;
    }
    const text = readFileSync(join(directory, name), "utf8");
    for (const match of text.matchAll(/Diagnostic::(?:error|warning)\("([a-z-]+)"\)/g)) {
      codes.add(match[1]);
    }
  }
  return codes;
}

/**
 * Finds the `nx` fences in one Markdown or MDX text, with the line each fence starts on and its
 * kind: `"complete"` for an unmarked block, otherwise the marking word.
 */
export function findNxBlocks(text, { mdx = false } = {}) {
  const tree = fromMarkdown(blankFrontmatter(text), mdx
    ? { extensions: [mdxjs()], mdastExtensions: [mdxFromMarkdown()] }
    : {});
  const blocks = [];
  visit(tree, (node) => {
    if (node.type !== "code" || node.lang !== "nx") {
      return;
    }
    const words = (node.meta ?? "").split(/\s+/).filter((word) => KINDS.includes(word));
    blocks.push({
      line: node.position.start.line,
      code: node.value,
      kinds: words,
      kind: words[0] ?? "complete"
    });
  });
  return blocks;
}

// Frontmatter isn't Markdown; blanking it keeps line numbers while hiding it from the parser.
function blankFrontmatter(text) {
  const match = /^---\r?\n[\s\S]*?\r?\n---\r?\n/.exec(text);
  return match ? match[0].replace(/[^\n]/g, "") + text.slice(match[0].length) : text;
}

function visit(node, callback) {
  callback(node);
  for (const child of node.children ?? []) {
    visit(child, callback);
  }
}

/**
 * Checks one block and returns its problems, each `{ line, column, message }` with the position
 * relative to the block's first code line.
 */
export function checkBlock(host, block) {
  if (block.kinds.length > 1) {
    return [{ line: 0, column: 1, message: `a block is one of ${KINDS.join(" or ")}, not both` }];
  }
  const errors = compileErrors(host, block.code);
  switch (block.kind) {
    case "complete":
      return errors.map(toProblem);
    case "fragment":
      return errors.filter((diagnostic) => SYNTAX_CODES.has(diagnostic.code)).map(toProblem);
    case "invalid":
      return errors.length > 0
        ? []
        : [{ line: 0, column: 1, message: "an `nx invalid` block compiles without errors" }];
  }
}

function compileErrors(host, source) {
  try {
    host.buildProgramArtifact(source).dispose();
    return [];
  } catch (error) {
    if (error instanceof NxEvaluationError) {
      return error.diagnostics.filter((diagnostic) => diagnostic.severity === "error");
    }
    throw error;
  }
}

function toProblem(diagnostic) {
  const span = diagnostic.labels[0]?.span;
  const code = diagnostic.code ? `${diagnostic.code}: ` : "";
  return { line: span?.startLine ?? 0, column: span?.startColumn ?? 1, message: code + diagnostic.message };
}

/**
 * Checks every block in the given files and returns the failures as `file:line:column: message`
 * strings, with `file` relative to `base` and the position absolute within the file.
 */
export function checkFiles(host, files, base = process.cwd()) {
  const failures = [];
  let blockCount = 0;
  for (const file of files) {
    const blocks = findNxBlocks(readFileSync(file, "utf8"), { mdx: extname(file) === ".mdx" });
    blockCount += blocks.length;
    for (const block of blocks) {
      for (const problem of checkBlock(host, block)) {
        const line = block.line + problem.line;
        failures.push(`${relative(base, file)}:${line}:${problem.column}: ${problem.message}`);
      }
    }
  }
  return { failures, blockCount };
}

/**
 * Lists the Markdown and MDX files under each path, or the path itself when it is a file.
 */
export function contentFiles(paths) {
  const files = [];
  for (const path of paths) {
    if (statSync(path).isDirectory()) {
      for (const entry of readdirSync(path, { recursive: true })) {
        const full = join(path, entry);
        if ([".md", ".mdx"].includes(extname(entry)) && statSync(full).isFile()) {
          files.push(full);
        }
      }
    } else {
      files.push(path);
    }
  }
  return files.sort();
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const paths = process.argv.slice(2);
  const files = contentFiles(paths.length > 0 ? paths : [join(siteRoot, "src/content")]);
  const host = createNxHost(await loadNxModule());
  const { failures, blockCount } = checkFiles(host, files, siteRoot);
  for (const failure of failures) {
    console.error(failure);
  }
  console.log(`${blockCount} nx blocks in ${files.length} files, ${failures.length} failures`);
  process.exitCode = failures.length > 0 ? 1 : 0;
}
