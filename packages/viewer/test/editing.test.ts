import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it, type TestContext } from "node:test";

import { fixture, nxHost, treeOf, type Fixture } from "./fixtures.js";
import { editTargets, isRemovable, unwrittenProperties } from "../src/edit.js";
import { cardOf } from "../src/card.js";
import { offsetAtPoint, rangeProblem } from "../src/editors.js";
import { indexTree } from "../src/tree.js";
import type { NxEditDetail, NxOpenDetail, NxViewerElement } from "../src/index.js";

const { NX_DRAFT_EVENT, NX_EDIT_EVENT, NX_OPEN_EVENT } = await import("../src/index.js");

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

/**
 * Every `nx-edit` and `nx-open` the element fires, in order: the edits it commits, and apart from
 * them the live edits it fires while a value changes.
 */
function events(element: NxViewerElement): { edits: NxEditDetail[]; opens: NxOpenDetail[]; lives: NxEditDetail[] } {
  const edits: NxEditDetail[] = [];
  const opens: NxOpenDetail[] = [];
  const lives: NxEditDetail[] = [];
  element.addEventListener(NX_EDIT_EVENT, (event) => {
    const detail = (event as CustomEvent<NxEditDetail>).detail;
    (detail.live === true ? lives : edits).push(detail);
  });
  element.addEventListener(NX_OPEN_EVENT, (event) => opens.push((event as CustomEvent<NxOpenDetail>).detail));
  return { edits, opens, lives };
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

describe("a reading drawn again", () => {
  it("keeps its height while it is drawn, so what scrolls it does not move up", async () => {
    const editing = await fixture("editing");
    const element = show(editing);
    const body = root(element).querySelector<HTMLElement>(".body")!;
    // jsdom lays nothing out: the reading is 500 pixels tall, and the fitting's first measure, taken
    // on a draft that may be shorter, says what holds the reading's height then.
    let held: string | undefined;
    Object.defineProperty(body, "offsetHeight", { configurable: true, get: () => 500 });
    Object.defineProperty(body, "clientWidth", {
      configurable: true,
      get: () => {
        held = body.style.minHeight;
        return 0;
      }
    });
    // As the playground gives a tree after an edit: none, then the next.
    element.tree = undefined;
    element.tree = editing.tree;
    assert.equal(held, "500px", "the height it had holds while it is measured");
    assert.equal(body.style.minHeight, "", "and lets go once it is drawn");
    // A host that gives the next tree a task later, once it has it, keeps the height too.
    held = undefined;
    element.tree = undefined;
    await new Promise((resolve) => setTimeout(resolve, 0));
    element.tree = editing.tree;
    assert.equal(held, "500px", "the height holds across a tree given later");
    assert.equal(body.style.minHeight, "", "and lets go once it is drawn");
  });

  it("lets an empty reading's height go when no tree comes", async (t) => {
    const editing = await fixture("editing");
    const element = show(editing);
    const body = root(element).querySelector<HTMLElement>(".body")!;
    Object.defineProperty(body, "offsetHeight", { configurable: true, get: () => 500 });
    t.mock.timers.enable({ apis: ["setTimeout"] });
    element.tree = undefined;
    assert.equal(body.style.minHeight, "500px", "it holds the height a while");
    t.mock.timers.tick(1000);
    assert.equal(body.style.minHeight, "", "and lets it go when no tree has come");
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

  it("says when an open field holds a change, and when it no longer does", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const drafts: boolean[] = [];
    element.addEventListener(NX_DRAFT_EVENT, (event) => drafts.push((event as CustomEvent<{ changed: boolean }>).detail.changed));
    element.selection = "latte.value.name";
    press(body(element), "Enter");
    assert.deepEqual(drafts, [], "opening changes nothing");
    const open = field(element)!;
    open.textContent = "Mocha";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    open.textContent = "Mochas";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(drafts, [true], "a change, once");
    open.textContent = "Latte";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(drafts, [true, false], "typed back to its value");
    open.textContent = "Mocha";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    press(open, "Enter");
    assert.deepEqual(drafts, [true, false, true, false], "committed");
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
    assert.ok(rendered(element, "latte.value.label.value").textContent!.includes('Hot "latte"'), "as before");
    assert.equal(element.selection, "latte.value.label");
  });

  it("opens a value on a click, with a caret and its property selected, and selects on a click on its name", async (t) => {
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
    assert.equal(element.selection, "latte.value.name", "and selects its property");
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
    assert.equal(root(element).querySelector<HTMLElement>(".popover")!.hidden, false, "opening the next value's editor");
    assert.equal(element.selection, "latte.value.size", "and selecting its property");
    press(root(element).querySelector(".popover-filter")!, "Tab", { shiftKey: true });
    assert.ok(field(element) !== null, "Shift+Tab goes back");
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

  it("reads a string's quote and backslash escapes as what they escape outside code, and other escapes as written", async (t) => {
    fakeSelection(t);
    const element = show(await fixture("editingEdges"));
    assert.equal(rendered(element, "box.value.note.value").textContent, 'say "hi" \\ ok');
    assert.equal(rendered(element, "box.value.title.value").textContent, "a\\nb");
    const { edits } = events(element);
    // A string drawn as code, between its own quotes, as an item of a sequence is, keeps its escapes:
    // a bare quote there would look like the string's end.
    const item = root(element).querySelector<HTMLElement>('[data-key^="said."][data-role="literal"]')!;
    assert.equal(item.textContent, '"say \\"hi\\""');
    item.click();
    const open = field(element)!;
    assert.equal(open.textContent, 'say "hi"', "its field holds the value itself");
    assert.equal(item.textContent, '"say "hi""', "between the quotes the reading draws");
    open.textContent = 'say "yo"';
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: item.dataset["key"], edit: { kind: "setValue", value: { kind: "string", value: 'say "yo"' } } }]);
    assert.equal(item.textContent, '"say \\"yo\\""', "and shown as the reading draws it until the host's tree");
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
    assert.equal(element.selection, "latte.value.iced", "and selects its property");
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
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const problem = root(element).querySelector<HTMLElement>(".edit-problem")!;
    open.textContent = "12px";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.ok(open.classList.contains("invalid"));
    assert.equal(problem.hidden, true, "why waits for typing to pause");
    t.mock.timers.tick(400);
    assert.equal(problem.hidden, false, "and then says why");
    assert.equal(problem.textContent, "Not a number");
    assert.equal(open.getAttribute("aria-describedby"), problem.id);
    open.textContent = "1";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.equal(problem.hidden, true, "an entry it can take says nothing");
    open.textContent = "12px";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    press(open, "Enter");
    assert.deepEqual(edits, [], "not committed");
    assert.equal(field(element), open, "the field stays open");
    assert.equal(problem.hidden, false, "Enter says why at once");
    open.textContent = "12";
    press(open, "Enter");
    assert.equal(problem.hidden, true, "and closing the field hides it");
    assert.deepEqual(edits, [{ key: "latte.value.shots.value", edit: { kind: "setValue", value: { kind: "number", text: "12" } } }]);
  });

  it("does not drag a number's name: a press and a move on it select the property and change nothing", async (t) => {
    narrow(t, 40);
    const element = show(await fixture("editing"));
    const { edits } = events(element);
    const name = root(element).querySelector<HTMLElement>('.sheet-name[data-hover="latte.value.shots"]')!;
    assert.ok(!name.classList.contains("scrub"));
    name.dispatchEvent(pointer("pointerdown", 100));
    name.dispatchEvent(pointer("pointermove", 120));
    assert.equal(rendered(element, "latte.value.shots.value").textContent, "2", "the number stays");
    name.dispatchEvent(pointer("pointerup", 120));
    name.click();
    assert.equal(element.selection, "latte.value.shots");
    assert.deepEqual(edits, []);
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

/** The rows listing the selected element's unwritten properties, as drawn. */
function moreRows(element: NxViewerElement): HTMLElement[] {
  return Array.from(root(element).querySelectorAll<HTMLElement>(".dimmed-row, .added-row"));
}

function moreToggle(element: NxViewerElement): HTMLElement | null {
  return root(element).querySelector<HTMLElement>(".more-toggle");
}

/** What a row reads, its spaces run together. */
function reads(row: Element): string {
  return Array.from(row.children)
    .map((cell) => cell.textContent ?? "")
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
}

function popover(element: NxViewerElement): HTMLElement {
  return root(element).querySelector<HTMLElement>(".popover")!;
}

describe("what an element does not write", () => {
  it("lists the unwritten properties in the visualizer's order, else declaration order, with their editors", async () => {
    const { tree, text } = await fixture("adding");
    const index = indexTree(tree, text);
    const listed = (key: string) =>
      unwrittenProperties(index, index.byKey.get(key)!).map((entry) => `${entry.property.name}:${entry.editor?.kind ?? "-"}`);
    assert.deepEqual(listed("latte.value"), ["note:text", "iced:boolean", "shots:number", "color:color"]);
    assert.deepEqual(listed("bare.value"), ["gap:number", "count:number", "mode:case", "body:text", "padding:number", "theme:-"]);
    assert.deepEqual(listed("latte.value.code"), [], "an attribute is no element");
  });

  it("ends the selected element with Show N unset properties, and only the selected one, only when editable", async () => {
    const element = show(await fixture("adding"));
    assert.equal(moreToggle(element), null, "nothing is selected");
    element.selection = "latte.value";
    assert.equal(moreToggle(element)?.textContent, "Show 4 unset properties");
    assert.equal(root(element).querySelectorAll(".more-toggle").length, 1);
    element.selection = "latte.value.code";
    assert.equal(moreToggle(element), null, "a row is not an element");
    element.selection = "wide.value";
    assert.equal(moreToggle(element)?.textContent, "Show 14 unset properties");
    const reading = show(await fixture("adding"), false);
    reading.selection = "latte.value";
    assert.equal(moreToggle(reading), null, "not when the reading only reads");
  });

  it("opens to dimmed rows reading each default, and closes on Escape or another selection", async () => {
    const element = show(await fixture("adding"));
    element.selection = "latte.value";
    moreToggle(element)!.click();
    assert.deepEqual(moreRows(element).map(reads), ["note", "iced ✗ default", "shots 1 default", "color"]);
    assert.equal(element.selection, "latte.value", "the element stays selected");
    press(moreRows(element)[0]!, "Escape");
    assert.deepEqual(moreRows(element), [], "Escape closes the list");
    assert.ok(moreToggle(element), "and keeps the row");
    moreToggle(element)!.click();
    element.selection = "mocha.value";
    assert.deepEqual(moreRows(element), [], "another selection closes it");
  });

  it("opens with Enter on the row, and keeps open across a newer tree", async () => {
    const fixed = await fixture("adding");
    const element = show(fixed);
    element.selection = "latte.value";
    press(moreToggle(element)!, "Enter");
    assert.equal(moreRows(element).length, 4);
    element.tree = fixed.tree;
    assert.equal(moreRows(element).length, 4, "still open on the same element");
  });

  it("adds a property from its dimmed row, which then reads as written", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    element.selection = "latte.value";
    moreToggle(element)!.click();
    rendered(element, "latte.value.iced.value").click();
    assert.deepEqual(edits, [
      { key: "latte.value", edit: { kind: "addProperty", name: "iced", value: { kind: "boolean", value: true } } }
    ]);
    const row = moreRows(element)[1]!;
    assert.ok(row.classList.contains("added-row"), "no longer dimmed");
    assert.equal(reads(row), "iced ✓", "without its default note");
    assert.equal(element.selection, "latte.value", "the selection is kept");
  });

  it("names the toggle for what it lists, and reads an unset value's type in its place", async () => {
    const element = show(await fixture("adding"));
    element.selection = "latte.value";
    assert.equal(moreToggle(element)!.getAttribute("aria-label"), "4 unset properties");
    moreToggle(element)!.click();
    assert.equal(moreToggle(element)!.textContent, "Hide unset properties");
    assert.equal(moreToggle(element)!.getAttribute("aria-expanded"), "true");
    const note = rendered(element, "latte.value.note.value");
    assert.equal(note.textContent, "", "nothing is written there");
    assert.equal(note.dataset["placeholder"], "string", "its type, as a placeholder");
    assert.equal(rendered(element, "latte.value.iced.value").dataset["placeholder"], undefined, "a default reads instead");
    note.click();
    assert.equal(field(element)!.dataset["placeholder"], "string", "the field keeps it until something is typed");
  });

  it("moves nothing when the newer tree writes the added property", async () => {
    const fixed = await fixture("adding");
    const element = show(fixed);
    element.selection = "latte.value";
    moreToggle(element)!.click();
    rendered(element, "latte.value.iced.value").click();
    const text = fixed.text.replace('size=small />', "size=small iced=true />");
    assert.notEqual(text, fixed.text);
    element.tree = (await treeOf([{ uri: "nx:///adding.nx", identity: "adding.nx", source: text }], { visualizers: { modules: [] } })).tree;
    assert.equal(element.selection, "latte.value", "the selection is kept");
    assert.ok(rendered(element, "latte.value.iced"), "the property reads as written");
    assert.equal(moreToggle(element)?.textContent, "Hide unset properties");
    assert.deepEqual(
      moreRows(element).map((row) => row.dataset["name"]),
      ["note", "shots", "color"],
      "the list stays open on the rest"
    );
  });

  it("opens a dimmed number on its default and adds what is typed", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    element.selection = "latte.value";
    moreToggle(element)!.click();
    rendered(element, "latte.value.shots.value").click();
    const open = field(element)!;
    assert.equal(open.textContent, "1", "its default");
    open.textContent = "3";
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "latte.value", edit: { kind: "addProperty", name: "shots", value: { kind: "number", text: "3" } } }]);
  });

  it("opens Source for a property no editor takes", async () => {
    const element = show(await fixture("adding"));
    const { edits, opens } = events(element);
    element.selection = "bare.value";
    moreToggle(element)!.click();
    const theme = moreRows(element).find((row) => row.dataset["name"] === "theme")!;
    press(theme, "Enter");
    assert.deepEqual(edits, []);
    assert.deepEqual(opens.map((open) => open.key), ["bare.value"]);
    theme.dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    assert.equal(opens.length, 2);
  });

  it("filters past twelve", async () => {
    const element = show(await fixture("adding"));
    element.selection = "wide.value";
    moreToggle(element)!.click();
    const filter = root(element).querySelector<HTMLInputElement>(".more-filter");
    assert.ok(filter, "a filter leads the rows");
    filter.value = "c";
    filter.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(moreRows(element).filter((row) => !row.hidden).map((row) => row.dataset["name"]), ["c"]);
    element.selection = "latte.value";
    moreToggle(element)!.click();
    assert.equal(root(element).querySelector(".more-filter"), null, "four rows need no filter");
  });

  it("shows a dimmed name's property card", async () => {
    const element = show(await fixture("adding"));
    element.selection = "latte.value";
    moreToggle(element)!.click();
    const name = moreRows(element)[0]!.querySelector<HTMLElement>(".sheet-name")!;
    assert.equal(name.dataset["hover"], "latte.value.note");
  });
});

