import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "../dist/src/index.js";

for (const suffix of [".nxir", ".stripped.nxir"]) {
  const root = new URL("../../../specs/ir-conformance/geometry-library/expected/", import.meta.url);
  const geometry = prepareNxIrModule(readFileSync(new URL(`@nx__geometry__geometry.nx${suffix}`, root)));
  const entry = prepareNxIrModule(readFileSync(new URL(`main.nx${suffix}`, root)));
  assert.throws(() => linkNxIrProgram(entry, { resolve: () => undefined }));
  const program = linkNxIrProgram(entry, { resolve: (id) => id === geometry.identity ? geometry : undefined });
  for (const name of ["point", "rect"]) {
    for (const bad of [Infinity, -Infinity, NaN]) {
      assert.throws(() => evaluateFunction(program, name, [bad]),
        (error) => error.diagnostics[0].code === "nx-ir-constraint");
    }
  }
  assert.equal(evaluateFunction(program, "size", [Infinity]).width, Infinity);
  for (const bad of [-Infinity, NaN, -1]) {
    assert.throws(() => evaluateFunction(program, "size", [bad]),
      (error) => error.diagnostics[0].code === "nx-ir-constraint");
  }
  assert.equal(evaluateFunction(program, "root", []).anchor.x, 1000000000000.125);
}
console.log("Geometry preserves precision and rejects non-finite host coordinates in both image forms.");
