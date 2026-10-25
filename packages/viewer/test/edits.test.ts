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
      text = applied(text, edits);
    }
  };

  function applied(before: string, edits: readonly TextEdit[]): string {
    let after = before;
    const spans = edits
      .map((edit) => ({ start: offset(before, edit.range.start), end: offset(before, edit.range.end), newText: edit.newText }))
      .sort((a, b) => b.start - a.start);
    for (const { start, end, newText } of spans) {
      after = after.slice(0, start) + newText + after.slice(end);
    }
    return after;
  }
}

/** A document that also previews edits outside its history, as Monaco's `model.applyEdits` does. */
function previewing(initial: string) {
  const base = document(initial);
  const previews: string[] = [];
  return {
    ...base,
    previews,
    preview(edits: readonly TextEdit[]) {
      // Applied as a step would be, then the step forgotten: the history never sees it.
      base.apply(edits);
      base.steps.pop();
      previews.push(base.text());
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

const shots = (value: string) => ({ kind: "setValue" as const, value: { kind: "string" as const, value } });

test("live edits preview each value, and their commit is one step of the history", async () => {
  const edited = previewing(example);
  const applied: string[] = [];
  const queue = createEditQueue(service, uri, edited, (text) => applied.push(text));
  const key = await keyOf(example, "Latte", "name");
  await queue({ key, edit: shots("F"), live: true });
  await queue({ key, edit: shots("Fl"), live: true });
  assert.ok(edited.text().includes('name="Fl"'));
  assert.deepEqual(edited.steps, [], "previews are outside the history");
  await queue({ key, edit: shots("Flat white") });
  assert.ok(edited.text().includes('name="Flat white"'));
  assert.deepEqual(edited.steps, [example], "one step, from the text before the change");
  assert.equal(applied.at(-1), edited.text());
});

test("a change taken back leaves the text and the history as they were", async () => {
  const edited = previewing(example);
  const applied: string[] = [];
  const queue = createEditQueue(service, uri, edited, (text) => applied.push(text));
  const key = await keyOf(example, "Latte", "name");
  await queue({ key, edit: shots("Mocha"), live: true });
  await queue({ key, edit: shots("Latte") });
  assert.equal(edited.text(), example);
  assert.deepEqual(edited.steps, []);
  assert.equal(applied.at(-1), example, "the host reads the text the preview went back to");
});

test("a live edit superseded before its turn is skipped", async () => {
  const edited = previewing(example);
  let asked = 0;
  const counting = {
    sourceEdit(request: SourceEditRequest) {
      asked += 1;
      return service.sourceEdit(request);
    }
  };
  const queue = createEditQueue(counting, uri, edited, () => undefined);
  const key = await keyOf(example, "Latte", "name");
  await Promise.all([
    queue({ key, edit: shots("A"), live: true }),
    queue({ key, edit: shots("B"), live: true }),
    queue({ key, edit: shots("C"), live: true })
  ]);
  assert.equal(asked, 1, "only the last is asked");
  assert.ok(edited.text().includes('name="C"'));
});

test("another value's edit, or a step after the edits, records a preview still shown as one step", async () => {
  const edited = previewing(example);
  const queue = createEditQueue(service, uri, edited, () => undefined);
  await queue({ key: await keyOf(example, "Latte", "name"), edit: shots("Mocha"), live: true });
  await queue({ key: await keyOf(example, "Cold brew", "name"), edit: shots("Nitro") });
  assert.equal(edited.steps.length, 2, "the preview as a step, then the edit");
  assert.ok(edited.text().includes('name="Mocha"') && edited.text().includes('name="Nitro"'));
  const second = previewing(example);
  const queued = createEditQueue(service, uri, second, () => undefined);
  await queued({ key: await keyOf(example, "Latte", "name"), edit: shots("Mocha"), live: true });
  let seen = "";
  await queued.after(() => (seen = second.text()));
  assert.ok(seen.includes('name="Mocha"'));
  assert.deepEqual(second.steps, [example], "recorded before the step runs");
});

test("a document that cannot preview ignores live edits", async () => {
  const edited = document(example);
  const queue = createEditQueue(service, uri, edited, () => assert.fail("nothing is applied"));
  await queue({ key: await keyOf(example, "Latte", "name"), edit: shots("Mocha"), live: true });
  assert.equal(edited.text(), example);
});


test("the commit of a change a step after the edits recorded changes nothing", async () => {
  const edited = previewing(example);
  const unchanged: number[] = [];
  const queue = createEditQueue(service, uri, edited, () => undefined, () => unchanged.push(1));
  const key = await keyOf(example, "Latte", "name");
  await queue({ key, edit: shots("Mocha"), live: true });
  // An undo pressed before the viewer commits: the preview is recorded, then undone.
  await queue.after(() => edited.set(edited.steps.pop()!));
  assert.equal(edited.text(), example);
  await queue({ key, edit: shots("Mocha") });
  assert.equal(edited.text(), example, "the undone change does not come back");
  assert.deepEqual(edited.steps, []);
  assert.equal(unchanged.length, 1);
});

test("a commit keeps the previewed value in the text while it is asked", async () => {
  const edited = previewing(example);
  const key = await keyOf(example, "Latte", "name");
  const seen: string[] = [];
  const watching = {
    sourceEdit(request: SourceEditRequest) {
      seen.push(edited.text());
      return service.sourceEdit(request);
    }
  };
  const queue = createEditQueue(watching, uri, edited, () => undefined);
  await queue({ key, edit: shots("Mocha"), live: true });
  await queue({ key, edit: shots("Mocha!") });
  assert.ok(seen[1]!.includes('name="Mocha"'), "not the text before the change");
  assert.ok(edited.text().includes('name="Mocha!"'));
  assert.deepEqual(edited.steps, [example]);
});

test("a preview never splits a character outside the BMP", async () => {
  const edited = previewing(example);
  const ranges: TextEdit[] = [];
  const queue = createEditQueue(
    service,
    uri,
    {
      ...edited,
      preview(edits: readonly TextEdit[]) {
        ranges.push(...edits);
        edited.preview(edits);
      }
    },
    () => undefined
  );
  const key = await keyOf(example, "Latte", "name");
  await queue({ key, edit: shots("Hi 😀"), live: true });
  await queue({ key, edit: shots("Hi 😁"), live: true });
  assert.ok(edited.text().includes('name="Hi 😁"'));
  const last = ranges.at(-1)!;
  assert.equal(last.newText, "😁", "the whole character is replaced");
  assert.equal(last.range.endByte - last.range.startByte, 4);
});
