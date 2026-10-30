import "./check-gradient-sampling.mjs";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import {readFileSync,existsSync} from "node:fs";
const read=p=>JSON.parse(readFileSync(new URL(`../${p}`,import.meta.url),"utf8"));
const tasks=read("evaluations/graphics-v1/tasks.json");
const schema=read("evaluations/graphics-v1/result.schema.json");
assert.equal(tasks.profile,"graphics-core-v3");
assert.equal(tasks.version,1);assert.equal(tasks.tasks.length,4);
assert.deepEqual([...tasks.requiredResultFields].sort(),[...schema.required].sort());
assert.deepEqual(tasks.tasks.map(t=>t.id),schema.properties.task.enum);
for(const t of tasks.tasks){assert.ok(t.prompt.length>20);assert.ok(t.revision.length>20)}
const svg=read("crates/nx-graphics-skia/fixtures/svg-v1/expectations.json");
for(const fixture of svg.references){assert.ok(existsSync(new URL(`../crates/nx-graphics-skia/fixtures/svg-v1/${fixture.file}`,import.meta.url)));assert.ok(fixture.assertion.length>10)}
assert.ok(existsSync(new URL("../crates/nx-graphics-skia/fixtures/fonts/LICENSE.txt",import.meta.url)));
console.log("Graphics fixture/task schema checks passed without model calls.");

for (const face of read("crates/nx-graphics-skia/fixtures/fonts/manifest.json")) {
  const bytes = readFileSync(new URL(`../crates/nx-graphics-skia/fixtures/fonts/${face.path}`, import.meta.url));
  assert.equal(createHash("sha256").update(bytes).digest("hex"), face.sha256, face.path);
}
