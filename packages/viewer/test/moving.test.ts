import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it } from "node:test";

import type { SourceEdit } from "@nx-lang/language-protocol";

import { nxHost, treeOf, type Fixture } from "./fixtures.js";
import { canDelete, canMove, itemListOf, ItemMap, mappedKey, movedPosition } from "../src/move.js";
import { indexTree } from "../src/tree.js";
import type { NxEditDetail, NxFoldDetail, NxViewerElement } from "../src/index.js";

const { NX_EDIT_EVENT, NX_FOLD_EVENT, NX_OPEN_EVENT, NX_SELECT_EVENT } = await import("../src/index.js");

const window = document.defaultView!;
const created: NxViewerElement[] = [];

after(async () => {
  for (const element of created) {
    element.remove();
  }
  (await nxHost()).dispose();
});

const SOURCE = [
  "type Step = { id:string label?:string }",
  "type Choice = { value:string }",
  "type QuestionFlow = { content steps:Step+ }",
  "type Select = { choices:Choice* }",
  "type Box = { content items:Step* }",
  "let wide = true",
  "let flow = <QuestionFlow>",
  '  <Step id="a" label="First" />',
  '  <Step id="b" label="Second" />',
  '  <Step id="c" />',
  '  <Step id="d" />',
  "</QuestionFlow>",
  "let single = <QuestionFlow>",
  '  <Step id="only" />',
  "</QuestionFlow>",
  "let branch = <QuestionFlow>",
  '  <Step id="a" />',
  "  if wide {",
  '    <Step id="b" />',
  "  }",
  "</QuestionFlow>",
  'let pick = <Select choices={ <Choice value="x" /> <Choice value="y" /> <Choice value="z" /> } />',
  'let pair = <Select choices={ <Choice value="p" /> <Choice value="q" /> } />',
  'let xs = { <Choice value="m" /> <Choice value="n" /> <Choice value="o" /> }',
  "let lone = <Box>",
  '  <Step id="z" />',
  "</Box>",
  ""
].join("\n");

const URI = "nx:///moving.nx";

function sourceOf(text: string): Promise<Fixture> {
  return treeOf([{ uri: URI, identity: "moving.nx", source: text }]);
}

/** `text` once the language service has made `edit` at `key`. */
async function edited(text: string, key: string, edit: SourceEdit): Promise<string> {
  const snapshot = (await nxHost()).createLanguageSnapshot([{ uri: URI, identity: "moving.nx", source: text }]);
  try {
    const answer = snapshot.sourceEdit(URI, key, edit);
    assert.equal(answer.refused, undefined, `${key} was refused`);
    let result = text;
    for (const change of [...answer.edits].reverse()) {
      result = result.slice(0, change.range.startByte) + change.newText + result.slice(change.range.endByte);
    }
    return result;
  } finally {
    snapshot.dispose();
  }
}

/**
 * The element showing `fixture`, everything unfolded, editable, with an icon for every element unless
 * `icons` is false.
 */
function show({ tree, text }: Fixture, folded: readonly string[] = [], icons = true): NxViewerElement {
  const element = document.createElement("nx-viewer");
  document.body.append(element);
  element.folded = folded;
  element.editable = true;
  if (icons) {
    element.icons = () => document.createElementNS("http://www.w3.org/2000/svg", "svg");
  }
  element.text = text;
  element.tree = tree;
  created.push(element);
  return element;
}

function root(element: NxViewerElement): ShadowRoot {
  return element.shadowRoot!;
}

function rendered(element: NxViewerElement, key: string): HTMLElement {
  const found = [...root(element).querySelectorAll<HTMLElement>("[data-key]")].find((candidate) => candidate.dataset["key"] === key);
  assert.ok(found, `the node ${key} is rendered`);
  return found;
}

function body(element: NxViewerElement): HTMLElement {
  return root(element).querySelector<HTMLElement>(".body")!;
}

function press(target: Element, key: string, init: KeyboardEventInit = {}): void {
  target.dispatchEvent(new window.KeyboardEvent("keydown", { key, bubbles: true, composed: true, cancelable: true, ...init }));
}

function edits(element: NxViewerElement): NxEditDetail[] {
  const fired: NxEditDetail[] = [];
  element.addEventListener(NX_EDIT_EVENT, (event) => fired.push((event as CustomEvent<NxEditDetail>).detail));
  return fired;
}

/** A pointer event as a browser fires one; jsdom has no `PointerEvent`. */
function pointer(target: Element, type: string, x: number, y: number): void {
  const event = new window.MouseEvent(type, { bubbles: true, composed: true, cancelable: true, clientX: x, clientY: y, button: 0 });
  Object.defineProperty(event, "pointerId", { value: 1 });
  target.dispatchEvent(event);
}

