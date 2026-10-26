import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { keyAtRange, keyOfChange, rangeOfKey, spanOfChanges } from "../src/selection.js";
import { treeOf } from "./fixtures.js";

async function tree(source: string) {
  return (await treeOf([{ uri: "nx:///selection.nx", identity: "selection.nx", source }])).tree;
}

const source = 'type Card = { title:string }\nlet root() = <Card title="🎉 Hello" />\nlet next = <Card title="x" />\n';

describe("carrying a selection to and from an editor", () => {
  it("selects the smallest node the cursor stands in", async () => {
    const found = await tree(source);
    const inString = keyAtRange(found, source, source.indexOf("Hello"));
    assert.equal(found.nodes.find((candidate) => candidate.key === inString)!.role, "literal");
    const onTag = keyAtRange(found, source, source.indexOf("<Card") + 2);
    assert.equal(found.nodes.find((candidate) => candidate.key === onTag)!.role, "element");
  });

  it("selects the smallest node that holds a whole selection, less the whitespace around it", async () => {
    const found = await tree(source);
    const start = source.indexOf("<Card");
    const end = source.indexOf("/>", start) + 2;
    const exact = keyAtRange(found, source, start, end);
    assert.equal(found.nodes.find((candidate) => candidate.key === exact)!.role, "element");
    // The whole line, from its start through its newline, holds the declaration.
    const lineStart = source.lastIndexOf("\n", start) + 1;
    const line = keyAtRange(found, source, lineStart, source.indexOf("\n", start) + 1);
    assert.equal(found.nodes.find((candidate) => candidate.key === line)!.role, "declaration");
    // Backwards, as an editor reports a selection made upwards, it is the same.
    assert.equal(keyAtRange(found, source, end, start), exact);
  });

  it("gives a node's range, after a character outside the BMP too", async () => {
    const found = await tree(source);
    const [first, second] = found.nodes.filter((candidate) => candidate.role === "element");
    const firstStart = source.indexOf("<Card");
    assert.deepEqual(rangeOfKey(found, source, first!.key), { start: firstStart, end: source.indexOf("/>", firstStart) + 2 });
    assert.equal(rangeOfKey(found, source, second!.key)!.start, source.lastIndexOf("<Card"));
    const literal = found.nodes.find((candidate) => candidate.role === "literal")!;
    assert.deepEqual(rangeOfKey(found, source, literal.key), { start: source.indexOf('"🎉'), end: source.indexOf('Hello"') + 6 });
    assert.equal(rangeOfKey(found, source, "no such key"), undefined);
  });

  it("round-trips a node's range back to the node", async () => {
    const found = await tree(source);
    for (const node of found.nodes.filter((candidate) => candidate.role === "element")) {
      const range = rangeOfKey(found, source, node.key)!;
      assert.equal(keyAtRange(found, source, range.start, range.end), node.key);
      assert.equal(keyAtRange(found, source, range.start), node.key);
    }
  });

  it("selects nothing between declarations", async () => {
    const text = "let a = 1\n\n\nlet b = 2\n";
    assert.equal(keyAtRange(await tree(text), text, 11), undefined);
  });

  it("finds what a change touched: the property whose value changed, or the element that lost one", async () => {
    const found = await tree(source);
    const at = source.indexOf("Hello");
    const property = keyOfChange(found, source, at, at + 5);
    assert.equal(found.nodes.find((candidate) => candidate.key === property)!.role, "attribute");
    // An attribute taken out leaves its place between the element's tag and its end.
    const removed = 'let root() = <Card  />';
    const gone = await tree(removed);
    const place = removed.indexOf("  />") + 1;
    assert.equal(gone.nodes.find((candidate) => candidate.key === keyOfChange(gone, removed, place))!.role, "element");
  });

  it("spans the changes of one step in the text they left", () => {
    assert.equal(spanOfChanges([]), undefined);
    assert.deepEqual(spanOfChanges([{ rangeOffset: 10, rangeLength: 2, text: "abc" }]), { start: 10, end: 13 });
    // Given in any order, each by the text before them all: the later one moves by the earlier's growth.
    assert.deepEqual(
      spanOfChanges([
        { rangeOffset: 20, rangeLength: 1, text: "x" },
        { rangeOffset: 5, rangeLength: 0, text: "1234" }
      ]),
      { start: 5, end: 25 }
    );
  });
});
