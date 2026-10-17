import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it, type TestContext } from "node:test";

import type { SourceNode, SourceTree } from "@nx-lang/language-protocol";

import { fixture, nxHost, type Fixture } from "./fixtures.js";
import type { NxViewerElement } from "../src/index.js";

const { NxViewerElement: ViewerClass } = await import("../src/index.js");

const window = document.defaultView!;
const created: NxViewerElement[] = [];

after(async () => {
  for (const element of created) {
    element.remove();
  }
  (await nxHost()).dispose();
});

/** The element showing `fixture` in `parent`, everything unfolded, with an icon for every element. */
function show({ tree, text }: { tree: SourceTree; text: string }, parent: Node = document.body): NxViewerElement {
  const element = document.createElement("nx-viewer");
  parent.appendChild(element);
  element.folded = [];
  element.icons = () => document.createElementNS("http://www.w3.org/2000/svg", "svg");
  element.text = text;
  element.tree = tree;
  created.push(element);
  return element;
}

function root(element: NxViewerElement): ShadowRoot {
  return element.shadowRoot!;
}

/** The key of the first node of `fixture`'s `root` that `test` picks. */
function keyIn(fixture: Fixture, test: (node: SourceNode) => boolean): string {
  const node = fixture.tree.nodes.find((candidate) => candidate.key.startsWith("root.") && test(candidate));
  assert.ok(node, "the fixture has the node the test looks for");
  return node.key;
}

/** The node drawn with `key`. */
function drawn(element: NxViewerElement, key: string): HTMLElement {
  const found = [...root(element).querySelectorAll<HTMLElement>("[data-key]")].find((node) => node.dataset["key"] === key);
  assert.ok(found, `the node ${key} is drawn`);
  return found;
}

/** What marks the card of `key`, as the reading draws them. */
function marksOf(element: NxViewerElement, key: string): HTMLElement[] {
  return [...root(element).querySelectorAll<HTMLElement>("[data-hover]")].filter((node) => node.dataset["hover"] === key);
}

function card(element: NxViewerElement): HTMLElement {
  return root(element).querySelector<HTMLElement>(".card")!;
}

function body(element: NxViewerElement): HTMLElement {
  return root(element).querySelector<HTMLElement>(".body")!;
}

/** Moves the pointer onto `target`. */
function pointAt(target: Element): void {
  target.dispatchEvent(new window.MouseEvent("pointerover", { bubbles: true, composed: true }));
}

function press(target: Element, key: string): void {
  target.dispatchEvent(new window.KeyboardEvent("keydown", { key, bubbles: true, composed: true }));
}

/**
 * Lays the reading out as jsdom does not, for the test `t`: the element and its frame fill 400 by
 * 300 pixels, the card is 200 by 60, and each node is a line 18 pixels high at `place(node)`.
 */
function layOut(t: TestContext, element: NxViewerElement, place: (node: HTMLElement) => { top: number; left: number }): void {
  const prototype = window.HTMLElement.prototype;
  const { getBoundingClientRect } = prototype;
  const rect = (left: number, top: number, width: number, height: number): DOMRect =>
    ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) }) as DOMRect;
  prototype.getBoundingClientRect = function (this: HTMLElement) {
    if (this === element || this.classList.contains("frame")) {
      return rect(0, 0, 400, 300);
    }
    if (this.classList.contains("card")) {
      return rect(0, 0, 200, 60);
    }
    const { top, left } = place(this);
    return rect(left, top, 80, 18);
  };
  t.after(() => {
    prototype.getBoundingClientRect = getBoundingClientRect;
  });
}

/**
 * Lays the reading out for the test `t` so a line or a group's header of more than `columns`
 * characters takes two lines, and each element gives each property a row.
 */
function narrow(t: TestContext, columns: number): void {
  const prototype = window.HTMLElement.prototype;
  const { getClientRects } = prototype;
  Object.defineProperty(prototype, "clientWidth", { configurable: true, get: () => columns * 8 });
  prototype.getClientRects = function (this: HTMLElement) {
    const height = (this.textContent ?? "").replace(/\s+/g, " ").length > columns ? 36 : 18;
    return [{ top: 0, bottom: height, left: 0, right: 100, width: 100, height, x: 0, y: 0, toJSON: () => ({}) }] as unknown as DOMRectList;
  };
  t.after(() => {
    prototype.getClientRects = getClientRects;
    delete (prototype as { clientWidth?: number }).clientWidth;
  });
}

const firstShape = (node: SourceNode): boolean => node.role === "element" && node.name === "Shape";

function signatureOf(element: NxViewerElement): string | undefined {
  return card(element).hidden ? undefined : (card(element).querySelector(".card-signature")?.textContent ?? undefined);
}