/** Lays each item's line out as jsdom does not: 18 pixels each, from the top. */
function layOut(element: NxViewerElement, keys: readonly string[]): void {
  keys.forEach((key, position) => {
    const row = rendered(element, key).closest(".sheet-item")!;
    const box = { top: position * 18, bottom: position * 18 + 18, left: 0, right: 400, width: 400, height: 18, x: 0, y: position * 18 };
    row.getBoundingClientRect = () => ({ ...box, toJSON: () => box }) as DOMRect;
  });
  body(element).getBoundingClientRect = () => {
    const box = { top: 0, bottom: 400, left: 0, right: 400, width: 400, height: 400, x: 0, y: 0 };
    return { ...box, toJSON: () => box } as DOMRect;
  };
}

const steps = (count: number): string[] => Array.from({ length: count }, (_, position) => `flow.value.steps[${position}]`);

describe("which items move", () => {
  it("are elements of element content and of sequences, outside branches, in lists that hold no text", async () => {
    const { tree, text } = await sourceOf(SOURCE);
    const index = indexTree(tree, text);
    const at = (key: string): number => {
      const found = index.byKey.get(key);
      assert.notEqual(found, undefined, key);
      return found!;
    };
    assert.equal(itemListOf(index, at("flow.value.steps[2]"))?.position, 2);
    assert.equal(canMove(index, at("flow.value.steps[0]")), true);
    assert.equal(canMove(index, at("pick.value.choices.value[1]")), true, "a list on one line");
    assert.equal(canMove(index, at("single.value.steps[0]")), false, "the only item");
    assert.equal(canDelete(index, at("single.value.steps[0]")), false, "the only item of a + list");
    assert.equal(canMove(index, at("lone.value.items[0]")), false);
    assert.equal(canDelete(index, at("lone.value.items[0]")), true, "the only item of a * list");
    assert.equal(itemListOf(index, at("branch.value.steps[1].then[0]")), undefined, "an item an if makes");
    assert.equal(itemListOf(index, at("branch.value.steps[1]")), undefined, "the if itself");
    assert.equal(canMove(index, at("branch.value.steps[0]")), true, "an item beside an if");
    assert.equal(itemListOf(index, at("flow.value")), undefined, "not an item");
    const text_ = await sourceOf("type T = { content items:string* }\ntype I = {}\nlet t = <T>Hi <I /> <I /></T>\n");
    const withText = indexTree(text_.tree, text_.text);
    assert.equal(itemListOf(withText, withText.byKey.get("t.value.items[1]")!), undefined, "a list that holds text");
  });

  it("maps keys as items move and go", () => {
    assert.deepEqual([0, 1, 2, 3].map((position) => movedPosition(position, 0, 2)), [2, 0, 1, 3]);
    assert.deepEqual([0, 1, 2, 3].map((position) => movedPosition(position, 3, 1)), [0, 2, 3, 1]);
    const map = new ItemMap(4);
    map.move(0, 2);
    assert.equal(mappedKey("l[1].id", "l", map, false), "l[0].id");
    assert.equal(mappedKey("l[0]", "l", map, false), "l[2]");
    assert.equal(mappedKey("l[3]:comment[0]", "l", map, false), "l[3]:comment[0]");
    assert.equal(mappedKey("other[0]", "l", map, false), "other[0]");
    assert.equal(mappedKey("l[10]x", "l", map, false), "l[10]x", "not a key of an item");
    map.delete(1);
    assert.equal(map.count, 3);
    assert.equal(mappedKey("l[2]", "l", map, false), undefined, "deleted");
    assert.equal(mappedKey("l[0]", "l", map, false), "l[1]");
    const pair = new ItemMap(2);
    pair.delete(0);
    assert.equal(mappedKey("s[1].id", "s", pair, true), "s.id", "a sequence of one reads as its item");
  });
});

