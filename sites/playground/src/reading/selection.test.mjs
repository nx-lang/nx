import { strict as assert } from "node:assert";
import { test } from "node:test";
import { keyAtOffset, offsetOfKey } from "./selection.ts";

const { createNxHost, loadNxModule } = await import("@nx-lang/sdk-wasm");
const host = createNxHost(await loadNxModule());

function treeOf(source) {
  const uri = "nx://playground/playground.nx";
  const snapshot = host.createLanguageSnapshot([{ uri, source }]);
  try {
    return snapshot.sourceTree(uri);
  } finally {
    snapshot.dispose();
  }
}

const source = 'type Card = { title:string }\nlet root() = <Card title="🎉 Hello" />\n';

test("the cursor selects the smallest node it stands in", () => {
  const tree = treeOf(source);
  const inString = source.indexOf("Hello");
  const key = keyAtOffset(tree, source, inString);
  const node = tree.nodes.find((candidate) => candidate.key === key);
  assert.equal(node.role, "literal");
  const onTag = source.indexOf("<Card") + 2;
  assert.equal(tree.nodes.find((candidate) => candidate.key === keyAtOffset(tree, source, onTag)).role, "element");
});

test("a node's start is the editor offset the cursor goes to", () => {
  const tree = treeOf(source);
  const element = tree.nodes.find((candidate) => candidate.role === "element");
  assert.equal(offsetOfKey(tree, source, element.key), source.indexOf("<Card"));
  const literal = tree.nodes.find((candidate) => candidate.role === "literal");
  assert.equal(offsetOfKey(tree, source, literal.key), source.indexOf('"🎉'));
  assert.equal(offsetOfKey(tree, source, "no such key"), undefined);
});

test("a cursor between declarations selects nothing", () => {
  const tree = treeOf("let a = 1\n\n\nlet b = 2\n");
  assert.equal(keyAtOffset(tree, "let a = 1\n\n\nlet b = 2\n", 11), undefined);
});