describe("a row's menu", () => {
  it("resets a property with a default, removes an optional one, and shows any row in Source", async (t) => {
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits, opens } = events(element);
    const menuOf = (key: string): string[] => {
      rendered(element, key).querySelector<HTMLElement>(":scope > * > .row-anchor > .row-menu")!.click();
      return Array.from(root(element).querySelectorAll(".menu-item")).map(reads);
    };
    assert.ok(rendered(element, "latte.value.size").matches(".sheet-row"), "the drink reads as a group");
    assert.deepEqual(menuOf("latte.value.size"), ["Reset to default medium", "Show in Source"], "the default it resets to");
    assert.equal(element.selection, "latte.value.size", "opening the menu selects its row");
    root(element).querySelector<HTMLElement>(".menu-item")!.click();
    assert.deepEqual(edits, [{ key: "latte.value.size", edit: { kind: "remove" } }]);
    assert.ok(root(element).querySelector<HTMLElement>(".menu")!.hidden, "a choice closes it");
    assert.deepEqual(menuOf("mocha.value.color"), ["Remove", "Show in Source"]);
    press(root(element).querySelector(".menu-item")!, "Escape");
    assert.ok(root(element).querySelector<HTMLElement>(".menu")!.hidden, "Escape closes it");
    assert.equal(edits.length, 1);
    assert.deepEqual(menuOf("latte.value.name"), ["Show in Source"], "a required property is not left out");
    press(root(element).querySelector(".menu-item")!, "Escape");
    assert.deepEqual(menuOf("card.value.padding"), ["Reset to default 8", "Show in Source"]);
    press(root(element).querySelector(".menu-item")!, "Escape");
    assert.deepEqual(menuOf("card.value.gap"), ["Reset to default 4", "Show in Source"]);
    press(root(element).querySelector(".menu-item")!, "Escape");
    menuOf("latte.value.name");
    root(element).querySelector<HTMLElement>(".menu-item")!.click();
    assert.deepEqual(opens.map((open) => open.key), ["latte.value.name.value"]);
  });

  it("closes on a second press and click on its mark, and is not read as part of the row", async (t) => {
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const mark = rendered(element, "latte.value.size").querySelector<HTMLElement>(":scope > * > .row-anchor > .row-menu")!;
    const menu = root(element).querySelector<HTMLElement>(".menu")!;
    const pressAndClick = () => {
      mark.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true }));
      mark.dispatchEvent(new window.MouseEvent("pointerup", { bubbles: true, composed: true }));
      mark.click();
    };
    pressAndClick();
    assert.equal(menu.hidden, false, "the first opens it");
    pressAndClick();
    assert.equal(menu.hidden, true, "the second closes it");
    element.selection = "latte.value.size";
    assert.ok(!(element.getAttribute("aria-label") ?? "").includes("\u22EF"));
    const status = root(element).querySelector("[aria-live]")?.textContent ?? "";
    assert.ok(!status.includes("\u22EF"), status);
  });

  it("is a property's on a line too: its name selects it, the mark follows it, and Delete removes it", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    const property = rendered(element, "mocha.value.color");
    assert.ok(property.matches(".line-property"), "the drink reads as a line");
    const mark = Array.from(body(element).querySelectorAll<HTMLElement>(".row-menu.shared")).find((candidate) => candidate.dataset["menuKey"] === "mocha.value.color")!;
    assert.ok(mark, "a mark for the property");
    assert.equal(property.nextElementSibling, mark.parentElement, "right after the property, not inside it");
    assert.ok(!mark.classList.contains("selected"), "hidden until the property is selected");
    property.querySelector<HTMLElement>(".line-name")!.click();
    assert.equal(element.selection, "mocha.value.color", "a click on its name selects the property");
    assert.ok(mark.classList.contains("selected"), "and shows its mark");
    mark.click();
    assert.deepEqual(Array.from(root(element).querySelectorAll(".menu-item")).map(reads), ["Remove", "Show in Source"]);
    press(root(element).querySelector(".menu-item")!, "Escape");
    element.selection = "mocha.value.color";
    press(body(element), "Delete");
    assert.deepEqual(edits, [{ key: "mocha.value.color", edit: { kind: "remove" } }]);
  });

  it("opens from the keyboard on the selected row", async (t) => {
    narrow(t, 30);
    const element = show(await fixture("adding"));
    element.selection = "latte.value.size";
    press(body(element), "F10", { shiftKey: true });
    const items = Array.from(root(element).querySelectorAll<HTMLElement>(".menu-item"));
    assert.equal(items.length, 2);
    assert.equal(root(element).activeElement, items[0], "its first item takes focus");
    press(items[0]!, "ArrowDown");
    assert.equal(root(element).activeElement, items[1]);
  });
});

