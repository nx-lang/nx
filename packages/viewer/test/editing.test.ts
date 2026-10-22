import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it, type TestContext } from "node:test";

import { fixture, nxHost, type Fixture } from "./fixtures.js";
import { editTargets, isRemovable } from "../src/edit.js";
import { offsetAtPoint } from "../src/editors.js";
import { indexTree } from "../src/tree.js";
import type { NxEditDetail, NxOpenDetail, NxViewerElement } from "../src/index.js";

const { NX_EDIT_EVENT, NX_OPEN_EVENT } = await import("../src/index.js");

const window = document.defaultView!;
const created: NxViewerElement[] = [];

after(async () => {
  for (const element of created) {
    element.remove();
  }
  (await nxHost()).dispose();
});

/** The element showing `fixture`, everything unfolded, editable unless `editable` says otherwise. */
function show({ tree, text }: Fixture, editable = true): NxViewerElement {
  const element = document.createElement("nx-viewer");
  document.body.append(element);
  element.folded = [];
  element.editable = editable;
  element.text = text;
  element.tree = tree;
  created.push(element);
  return element;
}

/**
 * Lays the reading out as jsdom does not, for the test `t`: a line of more than `columns` characters
 * takes two lines, so its element gives each property a row.
 */
function narrow(t: TestContext, columns: number): void {
  const prototype = window.HTMLElement.prototype;
  const { getClientRects } = prototype;
  Object.defineProperty(prototype, "clientWidth", { configurable: true, get: () => columns * 8 });
  const tooLong = (element: Element): boolean => (element.textContent ?? "").replace(/\s+/g, " ").length > columns;
  prototype.getClientRects = function (this: HTMLElement) {
    const squeezed = this.parentElement?.closest(".line-element, .sheet-head");
    const long = tooLong(this) || (squeezed !== null && squeezed !== undefined && tooLong(squeezed));
    const height = long ? 36 : 18;
    return [{ top: 0, bottom: height, left: 0, right: 100, width: 100, height, x: 0, y: 0, toJSON: () => ({}) }] as unknown as DOMRectList;
  };
  t.after(() => {
    prototype.getClientRects = getClientRects;
    delete (prototype as { clientWidth?: number }).clientWidth;
  });
}

function root(element: NxViewerElement): ShadowRoot {
  return element.shadowRoot!;
}

/** The rendering of the node with `key`. */
function rendered(element: NxViewerElement, key: string): HTMLElement {
  const found = Array.from(root(element).querySelectorAll<HTMLElement>("[data-key]")).find((candidate) => candidate.dataset["key"] === key);
  assert.ok(found, `the node ${key} is rendered`);
  return found;
}

function body(element: NxViewerElement): HTMLElement {
  return root(element).querySelector<HTMLElement>(".body")!;
}

function press(target: Element, key: string, init: KeyboardEventInit = {}): void {
  target.dispatchEvent(new window.KeyboardEvent("keydown", { key, bubbles: true, composed: true, cancelable: true, ...init }));
}

/** The open field, if any. */
function field(element: NxViewerElement): HTMLElement | null {
  return root(element).querySelector<HTMLElement>(".field");
}

/** Every `nx-edit` and `nx-open` the element fires, in order. */
function events(element: NxViewerElement): { edits: NxEditDetail[]; opens: NxOpenDetail[] } {
  const edits: NxEditDetail[] = [];
  const opens: NxOpenDetail[] = [];
  element.addEventListener(NX_EDIT_EVENT, (event) => edits.push((event as CustomEvent<NxEditDetail>).detail));
  element.addEventListener(NX_OPEN_EVENT, (event) => opens.push((event as CustomEvent<NxOpenDetail>).detail));
  return { edits, opens };
}

/**
 * A selection that holds ranges in a shadow root, as browsers' do and jsdom's does not, standing in
 * for the document's for the test `t`.
 */
function fakeSelection(t: TestContext): Selection {
  let ranges: Range[] = [];
  const selection = {
    get rangeCount() {
      return ranges.length;
    },
    get isCollapsed() {
      return ranges.every((range) => range.collapsed);
    },
    getRangeAt: (index: number) => ranges[index]!,
    removeAllRanges: () => {
      ranges = [];
    },
    addRange: (range: Range) => {
      ranges = [range];
    },
    selectAllChildren: (node: Node) => {
      const range = document.createRange();
      range.selectNodeContents(node);
      ranges = [range];
    }
  } as unknown as Selection;
  const { getSelection } = document;
  document.getSelection = () => selection;
  t.after(() => {
    document.getSelection = getSelection;
  });
  return selection;
}

