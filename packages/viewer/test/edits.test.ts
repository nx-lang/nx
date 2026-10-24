/**
 * `createEditQueue`: the element's edits made in a host's text, through `@nx-lang/sdk-wasm`'s
 * language snapshot, against the playground's visualizer example.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import type { SourceEditRequest, SourceTree, TextEdit } from "@nx-lang/language-protocol";

import { createEditQueue } from "../src/edits.js";
import { nxHost, repository } from "./fixtures.js";

const uri = "nx://viewer/edits.nx";
const example = readFileSync(new URL("sites/playground/src/examples/nx/visualizer-declarations.nx", repository), "utf8");

const service = {
  async sourceEdit(request: SourceEditRequest) {
    const snapshot = (await nxHost()).createLanguageSnapshot(request.documents);
    try {
      return snapshot.sourceEdit(request.uri, request.key, request.edit);
    } finally {
      snapshot.dispose();
    }
  }
};

async function sourceTree(source: string): Promise<SourceTree> {
  const snapshot = (await nxHost()).createLanguageSnapshot([{ uri, source }]);
  try {
    return snapshot.sourceTree(uri);
  } finally {
    snapshot.dispose();
  }
}

/** A document as an editor holds it: an answer's text edits applied together, as one undo step. */
function document(initial: string) {
  let text = initial;
  const steps: string[] = [];
  const offset = (source: string, { line, character }: { line: number; character: number }) => {
    let at = 0;
    for (let current = 0; current < line; current += 1) {
      at = source.indexOf("\n", at) + 1;
    }
    return at + character;
  };
  return {
    steps,
    text: () => text,
    set(next: string) {
      text = next;
    },
    apply(edits: readonly TextEdit[]) {
      steps.push(text);
      const before = text;
      const spans = edits
        .map((edit) => ({ start: offset(before, edit.range.start), end: offset(before, edit.range.end), newText: edit.newText }))
        .sort((a, b) => b.start - a.start);
      for (const { start, end, newText } of spans) {
        text = text.slice(0, start) + newText + text.slice(end);
      }
    }
  };
}

/** The key of the literal a `Drink` named `name` sets for `property`. */
async function keyOf(source: string, name: string, property: string): Promise<string> {
  const tree = await sourceTree(source);
  const drink = tree.nodes.findIndex(
    (node, at) =>
      node.role === "element" &&
      tree.nodes.some(
        (attribute, index) =>
          attribute.parent === at &&
          attribute.name === "name" &&
          tree.nodes.some((value) => value.parent === index && value.value === `"${name}"`)
      )
  );
  const attribute = tree.nodes.findIndex((node) => node.parent === drink && node.name === property);
  const key = tree.nodes.find((node) => node.parent === attribute)?.key;
  assert.ok(key !== undefined, `${name} sets ${property}`);
  return key;
}

test("an edit is applied to the text as one step", async () => {
  const edited = document(example);
  const applied: string[] = [];
  const queue = createEditQueue(service, uri, edited, (text) => applied.push(text));
  await queue({ key: await keyOf(example, "Latte", "name"), edit: { kind: "setValue", value: { kind: "string", value: "Flat white" } } });
  assert.ok(edited.text().includes('name="Flat white"'));
  assert.ok(!edited.text().includes('name="Latte"'));
  assert.deepEqual(applied, [edited.text()]);
  assert.deepEqual(edited.steps, [example]);
});

test("edits apply in the order they fire, each against the text the one before left", async () => {
  const edited = document(example);
  const queue = createEditQueue(service, uri, edited, () => undefined);
  const name = await keyOf(example, "Cold brew", "name");
  const size = await keyOf(example, "Cold brew", "size");
  const iced = await keyOf(example, "Cold brew", "iced");
  await Promise.all([
    queue({ key: name, edit: { kind: "setValue", value: { kind: "string", value: "Nitro cold brew" } } }),
    queue({ key: size, edit: { kind: "setValue", value: { kind: "case", name: "medium" } } }),
    queue({ key: iced, edit: { kind: "setValue", value: { kind: "boolean", value: false } } })
  ]);
  assert.ok(
    edited.text().includes('<Drink code="CBR" name="Nitro cold brew" size=medium iced={false} color="#5a3a22" />'),
    edited.text()
  );
  assert.equal(edited.steps.length, 3);
});

test("a refused edit, or one that changes nothing, leaves the text and says so", async () => {
  const edited = document(example);
  let unchanged = 0;
  const queue = createEditQueue(service, uri, edited, () => assert.fail("nothing is applied"), () => (unchanged += 1));
  const latte = await keyOf(example, "Latte", "name");
  await queue({ key: "no.such.node", edit: { kind: "setValue", value: { kind: "string", value: "x" } } });
  await queue({ key: latte, edit: { kind: "setValue", value: { kind: "number", text: "12px" } } });
  await queue({ key: latte, edit: { kind: "setValue", value: { kind: "string", value: "Latte" } } });
  assert.equal(edited.text(), example);
  assert.deepEqual(edited.steps, []);
  assert.equal(unchanged, 3);
});

test("an answer for text that changed while it was asked is not applied", async () => {
  const edited = document(example);
  const key = await keyOf(example, "Latte", "name");
  const slow = {
    async sourceEdit(request: SourceEditRequest) {
      const answer = await service.sourceEdit(request);
      edited.set(`${example}\n// typed in the editor meanwhile\n`);
      return answer;
    }
  };
  let unchanged = 0;
  const queue = createEditQueue(slow, uri, edited, () => assert.fail("nothing is applied"), () => (unchanged += 1));
  await queue({ key, edit: { kind: "setValue", value: { kind: "string", value: "Mocha" } } });
  assert.ok(edited.text().includes('name="Latte"'));
  assert.equal(unchanged, 1);
});

test("a failed edit does not hold up the ones after it", async () => {
  const edited = document(example);
  let calls = 0;
  const flaky = {
    async sourceEdit(request: SourceEditRequest) {
      calls += 1;
      if (calls === 1) {
        throw new Error("the service is gone");
      }
      return service.sourceEdit(request);
    }
  };
  let unchanged = 0;
  const queue = createEditQueue(flaky, uri, edited, () => undefined, () => (unchanged += 1));
  const key = await keyOf(example, "Latte", "name");
  await queue({ key, edit: { kind: "setValue", value: { kind: "string", value: "Mocha" } } }).catch(() => undefined);
  await queue({ key, edit: { kind: "setValue", value: { kind: "string", value: "Mocha" } } });
  assert.equal(unchanged, 1);
  assert.ok(edited.text().includes('name="Mocha"'));
});

test("a step taken after an edit is fired waits for it, as an undo pressed while it is asked", async () => {
  const edited = document(example);
  const queue = createEditQueue(service, uri, edited, () => undefined);
  const key = await keyOf(example, "Latte", "name");
  const order: string[] = [];
  const edit = queue({ key, edit: { kind: "setValue", value: { kind: "string", value: "Mocha" } } }).then(() => order.push("edit"));
  const undo = queue.after(() => {
    order.push("undo");
    assert.ok(edited.text().includes('name="Mocha"'));
  });
  await Promise.all([edit, undo]);
  assert.deepEqual(order, ["edit", "undo"]);
});