describe("editor hints", () => {
  it("keeps a number out of range in its field with a note", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "card.value.padding.value").click();
    const input = field(element)!;
    input.textContent = "80";
    input.dispatchEvent(new window.Event("input", { bubbles: true }));
    press(input, "Enter");
    assert.deepEqual(edits, []);
    assert.equal(root(element).querySelector(".edit-problem")?.textContent, "At most 64");
    input.textContent = "12";
    input.dispatchEvent(new window.Event("input", { bubbles: true }));
    press(input, "Enter");
    assert.deepEqual(edits, [{ key: "card.value.padding.value", edit: { kind: "setValue", value: { kind: "number", text: "12" } } }]);
  });

  it("steps a number within its range", async () => {
    const element = show(await fixture("adding"));
    rendered(element, "card.value.count.value").click();
    const open = field(element)!;
    open.textContent = "9";
    press(open, "ArrowUp", { shiftKey: true });
    assert.equal(open.textContent, "10", "kept at the most");
  });

  it("shows a segmented union as a row of buttons", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "card.value.mode.value").click();
    assert.ok(popover(element).classList.contains("segmented"));
    const filter = root(element).querySelector(".popover-filter")!;
    press(filter, "ArrowRight");
    press(filter, "Enter");
    assert.deepEqual(edits, [{ key: "card.value.mode.value", edit: { kind: "setValue", value: { kind: "case", name: "dark" } } }]);
  });

  it("breaks a string's line with Shift+Enter, and with Enter where it takes several lines", async (t) => {
    fakeSelection(t);
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "mocha.value.note.value").click();
    let open = field(element)!;
    open.textContent = "Ask for oat milk";
    const caret = document.createRange();
    caret.setStart(open.firstChild!, 3);
    caret.collapse(true);
    document.getSelection()!.removeAllRanges();
    document.getSelection()!.addRange(caret);
    press(open, "Enter", { shiftKey: true });
    assert.equal(open.textContent, "Ask\n for oat milk");
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "mocha.value.note.value", edit: { kind: "setValue", value: { kind: "string", value: "Ask\n for oat milk" } } }]);
    rendered(element, "card.value.body.value").click();
    open = field(element)!;
    assert.equal(open.dataset["placeholder"], "Say more", "an empty field shows its placeholder");
    assert.ok(open.classList.contains("field-lines"), "one box over several lines");
    open.textContent = "One";
    const end = document.createRange();
    end.setStart(open.firstChild!, 3);
    end.collapse(true);
    document.getSelection()!.removeAllRanges();
    document.getSelection()!.addRange(end);
    press(open, "Enter");
    assert.equal(edits.length, 1, "Enter breaks the line where the hint says several lines");
    assert.equal(open.textContent, "One\n\n", "with the line break a browser keeps after a last one");
    press(open, "Enter", { ctrlKey: true });
    assert.equal(edits.length, 2, "Ctrl+Enter commits");
    assert.deepEqual(edits[1]!.edit, { kind: "setValue", value: { kind: "string", value: "One\n" } }, "without the browser's line break");
  });
});

/** The rows of the choices list, shown or not. */
function choices(element: NxViewerElement): HTMLElement[] {
  return Array.from(popover(element).querySelectorAll<HTMLElement>(".option.choice"));
}

/** What the shown rows of the choices list read. */
function shownChoices(element: NxViewerElement): string[] {
  return popover(element).hidden ? [] : choices(element).filter((row) => !row.hidden).map(reads);
}

function tools(element: NxViewerElement): HTMLElement | null {
  return root(element).querySelector<HTMLElement>(".value-tools");
}