/** A pointer event as jsdom lacks one: a mouse event with a pointer id. */
function pointer(type: string, clientX: number, init: MouseEventInit = {}): MouseEvent {
  const event = new window.MouseEvent(type, { bubbles: true, composed: true, cancelable: true, clientX, button: 0, ...init });
  Object.defineProperty(event, "pointerId", { value: 1 });
  return event;
}

describe("what the reading can edit", () => {
  it("finds each editable value and its editor from the tree", async () => {
    const { tree, text } = await fixture("editing");
    const targets = editTargets(indexTree(tree, text));
    const kind = (key: string): string | undefined => targets.get(key)?.kind;
    assert.equal(kind("latte.value.name.value"), "text");
    assert.equal(targets.get("latte.value.name.value")?.value, "Latte");
    assert.equal(kind("latte.value.label.value"), "text", "a uitext string");
    assert.equal(targets.get("latte.value.label.value")?.value, 'Hot "latte"', "without its quotes and escapes");
    assert.equal(kind("latte.value.size.value"), "case");
    assert.deepEqual(
      targets.get("latte.value.size.value")?.cases?.map((option) => option.name),
      ["small", "medium", "large"],
      "the cases that hold no properties"
    );
    assert.equal(kind("latte.value.iced.value"), "boolean");
    assert.equal(kind("latte.value.shots.value"), "number", "a number typed by an alias of int");
    assert.equal(kind("latte.value.color.value"), "color", "a string its alias's visualizer formats as a color");
    assert.equal(kind("latte.value.tags.value[0]"), "text", "an item of a braced list");
    assert.equal(kind("total.value"), "number", "a parameterless let's value");
    assert.equal(kind("ratio.value"), "number");
    assert.equal(kind("note.value.text[0]"), "run", "a run of text on one line");
    assert.equal(kind("counter.value.count.value"), undefined, "an expression");
    assert.equal(kind("counter.value.count.value.right"), undefined, "an operand");
    assert.equal(kind("Drink.size.default"), undefined, "a default");
  });

  it("knows which properties may be left out", async () => {
    const { tree, text } = await fixture("editing");
    const index = indexTree(tree, text);
    const removable = (key: string) => isRemovable(index, index.byKey.get(key)!);
    assert.equal(removable("latte.value.size"), true, "a property with a default");
    assert.equal(removable("latte.value.color"), true, "an optional property");
    assert.equal(removable("latte.value.name"), false, "a required property");
  });
});

describe("where a click puts the caret", () => {
  it("falls before or after the character the point is over, on the point's line", (t) => {
    // Lays out text as jsdom does not: eight pixels a character, four characters a line.
    const prototype = window.Range.prototype as Range & { getBoundingClientRect(): DOMRect };
    const original = Object.getOwnPropertyDescriptor(prototype, "getBoundingClientRect");
    Object.defineProperty(prototype, "getBoundingClientRect", {
      configurable: true,
      value(this: Range) {
        const at = this.startOffset;
        const left = (at % 4) * 8;
        const top = Math.floor(at / 4) * 18;
        return { left, right: left + 8, top, bottom: top + 18, width: 8, height: 18, x: left, y: top } as DOMRect;
      }
    });
    t.after(() => {
      if (original === undefined) {
        delete (prototype as { getBoundingClientRect?: unknown }).getBoundingClientRect;
      } else {
        Object.defineProperty(prototype, "getBoundingClientRect", original);
      }
    });
    const span = document.createElement("span");
    span.textContent = "abcdefgh";
    document.body.append(span);
    t.after(() => span.remove());
    assert.equal(offsetAtPoint(span, 19, 9), 2, "before the third character, left of its middle");
    assert.equal(offsetAtPoint(span, 21, 9), 3, "after it, right of its middle");
    assert.equal(offsetAtPoint(span, 1, 27), 4, "the second line's start");
    assert.equal(offsetAtPoint(span, 200, 27), 8, "past the second line's end");
    assert.equal(offsetAtPoint(span, 200, 9), 4, "past the first line's end, at its end");
  });
});

