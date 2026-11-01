import assert from "node:assert/strict";
import test from "node:test";
import { layeringViolations, workspaceMetadata } from "./check-crate-layering.mjs";

const metadata = workspaceMetadata();
test("the workspace respects the crate boundaries", () => {
  assert.deepEqual(layeringViolations(metadata), []);
});

for (const kind of [null, "dev", "build"]) {
  test(`rejects ${kind ?? "normal"} upward edges, including transitive foundation crates`, () => {
    const graph = structuredClone(metadata);
    graph.packages.find(pkg => pkg.name === "nx-hir").dependencies.push({ name: "nx-api", kind });
    assert.deepEqual(layeringViolations(graph), [`nx-hir -> nx-api (${kind ?? "normal"} dependency)`]);
  });
  for (const [from, to] of [["nx-program", "nx-ir-runtime"], ["nx-ir-runtime", "nx-program"]]) {
    test(`rejects ${from} -> ${to} as a ${kind ?? "normal"} edge even on another target`, () => {
      const graph = structuredClone(metadata);
      graph.packages.find(pkg => pkg.name === from).dependencies.push({ name: to, kind, target: 'cfg(windows)' });
      assert.deepEqual(layeringViolations(graph), [`${from} -> ${to} (${kind ?? "normal"} dependency, target cfg(windows))`]);
    });
  }
}

for (const name of ["nx-interpreter", "nx-codegen", "nx-language-service"]) {
  test(`rejects a foundation dev dependency on ${name}`, () => {
    const graph = structuredClone(metadata);
    graph.packages.find(pkg => pkg.name === "nx-program").dependencies.push({ name, kind: "dev" });
    assert.deepEqual(layeringViolations(graph), [`nx-program -> ${name} (dev dependency)`]);
  });
}