/** Presses `part` of the tools and lets go over it, as a click does, at the heights given. */
function pressTool(part: Element, init: { shiftKey?: boolean; from?: number; to?: number[] } = {}): void {
  const at = (type: string, clientY: number) =>
    part.dispatchEvent(new window.MouseEvent(type, { bubbles: true, composed: true, cancelable: true, clientY, shiftKey: init.shiftKey === true }));
  at("pointerdown", init.from ?? 0);
  for (const y of init.to ?? []) {
    at("pointermove", y);
  }
  at("pointerup", init.to?.at(-1) ?? init.from ?? 0);
}

describe("choices", () => {
  it("lists a number's labeled choices under its field once asked, the default first, and picks one", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "base.value.size.value").click();
    const open = field(element)!;
    assert.ok(popover(element).hidden, "the list waits to be asked for");
    assert.ok(tools(element)?.classList.contains("with-choices"), "a chevron says it is there");
    assert.equal(open.getAttribute("aria-expanded"), "false");
    press(open, "ArrowDown", { altKey: true });
    assert.deepEqual(shownChoices(element), ["Default 16", "sm 12", "base 16", "lg 20"]);
    assert.ok(popover(element).querySelector(".choices-list.labeled"), "two columns");
    assert.ok(popover(element).querySelector(".option.choice-default + .choices-rule"), "a rule sets the default apart");
    assert.deepEqual(
      choices(element).filter((row) => row.classList.contains("current")).map(reads),
      ["base 16"],
      "the current value's choice is marked, not the default's"
    );
    assert.equal(open.getAttribute("aria-expanded"), "true");
    press(open, "ArrowDown");
    assert.equal(open.getAttribute("aria-activedescendant"), choices(element)[3]!.id, "Down starts from the current value's row");
    press(open, "ArrowUp");
    press(open, "ArrowUp");
    assert.equal(open.getAttribute("aria-activedescendant"), choices(element)[1]!.id);
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "base.value.size.value", edit: { kind: "setValue", value: { kind: "number", text: "12" } } }]);
    assert.equal(field(element), null);
  });

  it("keeps a choice the field would refuse in the field, with its note; a field with choices is a combobox", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "card.value.count.value").click();
    const open = field(element)!;
    assert.equal(open.getAttribute("role"), "combobox");
    assert.equal(open.getAttribute("aria-autocomplete"), "list");
    assert.equal(open.getAttribute("aria-controls"), null, "no list is named while none is drawn");
    press(open, "ArrowDown", { altKey: true });
    assert.deepEqual(shownChoices(element), ["Default 0", "5", "99"]);
    assert.ok(!popover(element).querySelector(".choices-list")!.classList.contains("labeled"), "one column");
    const list = popover(element).querySelector(".choices-list")!;
    assert.equal(open.getAttribute("aria-controls"), list.id);
    const rule = list.querySelector(".choices-rule")!;
    assert.equal(rule.getAttribute("role"), "none");
    assert.equal(rule.getAttribute("aria-hidden"), "true");
    choices(element)[2]!.click();
    assert.deepEqual(edits, [], "past its max");
    assert.equal(field(element), open, "the field stays open");
    assert.equal(open.textContent, "99");
    assert.equal(open.getAttribute("aria-invalid"), "true");
    assert.equal(open.getAttribute("aria-expanded"), "false");
    assert.equal(open.getAttribute("aria-controls"), null);
    press(open, "ArrowDown", { altKey: true });
    choices(element)[1]!.click();
    assert.deepEqual(edits, [{ key: "card.value.count.value", edit: { kind: "setValue", value: { kind: "number", text: "5" } } }]);
  });

  it("shows its chevron under the pointer in the selected row, which opens the field and its list", async () => {
    const element = show(await fixture("adding"));
    const over = (target: Element) => target.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    element.selection = "base.value.tone";
    over(rendered(element, "base.value.tone.value"));
    const shown = tools(element)!;
    assert.ok(shown.classList.contains("with-choices"), "a string's choices");
    assert.ok(!shown.classList.contains("with-steppers"));
    element.selection = "base.value.size";
    over(rendered(element, "base.value.size.value"));
    assert.ok(tools(element)!.classList.contains("in-line"), "taking room in the line, so tone, which follows, moves along");
    element.selection = "big.value.size";
    over(rendered(element, "big.value.size.value"));
    assert.ok(tools(element)!.classList.contains("with-choices"), "a number's choices");
    assert.ok(tools(element)!.classList.contains("with-steppers"), "and its steppers");
    pressTool(tools(element)!.querySelector(".choices-toggle")!);
    assert.ok(field(element), "the field opens");
    assert.deepEqual(shownChoices(element), ["Default 16", "sm 12", "base 16", "lg 20"], "with its list");
  });

  it("makes room in the line for an open field's tools when more of the line follows", async (t) => {
    const element = show(await fixture("adding"));
    rendered(element, "card.value.count.value").click();
    assert.ok(tools(element)!.classList.contains("in-line"), "mode follows count on the card's line");
    press(field(element)!, "Escape");
    element.selection = "big.value.size";
    assert.ok(tools(element)!.classList.contains("in-line"), "before the line's closing mark alone");
    narrow(t, 30);
    const sheet = show(await fixture("adding"));
    rendered(sheet, "card.value.count.value").click();
    assert.ok(!tools(sheet)!.classList.contains("in-line"), "nothing follows in a sheet's row: drawn over the gap");
  });

  it("shows no tools under the pointer, only for the selected property", async () => {
    const over = (target: Element) => target.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    const element = show(await fixture("adding"));
    over(rendered(element, "big.value.size.value"));
    assert.equal(tools(element), null, "a pointer over a value moves nothing");
    element.selection = "big.value.size";
    assert.ok(tools(element)!.classList.contains("in-line"), "selected, they take room in the line");
    over(rendered(element, "base.value.size.value"));
    assert.equal(tools(element)?.dataset["toolsFor"], "big.value.size.value", "and stay with the selection");
  });

  it("shows a case's chevron as the tools, taking room before />, which opens and closes its popover", async (t) => {
    const element = show(await fixture("adding"));
    const size = rendered(element, "latte.value.size.value");
    element.selection = "latte.value.size";
    const shown = tools(element)!;
    assert.ok(shown.classList.contains("with-choices") && !shown.classList.contains("with-steppers"));
    assert.ok(shown.classList.contains("in-line"), "room before the closing />");
    assert.equal(shown.previousElementSibling, size);
    pressTool(shown.querySelector(".choices-toggle")!);
    assert.ok(!popover(element).hidden, "the chevron opens the cases");
    assert.ok(tools(element)!.classList.contains("open"));
    pressTool(tools(element)!.querySelector(".choices-toggle")!);
    assert.ok(popover(element).hidden, "and closes them");
    assert.ok(tools(element)?.classList.contains("in-line"), "staying with the selection, its room kept");
    pressTool(tools(element)!.querySelector(".choices-toggle")!);
    assert.ok(!popover(element).hidden, "so the next press opens them again");
  });

  it("covers the gap rather than wrap a line that only just fits", async () => {
    const element = show(await fixture("adding"));
    const line = rendered(element, "big.value");
    assert.ok(line.matches(".line-element"));
    // The line wraps to two once the tools take room in it.
    line.getBoundingClientRect = () => {
      const height = line.querySelector(".value-tools.in-line") === null ? 18 : 36;
      return { x: 0, y: 0, top: 0, left: 0, right: 300, bottom: height, width: 300, height, toJSON: () => ({}) } as DOMRect;
    };
    element.selection = "big.value.size";
    const shown = tools(element)!;
    assert.ok(shown.classList.contains("with-steppers"));
    assert.ok(!shown.classList.contains("in-line"), "drawn over the gap");
  });

  it("shows an unset property's tools once its field opens, under a one-line element selected", async () => {
    const element = show(await fixture("adding"));
    element.selection = "plain.value";
    moreToggle(element)!.click();
    // size reads its quiet default past it, which the tools move aside.
    const size = rendered(element, "plain.value.size.value");
    size.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    assert.equal(tools(element), null, "not under the pointer");
    size.click();
    assert.ok(tools(element)?.classList.contains("with-steppers"), "with its field");
  });

  it("opens from its chevron, and picks with the pointer", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "base.value.size.value").click();
    pressTool(tools(element)!.querySelector(".choices-toggle")!);
    assert.equal(shownChoices(element).length, 4);
    assert.ok(tools(element)!.classList.contains("open"));
    choices(element)[1]!.click();
    assert.deepEqual(edits, [{ key: "base.value.size.value", edit: { kind: "setValue", value: { kind: "number", text: "12" } } }]);
  });

  it("resets to the default from its row, and adds nothing from it on a dimmed row", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "big.value.size.value").click();
    press(field(element)!, "ArrowDown", { altKey: true });
    choices(element)[0]!.click();
    assert.deepEqual(edits, [{ key: "big.value.size", edit: { kind: "remove" } }]);
    element.selection = "plain.value";
    moreToggle(element)!.click();
    rendered(element, "plain.value.size.value").click();
    press(field(element)!, "ArrowDown", { altKey: true });
    assert.ok(choices(element)[0]!.classList.contains("current"), "a dimmed row reads its default");
    choices(element)[0]!.click();
    assert.equal(edits.length, 1, "nothing to add");
    assert.equal(field(element), null);
  });

  it("filters as the reader types, and takes what is typed past them", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "base.value.size.value").click();
    const open = field(element)!;
    open.textContent = "1";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(shownChoices(element), ["Default 16", "sm 12", "base 16"], "typing opens the list, by value");
    open.textContent = "l";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(shownChoices(element), ["Default 16", "lg 20"], "and by label");
    open.textContent = "17";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(shownChoices(element), [], "none holds it");
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "base.value.size.value", edit: { kind: "setValue", value: { kind: "number", text: "17" } } }]);
  });

  it("closes the list with Escape before the field, and steps the number while it is closed", async () => {
    const element = show(await fixture("adding"));
    const { edits, lives } = events(element);
    rendered(element, "base.value.size.value").click();
    const open = field(element)!;
    press(open, "ArrowDown", { altKey: true });
    press(open, "Escape");
    assert.ok(popover(element).hidden);
    assert.equal(field(element), open, "the field stays open");
    press(open, "ArrowUp");
    assert.equal(open.textContent, "17");
    press(open, "Escape");
    assert.equal(field(element), null);
    // The step went live, and Escape takes it back.
    assert.deepEqual(lives, [{ key: "base.value.size.value", edit: { kind: "setValue", value: { kind: "number", text: "17" } }, live: true }]);
    assert.deepEqual(edits, [{ key: "base.value.size.value", edit: { kind: "setValue", value: { kind: "number", text: "16" } } }]);
  });

  it("lists a string's choices in one column when none has a label", async () => {
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "base.value.tone.value").click();
    press(field(element)!, "ArrowDown", { altKey: true });
    assert.deepEqual(shownChoices(element), ["warm", "cool"]);
    assert.equal(popover(element).querySelector(".choices-list.labeled"), null);
    assert.ok(choices(element)[0]!.classList.contains("current"));
    choices(element)[1]!.click();
    assert.deepEqual(edits, [{ key: "base.value.tone.value", edit: { kind: "setValue", value: { kind: "string", value: "cool" } } }]);
  });

  it("lists a color's choices over its field", async () => {
    const source = [
      'import "@nx/visualizer"',
      "type Swatch = string",
      'visualizer Swatch = <ValueVisualizer format=color><ValueChoice value="#c8a27a" label="tan" /><ValueChoice value="#5a3a22" label="brown" /></ValueVisualizer>',
      'type Cup = { color:Swatch = "#ffffff" }',
      'let cup = <Cup color="#c8a27a" />',
      ""
    ].join("\n");
    const element = show(await treeOf([{ uri: "nx:///cup.nx", identity: "cup.nx", source }], { visualizers: { modules: [] } }));
    const { edits } = events(element);
    rendered(element, "cup.value.color.value").click();
    assert.ok(popover(element).classList.contains("with-choices"));
    assert.deepEqual(shownChoices(element), ["Default #ffffff", "tan #c8a27a", "brown #5a3a22"]);
    assert.ok(choices(element)[1]!.querySelector(".swatch"), "each with its swatch");
    const hex = root(element).querySelector<HTMLInputElement>(".popover-hex")!;
    press(hex, "ArrowDown");
    press(hex, "ArrowDown");
    press(hex, "ArrowDown");
    press(hex, "Enter");
    assert.deepEqual(edits, [{ key: "cup.value.color.value", edit: { kind: "setValue", value: { kind: "string", value: "#5a3a22" } } }]);
  });
});