describe("hover targets", () => {
  it("are an element's name and icon, a property's name, an id or a title, and a value", async () => {
    const hover = await fixture("hover");
    const element = show(hover);
    const shape = keyIn(hover, firstShape);
    const line = drawn(element, shape);
    assert.deepEqual(
      marksOf(element, shape).map((mark) => mark.className),
      ["element-icon", "line-kind"],
      "the element's icon and name"
    );
    assert.equal(line.querySelector(".element-icon")!.hasAttribute("title"), false, "the card names it, not a tooltip");
    const type = keyIn(hover, (node) => node.role === "attribute" && node.name === "Type");
    assert.deepEqual(marksOf(element, type).map((mark) => mark.className), ["line-name", "line-equals"], "a line's name=");
    const circle = keyIn(hover, (node) => node.role === "case" && node.name === "Circle");
    assert.deepEqual(marksOf(element, circle).map((mark) => mark.dataset["key"]), [circle], "a case marks itself");
    for (const value of [
      keyIn(hover, (node) => node.role === "case" && node.name === "Rectangle"),
      keyIn(hover, (node) => node.role === "member"),
      keyIn(hover, (node) => node.role === "call"),
      keyIn(hover, (node) => node.role === "reference" && node.name === "size"),
    ]) {
      assert.equal(drawn(element, value).dataset["hover"], value, `${value} is a value with a card`);
    }
    const union = keyIn(hover, (node) => node.role === "reference" && node.name === "ShapeType");
    assert.equal(drawn(element, union).dataset["hover"], undefined, "a qualified case's union is the case's");
    // A task's id and title read without their names, and show their properties' cards.
    const id = keyIn(hover, (node) => node.role === "attribute" && node.name === "id");
    const title = keyIn(hover, (node) => node.role === "attribute" && node.name === "title");
    assert.deepEqual(marksOf(element, id).map((mark) => mark.textContent), ["ship"]);
    assert.deepEqual(marksOf(element, title).map((mark) => mark.textContent), ["Ship it"]);
    // Nothing else is marked: a literal, a comment, or a unit's room.
    const hi = keyIn(hover, (node) => node.role === "literal" && node.value === '"hi"');
    const note = keyIn(hover, (node) => node.role === "comment");
    assert.equal(drawn(element, hi).closest("[data-hover]"), null);
    assert.equal(drawn(element, note).closest("[data-hover]"), null);
    assert.equal(root(element).querySelectorAll(":is(.line-element, .sheet, .sheet-row, .sheet-head)[data-hover]").length, 0);
  });

  it("are a group's name and its rows' names", async (t) => {
    narrow(t, 30);
    const wrapped = await fixture("wrappedAttributes");
    const element = show(wrapped);
    const sheet = root(element).querySelector<HTMLElement>(".sheet")!;
    assert.equal(sheet.querySelector(":scope > .sheet-head > .sheet-kind")!.getAttribute("data-hover"), sheet.dataset["key"]);
    const rows = [...sheet.querySelectorAll<HTMLElement>(":scope > .sheet-row")];
    assert.ok(rows.length > 0);
    for (const row of rows) {
      assert.equal(row.querySelector(".sheet-name")!.getAttribute("data-hover"), row.dataset["key"]);
    }
  });
});

