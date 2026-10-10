/**
 * The reading is lossless: the source tree of every `.nx` file in the repository, rendered with
 * its details strips open, carries the key of every node exactly once.
 */
import { document } from "./dom.js";

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { after, describe, it } from "node:test";

import type { SourceTree } from "@nx-lang/language-protocol";

import { nxHost, repository, treeOf, type Fixture } from "./fixtures.js";

const { renderDocument, renderers } = await import("../src/render.js");
const { indexTree } = await import("../src/tree.js");

after(async () => (await nxHost()).dispose());

/** The keys of `tree` that do not appear exactly once in its rendering, with how often they do. */
function uncovered({ tree, text }: Fixture): string[] {
  const rendered = renderDocument({ document, index: indexTree(tree, text), ghosts: true, copy: false });
  for (const details of rendered.querySelectorAll("details")) {
    details.open = true;
  }
  const counts = new Map<string, number>();
  for (const element of rendered.querySelectorAll<HTMLElement>("[data-key]")) {
    const key = element.dataset["key"]!;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return tree.nodes
    .filter((node) => counts.get(node.key) !== 1)
    .map((node) => `${node.key} (${node.role}, rendered ${counts.get(node.key) ?? 0} times)`);
}

/** Every `.nx` file git tracks, as one snapshot so imports between them resolve. */
async function repositoryTrees(): Promise<{ path: string; fixture: Fixture }[]> {
  const root = fileURLToPath(repository);
  const paths = execFileSync("git", ["ls-files", "*.nx"], { cwd: root, encoding: "utf8" })
    .split("\n")
    .filter((path) => path !== "");
  const documents = paths.map((path) => ({
    uri: `nx:///${path}`,
    identity: path,
    source: readFileSync(new URL(path, repository), "utf8")
  }));
  const snapshot = (await nxHost()).createLanguageSnapshot(documents);
  try {
    return documents.map((document) => ({
      path: document.identity,
      fixture: { text: document.source, tree: snapshot.sourceTree(document.uri) as SourceTree }
    }));
  } finally {
    snapshot.dispose();
  }
}

/** For each file with a node not rendered exactly once, its path and the first such keys. */
function failuresOf(trees: readonly { path: string; fixture: Fixture }[]): string[] {
  const failures: string[] = [];
  for (const { path, fixture } of trees) {
    const missing = uncovered(fixture);
    if (missing.length > 0) {
      failures.push(`${path}: ${missing.slice(0, 5).join(", ")}${missing.length > 5 ? ` and ${missing.length - 5} more` : ""}`);
    }
  }
  return failures;
}

describe("coverage", () => {
  it("renders every node of every .nx file in the repository exactly once", async () => {
    const trees = await repositoryTrees();
    assert.ok(trees.length > 100, "the repository's .nx files are found");
    const failures = failuresOf(trees);
    assert.deepEqual(failures, [], `nodes not rendered exactly once:\n${failures.join("\n")}`);
  });

  it("fails naming a file and a key when a role's renderer is removed", async () => {
    const fixture = await treeOf([
      { uri: "nx:///commented.nx", identity: "commented.nx", source: "// A note.\nlet answer = 42\n" }
    ]);
    const renderer = renderers.comment;
    delete renderers.comment;
    try {
      assert.deepEqual(failuresOf([{ path: "commented.nx", fixture }]), [
        "commented.nx: answer:comment[0] (comment, rendered 0 times)"
      ]);
    } finally {
      if (renderer !== undefined) {
        renderers.comment = renderer;
      }
    }
  });

  it("renders the question-flow program quickly", async () => {
    const path = "specs/ir-conformance/question-flow/main.nx";
    const { fixture } = (await repositoryTrees()).find((entry) => entry.path === path)!;
    const index = indexTree(fixture.tree, fixture.text);
    const started = performance.now();
    renderDocument({ document, index, ghosts: true, copy: false });
    const elapsed = performance.now() - started;
    console.log(`Rendered ${path} (${fixture.tree.nodes.length} nodes) in ${elapsed.toFixed(1)} ms`);
    assert.ok(elapsed < 1000, `rendering took ${elapsed.toFixed(1)} ms`);
  });
});