describe("steppers", () => {
  it("steps a number on a click, and ten steps on Shift+click, live at once and committed once the reader pauses", async (t) => {
    // As a sheet, where the steppers cover nothing past the value.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits, lives } = events(element);
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const value = rendered(element, "card.value.padding.value");
    element.selection = "card.value.padding";
    const shown = tools(element)!;
    assert.ok(shown !== null, "on the selected row");
    assert.ok(shown.classList.contains("with-steppers"));
    assert.ok(!shown.classList.contains("with-choices"));
    assert.equal(shown.previousElementSibling, value, "just past the value");
    const up = shown.querySelector<HTMLElement>(".stepper.up")!;
    assert.match(up.title, /\u2191/);
    assert.match(up.title, /Shift/);
    assert.match(up.title, /[Dd]rag/);
    pressTool(up);
    pressTool(up, { shiftKey: true });
    assert.deepEqual(lives, [
      { key: "card.value.padding.value", edit: { kind: "setValue", value: { kind: "number", text: "9" } }, live: true },
      { key: "card.value.padding.value", edit: { kind: "setValue", value: { kind: "number", text: "19" } }, live: true }
    ]);
    assert.deepEqual(edits, [], "not committed while the reader is still clicking");
    t.mock.timers.tick(499);
    assert.deepEqual(edits, []);
    t.mock.timers.tick(1);
    assert.deepEqual(edits, [{ key: "card.value.padding.value", edit: { kind: "setValue", value: { kind: "number", text: "19" } } }]);
    element.selection = "card.value";
    assert.equal(tools(element), null, "gone once another unit is selected");
  });

  it("takes a press that wanders a few pixels as a click, not a drag", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { lives } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    pressTool(tools(element)!.querySelector(".stepper.up")!, { from: 100, to: [102, 97] });
    assert.ok(!body(element).classList.contains("stepping"));
    assert.deepEqual(lives, [{ key: "card.value.gap.value", edit: { kind: "setValue", value: { kind: "number", text: "5" } }, live: true }]);
  });

  it("steps into an open field, which commits", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "card.value.count.value").click();
    const open = field(element)!;
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    assert.equal(open.textContent, "3");
    pressTool(tools(element)!.querySelector(".stepper.down")!, { shiftKey: true });
    assert.equal(open.textContent, "0", "kept at the least");
    assert.deepEqual(edits, []);
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "card.value.count.value", edit: { kind: "setValue", value: { kind: "number", text: "0" } } }]);
  });

  it("scrubs as a stepper is dragged up or down, shown in place and committed once on release", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    element.selection = "card.value.gap";
    const value = rendered(element, "card.value.gap.value");
    value.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    const up = tools(element)!.querySelector<HTMLElement>(".stepper.up")!;
    up.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true, cancelable: true, clientY: 100 }));
    up.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 94 }));
    up.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 74 }));
    assert.equal(rendered(element, "card.value.gap.value").textContent, "9", "shown as it moves");
    up.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 30 }));
    assert.equal(rendered(element, "card.value.gap.value").textContent, "16", "kept within the range");
    assert.deepEqual(edits, []);
    up.dispatchEvent(new window.MouseEvent("pointerup", { bubbles: true, composed: true, clientY: 30 }));
    assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: { kind: "setValue", value: { kind: "number", text: "16" } } }]);
    assert.equal(element.selection, "card.value.gap", "the selection stays");
  });
  it("keeps the steppers as the pointer crosses the gap to reach them", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { lives } = events(element);
    element.selection = "card.value.padding";
    const over = (target: Element) => target.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    const value = rendered(element, "card.value.padding.value");
    over(value);
    const shown = tools(element)!;
    over(value.closest(".sheet-value")!);
    assert.equal(tools(element), shown, "over the value's cell");
    over(value.closest(".sheet-row")!);
    assert.equal(tools(element), shown, "over the gap in the value's row");
    over(shown);
    pressTool(shown.querySelector(".stepper.up")!);
    assert.deepEqual(lives, [{ key: "card.value.padding.value", edit: { kind: "setValue", value: { kind: "number", text: "9" } }, live: true }]);
    over(rendered(element, "card.value.gap.value").closest(".sheet-row")!);
    assert.equal(tools(element)?.dataset["toolsFor"], "card.value.padding.value", "over another property, back with the selected one's value");
    element.selection = "card.value";
    assert.equal(tools(element), null, "and gone once it is not selected");
  });

  it("shows a selected property's tools with no pointer over it, inline too, and keeps them as the pointer moves", async () => {
    const element = show(await fixture("adding"));
    const over = (target: Element) => target.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    assert.equal(tools(element), null);
    const name = rendered(element, "base.value.size").querySelector<HTMLElement>(".line-name")!;
    name.click();
    assert.equal(element.selection, "base.value.size");
    assert.equal(tools(element)?.dataset["toolsFor"], "base.value.size.value", "the steppers, as soon as it is selected");
    assert.ok(field(element) === null, "and no editor opens");
    over(rendered(element, "big.value.size.value"));
    body(element).dispatchEvent(new window.MouseEvent("pointerleave"));
    assert.equal(tools(element)?.dataset["toolsFor"], "base.value.size.value", "the pointer moves nothing");
    rendered(element, "big.value.size.value").click();
    assert.equal(element.selection, "big.value.size", "a click on another value selects its property");
    press(field(element)!, "Escape");
    assert.equal(tools(element)?.dataset["toolsFor"], "big.value.size.value", "and closing its field leaves its tools");
  });

  it("shows a let's steppers, and takes room for them where more of the line follows", async () => {
    const element = show(await fixture("adding"));
    element.selection = "pad";
    const shown = tools(element)!;
    assert.ok(shown.classList.contains("with-steppers"), "a let selected shows its value's");

    const edges = show(await fixture("editingEdges"));
    edges.selection = "box.value.depth";
    assert.ok(tools(edges)!.classList.contains("in-line"), "in the line, before the rest of a one-line element's line");
    edges.selection = "box.value.items.value[0]";
    assert.ok(tools(edges)!.classList.contains("in-line"), "and before the next item");
  });

  it("keeps the steppers under the pointer through the tree a click brings, and through a cleared one", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { lives } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    assert.equal(lives.length, 1);
    const tree = element.tree;
    element.tree = undefined;
    element.tree = tree;
    assert.ok(tools(element), "still there for the next click, not the row under them");
    assert.equal(element.selection, "card.value.gap");
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    assert.equal(lives.length, 2, "the next click steps again");
    assert.deepEqual(lives[1]!.edit, { kind: "setValue", value: { kind: "number", text: "6" } }, "on from the number the last click showed");
  });

  it("carries a drag on through a new tree, and ends it cleanly when the reading goes", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    const up = tools(element)!.querySelector<HTMLElement>(".stepper.up")!;
    up.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true, cancelable: true, clientY: 100 }));
    up.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 94 }));
    up.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 54 }));
    assert.equal(rendered(element, "card.value.gap.value").textContent, "14");
    element.tree = element.tree;
    assert.equal(rendered(element, "card.value.gap.value").textContent, "14", "still shown");
    assert.ok(body(element).classList.contains("stepping"));
    const again = tools(element)!.querySelector<HTMLElement>(".stepper.up")!;
    again.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 46 }));
    again.dispatchEvent(new window.MouseEvent("pointerup", { bubbles: true, composed: true, clientY: 46 }));
    assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: { kind: "setValue", value: { kind: "number", text: "16" } } }]);
    assert.ok(!body(element).classList.contains("stepping"));

    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    const down = tools(element)!.querySelector<HTMLElement>(".stepper.down")!;
    down.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true, cancelable: true, clientY: 100 }));
    down.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 110 }));
    assert.ok(body(element).classList.contains("stepping"));
    element.remove();
    assert.ok(!body(element).classList.contains("stepping"), "nothing is left resizing");
  });

  it("carries a drag on through a cleared tree and the next, as a host gives a live edit's", async (t) => {
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits, lives } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    const up = tools(element)!.querySelector<HTMLElement>(".stepper.up")!;
    up.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true, cancelable: true, clientY: 100 }));
    up.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 94 }));
    up.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 74 }));
    assert.ok(lives.length > 0, "the drag went live");
    const tree = element.tree;
    element.tree = undefined;
    element.tree = tree;
    const again = tools(element)!.querySelector<HTMLElement>(".stepper.up")!;
    again.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true, composed: true, clientY: 54 }));
    assert.equal(rendered(element, "card.value.gap.value").textContent, "14", "still stepping");
    again.dispatchEvent(new window.MouseEvent("pointerup", { bubbles: true, composed: true, clientY: 54 }));
    assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: { kind: "setValue", value: { kind: "number", text: "14" } } }]);
  });

  it("shows the value as before when a drag ends where it began", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits, lives } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    pressTool(tools(element)!.querySelector(".stepper.up")!, { from: 100, to: [94, 84, 94] });
    // The drag went live, and ending where it began takes it back.
    assert.ok(lives.length > 0);
    assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: { kind: "setValue", value: { kind: "number", text: "4" } } }]);
    const value = rendered(element, "card.value.gap.value");
    assert.equal(value.textContent, "4");
    assert.ok(!value.classList.contains("pending"), "nothing is waiting");
  });

  it("fills an open field from a drag, which commits", async (t) => {
    // As a sheet, where the steppers cover nothing under the pointer.
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    rendered(element, "card.value.count.value").click();
    const open = field(element)!;
    pressTool(tools(element)!.querySelector(".stepper.up")!, { from: 100, to: [94, 82] });
    assert.equal(open.textContent, "5");
    assert.equal(field(element), open);
    assert.deepEqual(edits, []);
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "card.value.count.value", edit: { kind: "setValue", value: { kind: "number", text: "5" } } }]);
  });
});