describe("hover cards", () => {
  it("opens an element's declaration once the pointer rests on its name, and moves nothing", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const shape = keyIn(hover, firstShape);
    const name = drawn(element, shape).querySelector(".line-kind")!;
    layOut(t, element, (node) => (node === name ? { top: 100, left: 50 } : { top: 0, left: 0 }));
    const folded = element.folded;
    pointAt(name);
    t.mock.timers.tick(499);
    assert.equal(card(element).hidden, true, "not yet: the pointer may be passing");
    t.mock.timers.tick(1);
    assert.equal(card(element).hidden, false);
    assert.equal(card(element).querySelector(".card-signature")!.textContent, "component Shape extends Node");
    assert.equal(card(element).querySelector(".card-doc")!.textContent, "A filled shape.");
    assert.equal(card(element).style.top, "122px", "below what it describes");
    assert.equal(card(element).style.left, "50px");
    assert.equal(element.selection, undefined, "the selection is as it was");
    assert.deepEqual(element.folded, folded, "and so are the folds");
    assert.ok(!body(element).contains(card(element)), "the card is outside what scrolls the reading");
  });

  it("opens above what it describes where there is no room below, inside the element's box", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const name = drawn(element, keyIn(hover, firstShape)).querySelector(".line-kind")!;
    layOut(t, element, (node) => (node === name ? { top: 260, left: 350 } : { top: 0, left: 0 }));
    pointAt(name);
    t.mock.timers.tick(500);
    assert.equal(card(element).style.top, "196px");
    assert.equal(card(element).style.left, "200px", "kept from running past the element's right edge");
  });

  it("stays while the pointer moves onto it, and closes once the pointer has left both", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const type = keyIn(hover, (node) => node.role === "attribute" && node.name === "Type");
    pointAt(marksOf(element, type)[0]!);
    t.mock.timers.tick(500);
    assert.equal(card(element).querySelector(".card-signature")!.textContent, "(property) Type: ShapeType = Rectangle");
    body(element).dispatchEvent(new window.MouseEvent("pointerleave"));
    t.mock.timers.tick(200);
    card(element).dispatchEvent(new window.MouseEvent("pointerenter"));
    t.mock.timers.tick(1000);
    assert.equal(card(element).hidden, false, "on the card, it stays");
    card(element).dispatchEvent(new window.MouseEvent("pointerleave"));
    t.mock.timers.tick(299);
    assert.equal(card(element).hidden, false, "a moment's grace");
    t.mock.timers.tick(1);
    assert.equal(card(element).hidden, true, "then it closes");
  });

  it("opens nothing over a literal, a comment or a unit's room", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const hi = drawn(element, keyIn(hover, (node) => node.role === "literal" && node.value === '"hi"'));
    const note = drawn(element, keyIn(hover, (node) => node.role === "comment"));
    const room = drawn(element, keyIn(hover, firstShape)).querySelector(".line-gap")!;
    for (const target of [hi, note, room]) {
      pointAt(target);
      t.mock.timers.tick(1000);
      assert.equal(card(element).hidden, true, `nothing over ${target.textContent}`);
    }
  });

  it("closes on Escape, on a scroll and on a new text", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const circle = keyIn(hover, (node) => node.role === "case" && node.name === "Circle");
    const open = (): void => {
      pointAt(document.body);
      body(element).dispatchEvent(new window.MouseEvent("pointerleave"));
      pointAt(drawn(element, circle));
      t.mock.timers.tick(500);
      assert.equal(card(element).hidden, false);
    };
    open();
    assert.equal(card(element).querySelector(".card-signature")!.textContent, "(case) ShapeType.Circle");
    press(body(element), "Escape");
    assert.equal(card(element).hidden, true, "Escape");
    open();
    document.dispatchEvent(new window.Event("scroll"));
    assert.equal(card(element).hidden, true, "a scroll");
    open();
    element.text = `${hover.text}\n`;
    assert.equal(card(element).hidden, true, "a new text");
  });

  it("shows the selected unit's card on i and says it, and Escape closes it, the selection kept", async (t) => {
    narrow(t, 10);
    const hover = await fixture("hover");
    const element = show(hover);
    const type = keyIn(hover, (node) => node.role === "attribute" && node.name === "Type");
    element.selection = type;
    assert.ok(drawn(element, type).classList.contains("sheet-row"), "the property has a row");
    press(body(element), "i");
    assert.equal(card(element).hidden, false);
    assert.equal(card(element).querySelector(".card-signature")!.textContent, "(property) Type: ShapeType = Rectangle");
    assert.equal(
      root(element).querySelector('[role="status"]')!.textContent,
      "(property) Type: ShapeType = Rectangle. What the shape draws."
    );
    press(body(element), "Escape");
    assert.equal(card(element).hidden, true);
    assert.equal(element.selection, type, "the row is still selected");
  });

  it("renders a doc with the host's markdown, set before the element is defined too", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const markdown = (doc: string): Node => {
      const bold = document.createElement("b");
      bold.textContent = doc.toUpperCase();
      return bold;
    };
    const tag = "nx-viewer-hover-late";
    const element = document.createElement(tag) as NxViewerElement;
    document.body.append(element);
    Object.assign(element, { folded: [], text: hover.text, tree: hover.tree, markdown });
    customElements.define(tag, class extends ViewerClass {});
    created.push(element);
    assert.equal(element.markdown, markdown);
    pointAt(drawn(element, keyIn(hover, firstShape)).querySelector(".line-kind")!);
    t.mock.timers.tick(500);
    assert.equal(card(element).querySelector(".card-doc > b")!.textContent, "A FILLED SHAPE.");
  });

  it("shows a doc's markup as text and makes no link that runs script", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const tree: SourceTree = structuredClone(hover.tree);
    const shape = tree.declarations.find((declaration) => declaration.name === "Shape")!;
    shape.doc = 'Draws <img src=x onerror="alert(1)"> and [runs](javascript:alert(1)).';
    const element = show({ tree, text: hover.text });
    pointAt(drawn(element, keyIn(hover, firstShape)).querySelector(".line-kind")!);
    t.mock.timers.tick(500);
    const doc = card(element).querySelector(".card-doc")!;
    assert.equal(doc.querySelector("img, a"), null);
    assert.equal(doc.textContent, 'Draws <img src=x onerror="alert(1)"> and runs.');
  });

  it("opens the next target's card once the pointer rests there, after the first", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const type = keyIn(hover, (node) => node.role === "attribute" && node.name === "Type");
    const circle = keyIn(hover, (node) => node.role === "case" && node.name === "Circle");
    pointAt(marksOf(element, type)[0]!);
    t.mock.timers.tick(500);
    pointAt(drawn(element, circle));
    t.mock.timers.tick(300);
    assert.equal(signatureOf(element), undefined, "the first closes after its grace");
    t.mock.timers.tick(200);
    assert.equal(signatureOf(element), "(case) ShapeType.Circle", "and the next opens");
    // From the card onto another target, and across a gap, the same.
    card(element).dispatchEvent(new window.MouseEvent("pointerenter"));
    card(element).dispatchEvent(new window.MouseEvent("pointerleave"));
    pointAt(drawn(element, keyIn(hover, firstShape)).querySelector(".line-gap")!);
    pointAt(marksOf(element, type)[0]!);
    t.mock.timers.tick(500);
    assert.equal(signatureOf(element), "(property) Type: ShapeType = Rectangle");
  });

  it("takes the spans of one node for one target: a line's name and its =", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const type = keyIn(hover, (node) => node.role === "attribute" && node.name === "Type");
    const [name, equals] = marksOf(element, type);
    pointAt(name!);
    t.mock.timers.tick(300);
    pointAt(equals!);
    t.mock.timers.tick(200);
    assert.equal(signatureOf(element), "(property) Type: ShapeType = Rectangle", "the rest is not restarted");
    pointAt(name!);
    t.mock.timers.tick(1000);
    assert.equal(signatureOf(element), "(property) Type: ShapeType = Rectangle", "and it stays open");
  });

  it("closes on a new tree", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    pointAt(drawn(element, keyIn(hover, firstShape)).querySelector(".line-kind")!);
    t.mock.timers.tick(500);
    assert.notEqual(signatureOf(element), undefined);
    element.tree = structuredClone(hover.tree);
    assert.equal(card(element).hidden, true);
  });

  it("closes when what scrolls the reading scrolls, inside a host's shadow root too", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const host = document.createElement("div");
    document.body.append(host);
    t.after(() => host.remove());
    const scroller = document.createElement("div");
    scroller.style.overflowY = "auto";
    host.attachShadow({ mode: "open" }).append(scroller);
    const element = show(hover, scroller);
    pointAt(drawn(element, keyIn(hover, firstShape)).querySelector(".line-kind")!);
    t.mock.timers.tick(500);
    assert.equal(card(element).hidden, false);
    scroller.dispatchEvent(new window.Event("scroll"));
    assert.equal(card(element).hidden, true, "a scroll the document does not hear of");
  });

  it("keeps a card asked for from the keyboard while the pointer crosses it, until the selection moves", async (t) => {
    narrow(t, 10);
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    const type = keyIn(hover, (node) => node.role === "attribute" && node.name === "Type");
    element.selection = type;
    press(body(element), "i");
    card(element).dispatchEvent(new window.MouseEvent("pointerenter"));
    card(element).dispatchEvent(new window.MouseEvent("pointerleave"));
    body(element).dispatchEvent(new window.MouseEvent("pointerleave"));
    t.mock.timers.tick(1000);
    assert.equal(signatureOf(element), "(property) Type: ShapeType = Rectangle", "the pointer does not close it");
    press(body(element), "ArrowDown");
    assert.equal(card(element).hidden, true, "a new selection does");
  });

  it("says there is nothing on i where the tree says nothing, closing any other card", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    const element = show(hover);
    pointAt(drawn(element, keyIn(hover, firstShape)).querySelector(".line-kind")!);
    t.mock.timers.tick(500);
    assert.equal(card(element).hidden, false);
    element.selection = keyIn(hover, (node) => node.role === "literal" && node.value === '"hi"');
    press(body(element), "i");
    assert.equal(card(element).hidden, true);
    assert.equal(root(element).querySelector('[role="status"]')!.textContent, "No information");
  });

  it("falls back to the built-in Markdown when the host's throws or gives no node", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const hover = await fixture("hover");
    for (const markdown of [
      (): Node => {
        throw new Error("no renderer");
      },
      (): Node => "A string" as unknown as Node,
    ]) {
      const element = show(hover);
      element.markdown = markdown;
      pointAt(drawn(element, keyIn(hover, firstShape)).querySelector(".line-kind")!);
      t.mock.timers.tick(500);
      assert.equal(card(element).querySelector(".card-doc > p")!.textContent, "A filled shape.");
    }
  });
});
