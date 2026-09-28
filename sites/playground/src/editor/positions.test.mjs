import { strict as assert } from "node:assert";
import { test } from "node:test";
import { findName, offsetOf, positionAt } from "./positions.ts";

test("an NX line and scalar column become a UTF-16 offset", () => {
  const source = "type A = { a:int }\nlet s = \"🎉\" + x\n";
  assert.equal(offsetOf(source, 1, 1), 0);
  assert.equal(offsetOf(source, 2, 1), 19);
  // The emoji is one scalar value and two UTF-16 units, so `+` at scalar column 13 is unit 14.
  assert.equal(source[offsetOf(source, 2, 13)], "+");
});

test("a UTF-16 offset becomes a 0-based line and character", () => {
  const source = "ab\ncd\nef";
  assert.deepEqual(positionAt(source, 0), { line: 0, character: 0 });
  assert.deepEqual(positionAt(source, 4), { line: 1, character: 1 });
  assert.deepEqual(positionAt(source, 8), { line: 2, character: 2 });
});

test("a declaration's name is found as a whole word within its span", () => {
  const source = "type TaskList = {}\ntype Task = { title:string }";
  const start = source.indexOf("type Task =");
  assert.deepEqual(findName(source, "Task", start, source.length), { start: start + 5, end: start + 9 });
  assert.equal(findName(source, "Tas", start, source.length), null);
  assert.deepEqual(findName(source, "title", start, source.length)?.start, source.indexOf("title"));
});

test("a node's declared name is found for records, properties, cases, payload cases and components", async () => {
  const { createNxHost, loadNxModule } = await import("@nx-lang/sdk-wasm");
  const { declaredName } = await import("./positions.ts");
  const source = [
    "type Status = active | retired",
    "type Load =",
    "  | idle",
    "  | failed { message:string }",
    "type Task = { title:string status:Status load:Load }",
    "external component <Button label:string />",
    'let root() = { <Task title="a" status=active load=<Load.failed message="m" /> /> <Button label="b" /> }',
  ].join("\n");
  const host = createNxHost(await loadNxModule());
  const artifact = host.buildProgramArtifact(source);
  const value = artifact.evaluateNx();
  artifact.dispose();
  host.dispose();

  const nameOf = (predicate) => {
    const node = value.nodes.find(predicate);
    assert.ok(node, "the node exists");
    const range = declaredName(node, source);
    return range === null ? null : source.slice(range.start, range.end);
  };
  assert.equal(nameOf((node) => node.role === "record" && node.type === "Task"), "Task");
  assert.equal(nameOf((node) => node.role === "property" && node.name === "title"), "title");
  assert.equal(nameOf((node) => node.role === "case"), "active");
  assert.equal(nameOf((node) => node.role === "record" && node.type === "Load.failed"), "failed");
  assert.equal(nameOf((node) => node.role === "record" && node.type === "Button"), "Button");
  assert.equal(nameOf((node) => node.role === "property" && node.name === "label"), "label");
});