describe("bindings and other elements", () => {
  it("edits no reference, to a named value, a parameter or a loop variable", async () => {
    const { tree, text } = await fixture("bindings");
    const targets = editTargets(indexTree(tree, text));
    assert.equal(targets.get("banded.value.color.value"), undefined, "a let's member");
    assert.equal(targets.get("tinted.value.color.value"), undefined, "a let");
    assert.equal(targets.get("Special.body.name.value"), undefined, "a component's parameter");
    assert.equal(targets.get("menu.body[0].size.value"), undefined, "a loop variable");
  });

  it("opens Source for a binding or a reference, never an editor", async () => {
    const element = show(await fixture("bindings"));
    const { edits, opens } = events(element);
    const name = rendered(element, "Special.body.name.value");
    assert.ok(!name.classList.contains("editable-value"));
    name.dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    const tint = rendered(element, "tinted.value.color.value");
    assert.ok(!tint.classList.contains("editable-value"));
    tint.dispatchEvent(new window.MouseEvent("dblclick", { bubbles: true, composed: true }));
    assert.deepEqual(opens.map((open) => open.key), ["Special.body.name.value", "tinted.value.color.value"]);
    assert.deepEqual(edits, []);
  });

  it("edits every integer and float type NX has, and its one boolean", async () => {
    const { tree, text } = await fixture("bindings");
    const index = indexTree(tree, text);
    assert.equal(editTargets(index).get("banded.value.big.value")?.kind, "number", "an int64");
    const listed = unwrittenProperties(index, index.byKey.get("banded.value")!).map(
      (entry) => `${entry.property.name}:${entry.editor?.kind ?? "-"}`
    );
    assert.deepEqual(listed, ["size:case", "few:number", "iced:boolean", "extra:case"]);
  });

  it("counts a property written under a condition as written", async () => {
    const { tree, text } = await fixture("bindings");
    const index = indexTree(tree, text);
    const names = unwrittenProperties(index, index.byKey.get("sometimes.value")!).map((entry) => entry.property.name);
    assert.ok(!names.includes("iced"), names.join(" "));
  });

  it("reads a value under a condition with its property's format and card", async () => {
    const { tree, text } = await fixture("bindings");
    const index = indexTree(tree, text);
    assert.equal(editTargets(index).get("tagged.value.if[0].then.tint.value")?.kind, "color");
    assert.equal(editTargets(index).get("tagged.value.if[0].then.label.value")?.kind, "text");
    const card = cardOf(index, index.byKey.get("tagged.value.if[0].then.tint")!);
    assert.equal(card?.signature, "(property) tint?: string");
  });

  it("offers a union case's element the properties it does not write", async () => {
    const element = show(await fixture("bindings"));
    const { edits } = events(element);
    element.selection = "promo.value";
    assert.equal(moreToggle(element)?.textContent, "Show 1 unset property");
    moreToggle(element)!.click();
    assert.deepEqual(moreRows(element).map(reads), ["pumps 1 default"]);
    rendered(element, "promo.value.pumps.value").click();
    const open = field(element)!;
    open.textContent = "2";
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "promo.value", edit: { kind: "addProperty", name: "pumps", value: { kind: "number", text: "2" } } }]);
  });

  it("bounds nothing with an empty range", () => {
    const hints = { min: 10, max: 0 };
    assert.equal(rangeProblem("5", hints), undefined);
    assert.equal(rangeProblem("20", hints), undefined);
  });
});