describe("editing in place", () => {
  it("is off by default: nothing opens and nothing fires", async () => {
    const element = show(await fixture("editing"), false);
    const { edits, opens } = events(element);
    assert.equal(root(element).querySelector(".editable-value"), null, "no value is marked editable");
    const value = rendered(element, "latte.value.name.value");
    value.click();
    value.click();
    value.dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    element.selection = "latte.value";
    press(body(element), "Enter");
    press(body(element), " ");
    press(body(element), "Delete");
    rendered(element, "counter.value.count.value").dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    assert.equal(field(element), null);
    assert.deepEqual(edits, []);
    assert.deepEqual(opens, []);
  });

  it("takes editable set before the element is defined", async () => {
    const { tree, text } = await fixture("editing");
    const tag = "nx-viewer-editable-late";
    const element = document.createElement(tag) as NxViewerElement;
    document.body.append(element);
    Object.assign(element, { text, tree, editable: true });
    const { NxViewerElement: ViewerClass } = await import("../src/index.js");
    customElements.define(tag, class extends ViewerClass {});
    created.push(element);
    assert.equal(element.editable, true);
    assert.ok(root(element).querySelector(".editable-value"), "values are marked editable");
  });

  it("edits a string: Enter opens it, Enter commits, and the value reads muted until a newer tree", async (t) => {
    narrow(t, 40);
    const fixed = await fixture("editing");
    const element = show(fixed);
    const { edits } = events(element);
    element.selection = "latte.value.name";
    assert.ok(rendered(element, "latte.value.name").matches(".sheet-row"), "the name reads as a row");
    press(body(element), "Enter");
    const open = field(element);
    assert.ok(open, "a field opens");
    assert.equal(open.textContent, "Latte", "holding the value without quotes");
    assert.ok(rendered(element, "latte.value.name.value").contains(open), "in the value's own place");
    open.textContent = "Flat white";
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "latte.value.name.value", edit: { kind: "setValue", value: { kind: "string", value: "Flat white" } } }]);
    assert.equal(field(element), null, "the field closes");
    const value = rendered(element, "latte.value.name.value");
    assert.equal(value.textContent, "Flat white");
    assert.ok(value.classList.contains("pending"), "muted until the host's tree");
    element.text = fixed.text;
    assert.equal(rendered(element, "latte.value.name.value").textContent, "Flat white", "kept when the text is set again");
    // The host refuses: it sets the same tree again.
    element.tree = fixed.tree;
    assert.equal(rendered(element, "latte.value.name.value").textContent, "Latte", "the tree as it is");
    assert.equal(element.selection, "latte.value.name", "the selection is kept");
  });

  it("cancels on Escape: nothing fires, the value reads as before and the unit stays selected", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.label";
    press(body(element), "F2");
    const open = field(element)!;
    assert.equal(open.textContent, 'Hot "latte"', "a string's escapes read as what they escape");
    open.textContent = "Cold";
    press(open, "Escape");
    assert.deepEqual(edits, []);
    assert.equal(field(element), null);
    assert.ok(rendered(element, "latte.value.label.value").textContent!.includes('Hot \\"latte\\"'), "as before");
    assert.equal(element.selection, "latte.value.label");
  });

  it("opens a value on a click, with a caret and nothing selected, and selects on a click on its name", async (t) => {
    narrow(t, 40);
    const selection = fakeSelection(t);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    root(element).querySelector<HTMLElement>('.sheet-name[data-hover="latte.value.name"]')!.click();
    assert.equal(element.selection, "latte.value.name", "a click on the name selects the row");
    assert.equal(field(element), null, "and opens nothing");
    element.selection = undefined;
    rendered(element, "latte.value.name.value").click();
    const open = field(element);
    assert.ok(open, "a click on a value opens its field");
    assert.equal(element.selection, "latte.value.name", "and selects its row");
    const range = selection.getRangeAt(0);
    assert.ok(open.contains(range.startContainer), "the caret is in the field");
    assert.equal(range.collapsed, true, "where the click fell, with nothing selected");
    press(open, "Escape");
    assert.deepEqual(edits, []);
    // The second click of a double-click is the field's own, and opens nothing again.
    rendered(element, "latte.value.shots.value").click();
    const number = field(element)!;
    number.dispatchEvent(new window.MouseEvent("click", { bubbles: true, composed: true, detail: 2 }));
    number.dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    assert.equal(field(element), number);
    assert.equal(number.textContent, "2");
  });

  it("selects a value's text on Enter, to type over it", async (t) => {
    narrow(t, 40);
    const selection = fakeSelection(t);
    const element = show(await fixture("editing"));
    element.selection = "latte.value.name";
    press(body(element), "Enter");
    const range = selection.getRangeAt(0);
    assert.equal(range.toString(), "Latte");
  });

  it("moves to the next and previous value with Tab and Shift+Tab, committing", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.name";
    press(body(element), "Enter");
    field(element)!.textContent = "Mocha";
    press(field(element)!, "Tab");
    assert.equal(edits.length, 1, "Tab commits");
    assert.equal(element.selection, "latte.value.size", "and selects the next value's unit");
    assert.equal(root(element).querySelector<HTMLElement>(".popover")!.hidden, false, "opening its editor");
    press(root(element).querySelector(".popover-filter")!, "Tab", { shiftKey: true });
    assert.equal(element.selection, "latte.value.name", "Shift+Tab goes back");
    assert.equal(field(element)?.textContent, "Mocha", "to the value as the reader left it");
  });

  it("keeps an editor open across a newer tree", async (t) => {
    narrow(t, 40);
    const fixed = await fixture("editing");
    const element = show(fixed);
    element.selection = "latte.value.name";
    press(body(element), "Enter");
    field(element)!.textContent = "Half typed";
    element.tree = fixed.tree;
    assert.equal(field(element)?.textContent, "Half typed");
  });

  it("keeps the caret of an editor open across a newer tree, and through a tree cleared first", async (t) => {
    narrow(t, 40);
    const selection = fakeSelection(t);
    const fixed = await fixture("editing");
    const element = show(fixed);
    element.selection = "latte.value.name";
    press(body(element), "Enter");
    const open = field(element)!;
    open.textContent = "Half typed";
    const caret = document.createRange();
    caret.setStart(open.firstChild!, 4);
    caret.collapse(true);
    selection.removeAllRanges();
    selection.addRange(caret);
    // As a host does: the tree cleared, the text set, then the new tree.
    element.tree = undefined;
    element.text = fixed.text;
    element.tree = fixed.tree;
    const reopened = field(element)!;
    assert.equal(reopened.textContent, "Half typed");
    assert.equal(selection.rangeCount, 1);
    const range = selection.getRangeAt(0);
    assert.ok(reopened.contains(range.startContainer), "the caret is in the field");
    assert.equal(range.collapsed, true, "nothing is selected, so the next key does not replace the draft");
    assert.equal(range.startOffset, 4, "the caret is where the reader left it");
  });

  it("puts the caret after the draft when where it was cannot be read", async (t) => {
    narrow(t, 40);
    const selection = fakeSelection(t);
    const fixed = await fixture("editing");
    const element = show(fixed);
    element.selection = "latte.value.name";
    press(body(element), "Enter");
    field(element)!.textContent = "Half typed";
    selection.removeAllRanges();
    // An engine that takes the shadow roots in another form throws.
    (selection as Selection & { getComposedRanges(): never }).getComposedRanges = () => {
      throw new TypeError("not a ShadowRoot");
    };
    element.tree = fixed.tree;
    const range = selection.getRangeAt(0);
    assert.ok(field(element)!.contains(range.startContainer));
    assert.equal(range.collapsed, true, "a reopened field never selects the draft");
    assert.equal(range.startOffset, "Half typed".length);
  });

  it("takes a sign only where the grammar takes one", async (t) => {
    narrow(t, 40);
    const fixed = await fixture("editingEdges");
    const targets = editTargets(indexTree(fixed.tree, fixed.text));
    assert.equal(targets.get("total.value")?.signed, true, "an unbraced let value");
    assert.equal(targets.get("box.value.depth.value")?.signed, false, "a braced value");
    assert.equal(targets.get("box.value.items.value[0]")?.signed, false, "an item of a sequence");
    const element = show(fixed);
    const { edits } = events(element);
    rendered(element, "box.value.depth.value").dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    const open = field(element)!;
    assert.equal(open.textContent, "3");
    press(open, "ArrowDown", { shiftKey: true });
    assert.equal(open.textContent, "0", "the arrows stop at zero");
    open.textContent = "-1";
    press(open, "Enter");
    assert.deepEqual(edits, [], "a negative number is not committed");
    assert.ok(open.classList.contains("invalid"));
  });

  it("offers no text field for a string with an escape other than a quote or a backslash", async () => {
    const fixed = await fixture("editingEdges");
    const targets = editTargets(indexTree(fixed.tree, fixed.text));
    assert.equal(targets.get("box.value.title.value"), undefined, "\\n would be written back as n");
    assert.equal(targets.get("box.value.note.value")?.value, 'say "hi" \\ ok');
    const element = show(fixed);
    const { opens } = events(element);
    rendered(element, "box.value.title.value").dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    assert.equal(field(element), null);
    assert.equal(opens.length, 1, "it opens in the source instead");
  });

  it("edits a run of text without the space that separates it from what follows", async () => {
    const element = show(await fixture("editingEdges"));
    const { edits } = events(element);
    rendered(element, "mixed.value.text[0]").dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    const open = field(element)!;
    assert.equal(open.textContent, "Hello");
    open.textContent = "Hi";
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "mixed.value.text[0]", edit: { kind: "setText", text: "Hi" } }]);
  });

  it("picks a case from a popover of the union's cases", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.size";
    press(body(element), "Enter");
    const popover = root(element).querySelector<HTMLElement>(".popover")!;
    assert.equal(popover.hidden, false);
    const options = Array.from(popover.querySelectorAll<HTMLElement>(".option"));
    assert.deepEqual(options.map((option) => option.dataset["name"]), ["small", "medium", "large"]);
    assert.equal(popover.querySelector(".option.current")?.textContent?.startsWith("medium"), true, "the current case is marked");
    assert.equal(options[0]!.querySelector(".option-doc")?.textContent, "The smallest cup.", "each with its doc");
    const filter = popover.querySelector<HTMLInputElement>(".popover-filter")!;
    filter.value = "lar";
    filter.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(
      options.filter((option) => !option.hidden).map((option) => option.dataset["name"]),
      ["large"],
      "typing filters"
    );
    press(filter, "Enter");
    assert.deepEqual(edits, [{ key: "latte.value.size.value", edit: { kind: "setValue", value: { kind: "case", name: "large" } } }]);
    assert.equal(popover.hidden, true);
    assert.equal(rendered(element, "latte.value.size.value").textContent, "large");
  });

  it("closes a case's popover on a click on its value, as a dropdown does", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const value = rendered(element, "latte.value.size.value");
    value.click();
    const popover = root(element).querySelector<HTMLElement>(".popover")!;
    assert.equal(popover.hidden, false, "a click opens it");
    value.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true }));
    value.click();
    assert.equal(popover.hidden, true, "a second click closes it");
    value.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true }));
    value.click();
    assert.equal(popover.hidden, false, "and a third opens it again");
  });

  it("chooses a case with the pointer and moves through them with the arrows", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.size";
    press(body(element), "Enter");
    const popover = root(element).querySelector<HTMLElement>(".popover")!;
    const filter = popover.querySelector(".popover-filter")!;
    press(filter, "ArrowDown");
    assert.equal(popover.querySelector(".option.active")?.getAttribute("data-name"), "large");
    press(filter, "Escape");
    assert.equal(edits.length, 0, "Escape chooses nothing");
    press(body(element), "Enter");
    root(element).querySelector<HTMLElement>('.option[data-name="small"]')!.click();
    assert.deepEqual(edits.map((edit) => edit.edit), [{ kind: "setValue", value: { kind: "case", name: "small" } }]);
  });

  it("toggles a boolean with Space, and with a click on its mark", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.iced";
    press(body(element), " ");
    assert.deepEqual(edits[0], { key: "latte.value.iced.value", edit: { kind: "setValue", value: { kind: "boolean", value: true } } });
    assert.equal(rendered(element, "latte.value.iced.value").textContent, "✓", "reads as a check");
    rendered(element, "latte.value.iced.value").click();
    assert.deepEqual(edits[1]?.edit, { kind: "setValue", value: { kind: "boolean", value: false } }, "a click toggles it back");
    assert.equal(field(element), null, "no field opens");
    rendered(element, "latte.value.iced.value").dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, composed: true, detail: 2 })
    );
    assert.equal(edits.length, 2, "a double-click's second click does not toggle it again");
    element.selection = undefined;
    rendered(element, "latte.value.iced.value").click();
    assert.equal(edits.length, 3, "a click toggles it with nothing selected first");
    assert.equal(element.selection, "latte.value.iced", "and selects its row");
  });

  it("steps a number with the arrows and keeps an entry it cannot take in the field, marked", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.shots";
    press(body(element), "Enter");
    const open = field(element)!;
    press(open, "ArrowUp");
    assert.equal(open.textContent, "3");
    press(open, "ArrowUp", { shiftKey: true });
    assert.equal(open.textContent, "13");
    press(open, "ArrowDown");
    assert.equal(open.textContent, "12");
    open.textContent = "12px";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.ok(open.classList.contains("invalid"));
    press(open, "Enter");
    assert.deepEqual(edits, [], "not committed");
    assert.equal(field(element), open, "the field stays open");
    open.textContent = "12";
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "latte.value.shots.value", edit: { kind: "setValue", value: { kind: "number", text: "12" } } }]);
  });

  it("changes a number while its name is dragged sideways, and commits once on release", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    const name = root(element).querySelector<HTMLElement>('.scrub[data-hover="latte.value.shots"]');
    assert.ok(name, "the number's name drags");
    name.dispatchEvent(pointer("pointerdown", 100));
    name.dispatchEvent(pointer("pointermove", 101));
    assert.equal(rendered(element, "latte.value.shots.value").textContent, "2", "a press is not a drag");
    name.dispatchEvent(pointer("pointermove", 110));
    assert.equal(rendered(element, "latte.value.shots.value").textContent, "7", "a step to each two pixels");
    name.dispatchEvent(pointer("pointermove", 120));
    assert.equal(rendered(element, "latte.value.shots.value").textContent, "12");
    assert.deepEqual(edits, [], "nothing is committed while dragging");
    name.dispatchEvent(pointer("pointerup", 120));
    name.click();
    assert.deepEqual(edits, [{ key: "latte.value.shots.value", edit: { kind: "setValue", value: { kind: "number", text: "12" } } }]);
    assert.notEqual(element.selection, "latte.value.shots", "the drag's click does not select");
  });

  it("carries a drag on through a newer tree", async (t) => {
    narrow(t, 40);
    const fixed = await fixture("editing");
    const element = show(fixed);
    const { edits } = events(element);
    const scrubName = (): HTMLElement => root(element).querySelector<HTMLElement>('.scrub[data-hover="latte.value.shots"]')!;
    scrubName().dispatchEvent(pointer("pointerdown", 100));
    scrubName().dispatchEvent(pointer("pointermove", 110));
    assert.equal(rendered(element, "latte.value.shots.value").textContent, "7");
    element.tree = fixed.tree;
    assert.equal(rendered(element, "latte.value.shots.value").textContent, "7", "the number dragged to is still shown");
    assert.ok(body(element).classList.contains("scrubbing"));
    scrubName().dispatchEvent(pointer("pointermove", 120));
    assert.equal(rendered(element, "latte.value.shots.value").textContent, "12", "the drag goes on");
    scrubName().dispatchEvent(pointer("pointerup", 120));
    assert.deepEqual(edits, [{ key: "latte.value.shots.value", edit: { kind: "setValue", value: { kind: "number", text: "12" } } }]);
    assert.ok(!body(element).classList.contains("scrubbing"), "the reading reads as before");
  });

  it("ends a drag cleanly when the number is gone from the newer tree", async (t) => {
    narrow(t, 40);
    const fixed = await fixture("editing");
    const element = show(fixed);
    const name = root(element).querySelector<HTMLElement>('.scrub[data-hover="latte.value.shots"]')!;
    name.dispatchEvent(pointer("pointerdown", 100));
    name.dispatchEvent(pointer("pointermove", 110));
    element.tree = (await fixture("editingEdges")).tree;
    assert.ok(!body(element).classList.contains("scrubbing"), "the reading does not stay in a drag");
  });

  it("sets a color from a hex field", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.color";
    press(body(element), "Enter");
    const hex = root(element).querySelector<HTMLInputElement>(".popover-hex")!;
    assert.equal(hex.value, "#5a3a22");
    assert.equal(root(element).querySelector<HTMLInputElement>(".popover-picker")!.value, "#5a3a22");
    hex.value = "brown";
    press(hex, "Enter");
    assert.deepEqual(edits, [], "a hex field takes a hex color");
    assert.ok(hex.classList.contains("invalid"));
    hex.value = "#336699";
    press(hex, "Enter");
    assert.deepEqual(edits, [{ key: "latte.value.color.value", edit: { kind: "setValue", value: { kind: "string", value: "#336699" } } }]);
    const popover = root(element).querySelector<HTMLElement>(".popover")!;
    assert.equal(popover.hidden, true);
    assert.equal(popover.className, "popover", "a closed popover keeps no kind");
    assert.equal(window.getComputedStyle(popover).display, "none", "and nothing of it shows");
  });

  it("sets a run of text as written, and keeps what would not read back in the field", async () => {
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    rendered(element, "note.value.text[0]").dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    const open = field(element)!;
    assert.equal(open.textContent, "Hello there");
    open.textContent = "Hi <there>";
    press(open, "Enter");
    assert.deepEqual(edits, []);
    open.textContent = "Hi & bye";
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "note.value.text[0]", edit: { kind: "setText", text: "Hi & bye" } }]);
  });

  it("resets a property with a default, and selects its element", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.size";
    assert.equal(root(element).querySelector(".row-action"), null, "no button offers it");
    press(body(element), "Delete");
    assert.deepEqual(edits[0], { key: "latte.value.size", edit: { kind: "remove" } });
    assert.equal(element.selection, "latte.value", "the element is selected");
    element.selection = "latte.value.color";
    press(body(element), "Backspace");
    assert.equal(edits.length, 1, "Backspace, pressed to change the text, does not remove it");
    press(body(element), "Backspace", { metaKey: true });
    assert.deepEqual(edits[1], { key: "latte.value.color", edit: { kind: "remove" } }, "Cmd+Backspace, a Mac laptop's Delete, clears it");
  });

  it("leaves a required property in place", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    element.selection = "latte.value.name";
    press(body(element), "Delete");
    assert.deepEqual(edits, []);
  });

  it("asks the host to open an expression in the source", async () => {
    const element = show(await fixture("editing"));
    const { edits, opens } = events(element);
    const operand = rendered(element, "counter.value.count.value.left");
    operand.dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    assert.equal(opens.length, 1);
    assert.equal(opens[0]!.key, "counter.value.count.value", "the whole value");
    assert.equal(opens[0]!.role, "operator");
    assert.deepEqual(edits, []);
  });

  it("leaves Ctrl+Z to the host, and keeps a field's keys to itself", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const reached: string[] = [];
    element.addEventListener("keydown", (event) => reached.push((event as KeyboardEvent).key));
    press(body(element), "z", { ctrlKey: true });
    assert.deepEqual(reached, ["z"], "the reading's undo is the host's");
    element.selection = "latte.value.name";
    press(body(element), "Enter");
    reached.length = 0;
    press(field(element)!, "z", { ctrlKey: true });
    assert.deepEqual(reached, [], "a field's undo is its own");
  });

  it("moves nothing else when an editor opens, and closes a hover card", async () => {
    const element = show(await fixture("editing"));
    const before = Array.from(root(element).querySelectorAll<HTMLElement>("[data-key]"), (node) => [node, node.textContent] as const);
    const card = root(element).querySelector<HTMLElement>(".card")!;
    element.selection = "counter.value";
    press(body(element), "i");
    assert.equal(card.hidden, false, "a card is open");
    rendered(element, "latte.value.name.value").dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    const open = field(element)!;
    assert.ok(open);
    assert.equal(card.hidden, true, "opening an editor closes the card");
    for (const [node, text] of before) {
      if (!node.contains(open)) {
        assert.ok(node.isConnected, `${node.dataset["key"]} is still drawn`);
        assert.equal(node.textContent, text, `${node.dataset["key"]} reads as it did`);
      }
    }
  });
});