describe("moving and deleting in the reading", () => {
  it("moves the selected item with Alt+Up and Alt+Down, and nothing past the ends", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    element.selection = "flow.value.steps[1]";
    press(body(element), "ArrowDown", { altKey: true });
    assert.deepEqual(fired, [{ key: "flow.value.steps[1]", edit: { kind: "move", to: 2 } }]);
    // Until the newer tree comes, the next press moves the same item on.
    press(body(element), "ArrowDown", { altKey: true });
    assert.deepEqual(fired[1], { key: "flow.value.steps[2]", edit: { kind: "move", to: 3 } });
    press(body(element), "ArrowDown", { altKey: true });
    assert.equal(fired.length, 2, "nothing past the end");

    const other = show(await sourceOf(SOURCE));
    const firedOther = edits(other);
    other.selection = "flow.value.steps[0]";
    press(body(other), "ArrowUp", { altKey: true });
    assert.equal(firedOther.length, 0, "nothing past the start");
    other.selection = "pick.value.choices.value[1]";
    press(body(other), "ArrowUp", { altKey: true });
    assert.deepEqual(firedOther, [{ key: "pick.value.choices.value[1]", edit: { kind: "move", to: 0 } }], "a list on one line");
  });

  it("selects the moved item in the newer tree, keeps folds on their items and says where it went", async () => {
    const before = await sourceOf(SOURCE);
    const element = show(before, ["flow.value.steps[1]"]);
    const selections: string[] = [];
    const folds: (readonly string[])[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => selections.push((event as CustomEvent<{ key: string }>).detail.key));
    element.addEventListener(NX_FOLD_EVENT, (event) => folds.push((event as CustomEvent<NxFoldDetail>).detail.folded));
    const fired = edits(element);
    element.selection = "flow.value.steps[0]";
    press(body(element), "ArrowDown", { altKey: true });
    press(body(element), "ArrowDown", { altKey: true });
    assert.equal(fired.length, 2);
    let text = SOURCE;
    for (const { key, edit } of fired) {
      text = await edited(text, key, edit);
    }
    const after_ = await sourceOf(text);
    element.tree = undefined;
    element.text = after_.text;
    element.tree = after_.tree;
    assert.equal(element.selection, "flow.value.steps[2]");
    assert.equal(selections.at(-1), "flow.value.steps[2]");
    assert.match(rendered(element, "flow.value.steps[2]").textContent ?? "", /First/, "the moved step");
    assert.deepEqual(element.folded, ["flow.value.steps[0]"], "the fold stays on the second step, now first");
    assert.deepEqual(folds.at(-1), ["flow.value.steps[0]"]);
    assert.equal(root(element).querySelector('[role="status"]')!.textContent, "Moved to 3 of 4");
  });

  it("leaves the selection and folds when the newer tree is the same text, as after a refusal", async () => {
    const before = await sourceOf(SOURCE);
    const element = show(before, ["flow.value.steps[1]"]);
    element.selection = "flow.value.steps[0]";
    press(body(element), "ArrowDown", { altKey: true });
    element.tree = before.tree;
    assert.equal(element.selection, "flow.value.steps[0]");
    assert.deepEqual(element.folded, ["flow.value.steps[1]"]);
  });

  it("deletes the selected item with Delete or Cmd+Backspace, not Backspace, and not the only item of a + list", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    element.selection = "single.value.steps[0]";
    press(body(element), "Delete");
    assert.equal(fired.length, 0, "the only step");
    element.selection = "lone.value.items[0]";
    press(body(element), "Backspace");
    assert.equal(fired.length, 0, "Backspace alone");
    press(body(element), "Backspace", { metaKey: true });
    assert.deepEqual(fired, [{ key: "lone.value.items[0]", edit: { kind: "remove" } }]);
    assert.ok(rendered(element, "lone.value.items[0]").classList.contains("pending"), "muted until the newer tree");

    const other = show(await sourceOf(SOURCE));
    const firedOther = edits(other);
    other.selection = "pick.value.choices.value[2]";
    press(body(other), "Delete");
    assert.deepEqual(firedOther, [{ key: "pick.value.choices.value[2]", edit: { kind: "remove" } }]);
    const text = await edited(SOURCE, firedOther[0]!.key, firedOther[0]!.edit);
    const after_ = await sourceOf(text);
    other.tree = undefined;
    other.text = after_.text;
    other.tree = after_.tree;
    assert.equal(other.selection, "pick.value.choices.value[1]", "the choice before it, the last deleted");
    assert.equal(root(other).querySelector('[role="status"]')!.textContent, "Deleted");
  });

  it("selects the item that took a deleted one's place, or what holds the list", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    element.selection = "flow.value.steps[1]";
    press(body(element), "Delete");
    let text = await edited(SOURCE, fired[0]!.key, fired[0]!.edit);
    let next = await sourceOf(text);
    element.tree = undefined;
    element.text = next.text;
    element.tree = next.tree;
    assert.equal(element.selection, "flow.value.steps[1]");
    assert.match(rendered(element, "flow.value.steps[1]").textContent ?? "", /"c"|c/);

    element.selection = "lone.value.items[0]";
    press(body(element), "Delete");
    text = await edited(text, fired[1]!.key, fired[1]!.edit);
    next = await sourceOf(text);
    element.tree = undefined;
    element.text = next.text;
    element.tree = next.tree;
    assert.equal(element.selection, "lone.value", "the element that held it");
  });

  it("gives a selected item a menu of its moves, Delete and Show in Source", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    const opened: string[] = [];
    element.addEventListener(NX_OPEN_EVENT, (event) => opened.push((event as CustomEvent<{ key: string }>).detail.key));
    element.selection = "flow.value.steps[0]";
    const mark = [...root(element).querySelectorAll<HTMLElement>(".row-menu")].find((candidate) => candidate.dataset["menuKey"] === "flow.value.steps[0]");
    assert.ok(mark, "the first step has a menu mark");
    mark.click();
    const items = (): string[] => [...root(element).querySelectorAll<HTMLElement>(".menu-item .menu-label")].map((label) => label.textContent ?? "");
    assert.deepEqual(items(), ["Move down", "Delete", "Show in Source"]);
    const down = [...root(element).querySelectorAll<HTMLElement>(".menu-item")].find((item) => item.dataset["action"] === "down")!;
    down.click();
    assert.deepEqual(fired, [{ key: "flow.value.steps[0]", edit: { kind: "move", to: 1 } }]);

    const other = show(await sourceOf(SOURCE));
    other.addEventListener(NX_OPEN_EVENT, (event) => opened.push((event as CustomEvent<{ key: string }>).detail.key));
    other.selection = "flow.value.steps[3]";
    press(body(other), "F10", { shiftKey: true });
    assert.deepEqual(
      [...root(other).querySelectorAll<HTMLElement>(".menu-item .menu-label")].map((label) => label.textContent),
      ["Move up", "Delete", "Show in Source"]
    );
    [...root(other).querySelectorAll<HTMLElement>(".menu-item")].find((item) => item.dataset["action"] === "open")!.click();
    assert.deepEqual(opened, ["flow.value.steps[3]"]);
  });

  it("marks the icon and the name of an item that moves as its handle, and nothing for one that does not", async () => {
    const element = show(await sourceOf(SOURCE));
    const handles = (key: string): HTMLElement[] =>
      [...root(element).querySelectorAll<HTMLElement>(".drag-handle")].filter((handle) => handle.dataset["dragKey"] === key);
    assert.ok(handles("flow.value.steps[0]").some((handle) => handle.classList.contains("element-icon")), "the icon");
    assert.equal(handles("single.value.steps[0]").length, 0, "the only step");
    assert.equal(handles("branch.value.steps[1].then[0]").length, 0, "inside an if");
    assert.equal(handles("pick.value.choices.value[0]").length, 0, "a list drawn on one line");
  });

  it("drags an item by its handle to the gap the pointer is nearest", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    layOut(element, steps(4));
    const handle = root(element).querySelector<HTMLElement>('.drag-handle[data-drag-key="flow.value.steps[0]"]')!;
    pointer(handle, "pointerdown", 10, 9);
    pointer(handle, "pointermove", 10, 12);
    assert.equal(root(element).querySelector(".drag-ghost"), null, "not a drag before 4 pixels");
    pointer(handle, "pointermove", 10, 40);
    assert.ok(root(element).querySelector(".drag-ghost"), "a copy follows the pointer");
    assert.ok(rendered(element, "flow.value.steps[0]").classList.contains("drag-source"), "the item is muted");
    // Between the third and the fourth step.
    pointer(handle, "pointermove", 10, 53);
    const line = root(element).querySelector<HTMLElement>(".drop-line")!;
    assert.equal(line.hidden, false);
    assert.equal(line.style.top, "53px");
    pointer(handle, "pointerup", 10, 53);
    assert.deepEqual(fired, [{ key: "flow.value.steps[0]", edit: { kind: "move", to: 2 } }]);
    assert.equal(root(element).querySelector(".drag-ghost"), null, "the copy is gone");
  });

  it("cancels a drag on Escape, and on a drop outside the list or where the item is", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    layOut(element, steps(4));
    const handle = root(element).querySelector<HTMLElement>('.drag-handle[data-drag-key="flow.value.steps[1]"]')!;
    pointer(handle, "pointerdown", 10, 27);
    pointer(handle, "pointermove", 10, 60);
    press(body(element), "Escape");
    assert.equal(root(element).querySelector(".drag-ghost"), null);
    pointer(handle, "pointermove", 10, 70);
    pointer(handle, "pointerup", 10, 70);
    pointer(handle, "pointerdown", 10, 27);
    pointer(handle, "pointermove", 10, 300);
    pointer(handle, "pointerup", 10, 300);
    pointer(handle, "pointerdown", 10, 27);
    pointer(handle, "pointermove", 10, 34);
    pointer(handle, "pointerup", 10, 34);
    assert.equal(fired.length, 0);
  });
  it("selects an item clicked on its handle, starting no drag, and its card says it drags", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    layOut(element, steps(4));
    const icon = root(element).querySelector<HTMLElement>('.element-icon.drag-handle[data-drag-key="flow.value.steps[1]"]')!;
    assert.ok(icon, "the icon is a handle");
    icon.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    t.mock.timers.tick(500);
    const card = root(element).querySelector<HTMLElement>(".card")!;
    assert.equal(card.hidden, false);
    assert.equal(card.lastElementChild?.textContent, "Drag to reorder");
    pointer(icon, "pointerdown", 10, 27);
    pointer(icon, "pointermove", 11, 29);
    pointer(icon, "pointerup", 11, 29);
    icon.dispatchEvent(new window.MouseEvent("click", { bubbles: true, composed: true, cancelable: true, clientX: 11, clientY: 29 }));
    assert.equal(element.selection, "flow.value.steps[1]");
    assert.equal(root(element).querySelector(".drag-ghost"), null);
    assert.equal(fired.length, 0);
  });

  it("drags by the name alone where the host draws no icons", async () => {
    const element = show(await sourceOf(SOURCE), [], false);
    const handles = [...root(element).querySelectorAll<HTMLElement>('.drag-handle[data-drag-key="flow.value.steps[0]"]')];
    assert.deepEqual(
      handles.map((handle) => handle.classList.contains("line-kind")),
      [true]
    );
  });

  it("starts no drag and moves no other list while a move waits for the host's tree", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    layOut(element, steps(4));
    element.selection = "flow.value.steps[0]";
    press(body(element), "ArrowDown", { altKey: true });
    assert.equal(fired.length, 1);
    const handle = root(element).querySelector<HTMLElement>('.drag-handle[data-drag-key="flow.value.steps[2]"]')!;
    pointer(handle, "pointerdown", 10, 45);
    pointer(handle, "pointermove", 10, 5);
    assert.equal(root(element).querySelector(".drag-ghost"), null, "no drag");
    pointer(handle, "pointerup", 10, 5);
    element.selection = "pick.value.choices.value[0]";
    press(body(element), "ArrowDown", { altKey: true });
    press(body(element), "Delete");
    assert.equal(fired.length, 1, "nothing for another list");
  });

  it("deletes once from a sequence of two before the newer tree comes, which leaves no list", async () => {
    const element = show(await sourceOf(SOURCE));
    const fired = edits(element);
    element.selection = "pair.value.choices.value[0]";
    press(body(element), "Delete");
    press(body(element), "Delete");
    assert.deepEqual(fired, [{ key: "pair.value.choices.value[0]", edit: { kind: "remove" } }]);
  });

  it("hangs the menu mark of an item sharing its line at the end of its row, shown while selected", async () => {
    const element = show(await sourceOf(SOURCE));
    const mark = [...root(element).querySelectorAll<HTMLElement>(".row-menu")].find(
      (candidate) => candidate.dataset["menuKey"] === "pick.value.choices.value[1]"
    )!;
    assert.ok(mark.classList.contains("shared"));
    assert.equal(mark.parentElement!.parentElement, rendered(element, "pick.value"), "at the end of the line");
    assert.equal(rendered(element, "pick.value.choices.value[1]").querySelector(".row-menu"), null, "not after the item");
    element.selection = "pick.value.choices.value[1]";
    assert.ok(mark.classList.contains("selected"));
    element.selection = "pick.value.choices.value[0]";
    assert.ok(!mark.classList.contains("selected"));
  });

  it("hangs the menu mark of an item of a let's one-line sequence after the sequence", async () => {
    const element = show(await sourceOf(SOURCE));
    const mark = [...root(element).querySelectorAll<HTMLElement>(".row-menu")].find((candidate) => candidate.dataset["menuKey"] === "xs.value[0]")!;
    assert.ok(mark.classList.contains("shared"));
    assert.equal(mark.parentElement!.parentElement, rendered(element, "xs.value"), "after the sequence");
    for (let after = mark.parentElement!.nextElementSibling; after !== null; after = after.nextElementSibling) {
      assert.ok(after.matches(".item-menu"), "past its last part, among the other items' marks");
    }
  });
});