describe("live edits", () => {
  const cup = async () => {
    const source = [
      'import "@nx/visualizer"',
      "type Swatch = string",
      "visualizer Swatch = <ValueVisualizer format=color />",
      'type Cup = { color:Swatch = "#ffffff" }',
      'let cup = <Cup color="#c8a27a" />',
      ""
    ].join("\n");
    return treeOf([{ uri: "nx:///cup.nx", identity: "cup.nx", source }], { visualizers: { modules: [] } });
  };
  const color = (value: string) => ({ kind: "setValue", value: { kind: "string", value } });
  const number = (text: string) => ({ kind: "setValue", value: { kind: "number", text } });

  it("goes live with each step of an open field's steppers, and commits on Enter", async () => {
    const element = show(await fixture("adding"));
    const { edits, lives } = events(element);
    rendered(element, "base.value.size.value").click();
    const open = field(element)!;
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    assert.deepEqual(lives.map((detail) => detail.edit), [number("17"), number("18")]);
    assert.ok(lives.every((detail) => detail.live === true && detail.key === "base.value.size.value"));
    assert.deepEqual(edits, []);
    press(open, "Enter");
    assert.deepEqual(edits, [{ key: "base.value.size.value", edit: number("18") }]);
  });

  it("goes live with a typed entry once typing pauses, and never with one it cannot take", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const element = show(await fixture("adding"));
    const { lives } = events(element);
    rendered(element, "base.value.size.value").click();
    const open = field(element)!;
    for (const text of ["2", "24"]) {
      open.textContent = text;
      open.dispatchEvent(new window.Event("input", { bubbles: true }));
      t.mock.timers.tick(100);
    }
    assert.equal(lives.length, 0, "not while typing");
    t.mock.timers.tick(200);
    assert.deepEqual(lives.map((detail) => detail.edit), [number("24")]);
    open.textContent = "24px";
    open.dispatchEvent(new window.Event("input", { bubbles: true }));
    t.mock.timers.tick(1000);
    assert.equal(lives.length, 1, "an entry it cannot take waits");
  });

  it("goes live as the color picker moves, and takes it back on Escape", async () => {
    const element = show(await cup());
    const { edits, lives } = events(element);
    rendered(element, "cup.value.color.value").click();
    const picker = popover(element).querySelector<HTMLInputElement>(".popover-picker")!;
    for (const value of ["#112233", "#445566"]) {
      picker.value = value;
      picker.dispatchEvent(new window.Event("input", { bubbles: true }));
    }
    assert.deepEqual(lives.map((detail) => detail.edit), [color("#112233"), color("#445566")]);
    press(popover(element).querySelector(".popover-hex")!, "Escape");
    assert.deepEqual(edits, [{ key: "cup.value.color.value", edit: color("#c8a27a") }]);
  });

  it("keeps the color popover's picker in the page through the trees its live edits bring", async () => {
    const element = show(await cup());
    const { edits, lives } = events(element);
    rendered(element, "cup.value.color.value").click();
    const picker = popover(element).querySelector<HTMLInputElement>(".popover-picker")!;
    picker.value = "#112233";
    picker.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.equal(lives.length, 1);
    const tree = element.tree;
    element.tree = undefined;
    element.tree = tree;
    assert.ok(!popover(element).hidden, "still open");
    assert.equal(popover(element).querySelector(".popover-picker"), picker, "the same picker, which the browser keeps open");
    assert.ok(picker.isConnected);
    picker.value = "#445566";
    picker.dispatchEvent(new window.Event("input", { bubbles: true }));
    assert.deepEqual(lives.map((detail) => detail.edit), [color("#112233"), color("#445566")], "each move once");
    picker.dispatchEvent(new window.Event("change", { bubbles: true }));
    assert.deepEqual(edits, [{ key: "cup.value.color.value", edit: color("#445566") }]);
  });

  it("commits a color when its popover is left for something else", async () => {
    const element = show(await cup());
    const { edits } = events(element);
    rendered(element, "cup.value.color.value").click();
    const picker = popover(element).querySelector<HTMLInputElement>(".popover-picker")!;
    picker.value = "#112233";
    picker.dispatchEvent(new window.Event("input", { bubbles: true }));
    document.body.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true }));
    assert.deepEqual(edits, [{ key: "cup.value.color.value", edit: color("#112233") }]);
    assert.ok(popover(element).hidden);
  });

  it("commits stepper clicks made with the field closed when the reader selects something else", async (t) => {
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits, lives } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    assert.equal(lives.length, 1);
    rendered(element, "card.value.padding").querySelector<HTMLElement>(".sheet-name")!.click();
    assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: number("5") }]);
  });

  it("commits stepper clicks made with the field closed when the reader opens that field", async (t) => {
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    rendered(element, "card.value.gap.value").click();
    assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: number("5") }]);
    assert.equal(field(element)?.textContent, "5", "the field opens on the value the clicks left");
  });

  it("commits stepper clicks made with the field closed when the element leaves the page", async (t) => {
    narrow(t, 30);
    const element = show(await fixture("adding"));
    const { edits } = events(element);
    element.selection = "card.value.gap";
    rendered(element, "card.value.gap.value").dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
    pressTool(tools(element)!.querySelector(".stepper.up")!);
    element.remove();
    assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: number("5") }]);
  });
});

