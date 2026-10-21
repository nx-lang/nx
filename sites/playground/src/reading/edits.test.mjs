/**
 * Visual's edits made in the visitor's source, through the worker's session as the playground asks
 * it, against the example the playground's spec names.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { loadNxModule } from "@nx-lang/sdk-wasm";
import { createNxSession } from "../worker/session.ts";
import { createEditQueue } from "./edits.ts";

const module = await loadNxModule();
const session = createNxSession({ module });
after(() => session.dispose());

const uri = "nx://playground/playground.nx";
const example = readFileSync(new URL("../examples/nx/visualizer-declarations.nx", import.meta.url), "utf8");

let id = 0;
const service = {
  sourceEdit: (request) => session.answer({ kind: "language", id: ++id, query: "sourceEdit", request }),
};
const sourceTree = (source) =>
  session.answer({ kind: "language", id: ++id, query: "sourceTree", request: { documents: [{ uri, source }], uri, visualizers: {} } });

/**
 * A document as the editor holds it: an answer's text edits applied together, every range in the
 * text as it was, as one undo step.
 */
function document(text) {
  const steps = [];
  const offset = (source, { line, character }) => {
    let at = 0;
    for (let current = 0; current < line; current += 1) {
      at = source.indexOf("\n", at) + 1;
    }
    return at + character;
  };
  return {
    steps,
    text: () => text,
    set: (next) => {
      text = next;
    },
    apply(edits) {
      steps.push(text);
      const before = text;
      const spans = edits
        .map((edit) => ({ start: offset(before, edit.range.start), end: offset(before, edit.range.end), newText: edit.newText }))
        .sort((a, b) => b.start - a.start);
      for (const { start, end, newText } of spans) {
        text = text.slice(0, start) + newText + text.slice(end);
      }
    },
  };
}

/** The key of the literal a `Drink` with the name `name` sets for `property`. */
async function keyOf(source, name, property) {
  const tree = await sourceTree(source);
  const drink = tree.nodes.findIndex(
    (node, at) =>
      node.role === "element" &&
      tree.nodes.some(
        (attribute, index) =>
          attribute.parent === at &&
          attribute.name === "name" &&
          tree.nodes.some((value) => value.parent === index && value.value === `"${name}"`),
      ),
  );
  const attribute = tree.nodes.findIndex((node) => node.parent === drink && node.name === property);
  return tree.nodes.find((node) => node.parent === attribute)?.key;
}

test("an edit in Visual shows in the source, as one step that undo takes back", async () => {
  const edited = document(example);
  const applied = [];
  const queue = createEditQueue(service, uri, edited, (text) => applied.push(text));
  await queue({ key: await keyOf(example, "Latte", "name"), edit: { kind: "setValue", value: { kind: "string", value: "Flat white" } } });
  assert.ok(edited.text().includes('name="Flat white"'));
  assert.ok(!edited.text().includes('name="Latte"'));
  assert.deepEqual(applied, [edited.text()], "the playground hears of the new text");
  assert.deepEqual(edited.steps, [example], "one undoable step, from the text as it was");
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
    queue({ key: iced, edit: { kind: "setValue", value: { kind: "boolean", value: false } } }),
  ]);
  assert.ok(edited.text().includes('<Drink code="CBR" name="Nitro cold brew" size=medium iced={false} color="#5a3a22" />'), edited.text());
  assert.equal(edited.steps.length, 3);
});

test("a refused edit leaves the source as it was, and says so, so Visual is given its tree again", async () => {
  const edited = document(example);
  let unchanged = 0;
  const queue = createEditQueue(service, uri, edited, () => assert.fail("nothing is applied"), () => (unchanged += 1));
  await queue({ key: "no.such.node", edit: { kind: "setValue", value: { kind: "string", value: "x" } } });
  await queue({ key: await keyOf(example, "Latte", "name"), edit: { kind: "setValue", value: { kind: "number", text: "12px" } } });
  await queue({ key: await keyOf(example, "Latte", "name"), edit: { kind: "setValue", value: { kind: "string", value: "Latte" } } });
  assert.equal(edited.text(), example);
  assert.deepEqual(edited.steps, []);
  assert.equal(unchanged, 3, "two refusals and an edit that changes nothing");
});

test("an answer for text that changed while it was asked is not applied", async () => {
  const edited = document(example);
  const key = await keyOf(example, "Latte", "name");
  const slow = {
    sourceEdit: async (request) => {
      const answer = await service.sourceEdit(request);
      edited.set(`${example}\n// typed in Source meanwhile\n`);
      return answer;
    },
  };
  let unchanged = 0;
  const queue = createEditQueue(slow, uri, edited, () => assert.fail("nothing is applied"), () => (unchanged += 1));
  await queue({ key, edit: { kind: "setValue", value: { kind: "string", value: "Mocha" } } });
  assert.ok(edited.text().includes('name="Latte"'));
  assert.equal(unchanged, 1);
});