describe("selection as editors open, and gestures the reader moves on from", () => {
  it("keeps an element selected, and its unset list open, when a value it lists opens", async () => {
    const element = show(await fixture("adding"));
    element.selection = "base.value";
    moreToggle(element)!.click();
    const listed = moreRows(element).length;
    assert.ok(listed > 0);
    rendered(element, "base.value.size.value").click();
    assert.ok(field(element), "the field opens");
    assert.equal(element.selection, "base.value", "the element stays selected");
    assert.equal(moreRows(element).length, listed, "and its list stays");
  });

  it("keeps a unit selected when Enter opens its first value, and Escape leaves it selected", async () => {
    const element = show(await fixture("adding"));
    element.selection = "latte.value";
    press(body(element), "Enter");
    const open = field(element) ?? root(element).querySelector<HTMLElement>(".popover:not([hidden]) input");
    assert.ok(open, "an editor opens");
    assert.equal(element.selection, "latte.value");
    press(open, "Escape");
    assert.equal(element.selection, "latte.value", "Escape keeps the selection");
  });

  it("commits stepper clicks made with the field closed on a press elsewhere, and on the host's undo keys", async (t) => {
    narrow(t, 30);
    for (const how of ["press", "keys"] as const) {
      const element = show(await fixture("adding"));
      const { edits } = events(element);
      element.selection = "card.value.gap";
      pressTool(tools(element)!.querySelector(".stepper.up")!);
      assert.deepEqual(edits, [], "settling");
      if (how === "press") {
        document.body.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true }));
      } else {
        press(body(element), "z", { ctrlKey: true });
      }
      assert.deepEqual(edits, [{ key: "card.value.gap.value", edit: { kind: "setValue", value: { kind: "number", text: "5" } } }], how);
    }
  });

  it("keeps a double-click's word selected", async (t) => {
    const selection = fakeSelection(t);
    const element = show(await fixture("adding"));
    const name = rendered(element, "base.value.size").querySelector<HTMLElement>(".line-name")!;
    selection.selectAllChildren(name);
    name.dispatchEvent(new window.MouseEvent("click", { bubbles: true, composed: true, detail: 2 }));
    assert.equal(selection.isCollapsed, false, "the word stays selected");
    assert.notEqual(element.selection, "base.value.size", "and nothing is chosen");
  });

  it("hangs a selected inline property's mark over the gap where its room would wrap the line", async () => {
    const element = show(await fixture("adding"));
    const line = rendered(element, "big.value");
    line.getBoundingClientRect = () => {
      const wraps = line.querySelector(".property-menu:not(.over-gap) > .row-menu.selected") !== null;
      const height = wraps ? 36 : 18;
      return { x: 0, y: 0, top: 0, left: 0, right: 300, bottom: height, width: 300, height, toJSON: () => ({}) } as DOMRect;
    };
    element.selection = "big.value.size";
    const anchor = rendered(element, "big.value.size").nextElementSibling as HTMLElement;
    assert.ok(anchor.matches(".property-menu"));
    assert.ok(anchor.classList.contains("over-gap"), "no room taken");
  });

  it("moves a selected inline property's mark past tools drawn over the gap without taking room", async () => {
    const element = show(await fixture("adding"));
    element.selection = "base.value.size";
    const shown = tools(element)!;
    const line = rendered(element, "big.value");
    // The line only just fits: tools in it, or a mark taking room, would wrap it.
    line.getBoundingClientRect = () => {
      const wraps = line.querySelector(".value-tools.in-line") !== null || line.querySelector(".property-menu:not(.over-gap) > .row-menu.selected") !== null;
      const height = wraps ? 36 : 18;
      return { x: 0, y: 0, top: 0, left: 0, right: 300, bottom: height, width: 300, height, toJSON: () => ({}) } as DOMRect;
    };
    shown.getBoundingClientRect = () => ({ x: 100, y: 0, top: 0, left: 100, right: 130, bottom: 18, width: 30, height: 18, toJSON: () => ({}) }) as DOMRect;
    const anchor = rendered(element, "big.value.size").nextElementSibling as HTMLElement;
    anchor.getBoundingClientRect = () => ({ x: 104, y: 0, top: 0, left: 104, right: 104, bottom: 18, width: 0, height: 18, toJSON: () => ({}) }) as DOMRect;
    element.selection = "big.value.size";
    assert.ok(!tools(element)!.classList.contains("in-line"), "the tools cover the gap");
    assert.equal(anchor.style.marginLeft, "", "the mark takes no room");
    assert.match(anchor.style.transform, /translateX\(26px\)/, "and is moved past the tools");
  });

  it("keeps the selected unit when Space toggles its boolean", async () => {
    const element = show(await fixture("editing"));
    element.selection = "latte.value";
    press(body(element), " ");
    assert.equal(element.selection, "latte.value");
  });
});

describe("typed color editors", () => {
  const fromSource = (source: string) => treeOf([{ uri: "nx:///typed-colors.nx", identity: "typed-colors.nx", source }], { visualizers: { modules: [] } });
  it("does not quantize or emit edits when a native picker is opened and dismissed", async () => {
    const item = await fromSource("let accent = #f008\n");
    const node = item.tree.nodes.find(node => node.value === "#f008")!;
    const element = show(item);
    const { edits, lives } = events(element);
    rendered(element,node.key).click();
    assert.ok(popover(element).querySelector(".popover-alpha"));
    press(popover(element).querySelector(".popover-hex")!,"Escape");
    assert.deepEqual(edits,[]);
    assert.deepEqual(lives,[]);
  });
  it("commits native alpha explicitly and leaves computed colors read-only", async () => {
    const item = await fromSource('import "@nx/color" as c\nlet accent = #33669980\nlet calculated = { c.withAlpha(accent,0.5) }\n');
    const node = item.tree.nodes.find(node => node.value === "#33669980")!;
    const element = show(item);
    const { edits } = events(element);
    rendered(element,node.key).click();
    const alpha=popover(element).querySelector<HTMLInputElement>(".popover-alpha")!;
    alpha.value="0.25";
    alpha.dispatchEvent(new window.Event("change",{bubbles:true}));
    assert.deepEqual(edits,[{key:node.key,edit:{kind:"setValue",value:{kind:"color",text:"#33669940"}}}]);
    const targets=editTargets(indexTree(item.tree,item.text));
    const computed=item.tree.nodes.find(node=>node.role==="call")!;
    assert.equal(targets.has(computed.key),false);
  });
  it("keeps legacy CSS string color editors free of the typed alpha control", async () => {
    const item=await fromSource('import "@nx/visualizer"\ntype Swatch = string\nvisualizer Swatch = <ValueVisualizer format=color/>\nlet accent:Swatch = "rgba(0, 0, 0, 0.5)"\n');
    const node=item.tree.nodes.find(node=>node.role==="literal" && node.value?.includes("rgba"))!;
    const element=show(item);
    const { edits }=events(element);
    rendered(element,node.key).click();
    assert.equal(popover(element).querySelector(".popover-alpha"),null);
    assert.equal(popover(element).querySelector<HTMLInputElement>(".popover-hex")!.value,"rgba(0, 0, 0, 0.5)");
    press(popover(element).querySelector(".popover-hex")!,"Escape");
    assert.deepEqual(edits,[]);
  });
});

describe("color editor revisions", () => {
  it("ignores a detached alpha control after the document tree changes", async () => {
    const before=await treeOf([{uri:"nx:///revision.nx",identity:"revision.nx",source:"let accent = #33669980\n"}]);
    const element=show(before);
    const {edits}=events(element);
    const node=before.tree.nodes.find(node=>node.value==="#33669980")!;
    rendered(element,node.key).click();
    const old=popover(element).querySelector<HTMLInputElement>(".popover-alpha")!;
    const after=await treeOf([{uri:"nx:///revision.nx",identity:"revision.nx",source:"let accent = #abcdef80\n"}]);
    element.tree=undefined;
    element.text=after.text;
    element.tree=after.tree;
    old.value="0";
    old.dispatchEvent(new window.Event("change",{bubbles:true}));
    assert.deepEqual(edits,[]);
  });
});
