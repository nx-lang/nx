/**
 * Rows and their inspectors: an element of the composition reads as one row, and selecting it, or
 * pinning it, opens its inspector under it.
 */
import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, afterEach, describe, it } from "node:test";

import { changeDetails, compositionOwner, detailsOpen, emptyDetails, isComposition, reconcileDetails } from "../src/compact.js";
import { derivedLabel } from "../src/labels.js";
import { indexTree } from "../src/tree.js";
import type { NxDetailsDetail, NxEditDetail, NxViewerElement } from "../src/index.js";
import { fixture, nxHost, treeOf, type Fixture } from "./fixtures.js";

const { NX_DETAILS_EVENT, NX_EDIT_EVENT } = await import("../src/index.js");

const window = document.defaultView!;
const created: NxViewerElement[] = [];
afterEach(() => {
  for (const element of created.splice(0)) {
    element.remove();
  }
});
after(async () => (await nxHost()).dispose());

const PANEL = [
  'import "@nx/visualizer"',
  "type Tone = quiet | loud",
  "type Card = { title:string padding:int = 8 tone:Tone = quiet elevated:boolean = false imageURL?:string }",
  "type Badge = { label:string count:int = 0 }",
  "type Panel = { x:int = 0 y:int = 0 width:int = 10 badge?:Badge content children:Card* }",
  "visualizer Panel = <ElementVisualizer>",
  "  <PropertyVisualizer property=x />",
  "  <PropertyVisualizer property=y />",
  '  <PropertyVisualizer property=width label="Breadth" />',
  "</ElementVisualizer>",
  "visualizer Card = <ElementVisualizer title=title />",
  'let panel = <Panel x=1 y=2 width=30 badge=<Badge label="new" count=3 />>',
  '  <Card title="Hello" padding=8 tone=quiet elevated=true />',
  '  <Card title="Bye" elevated=false />',
  "</Panel>",
  ""
].join("\n");

async function panel(): Promise<Fixture> {
  return treeOf([{ uri: "nx:///panel.nx", identity: "panel.nx", source: PANEL }], { visualizers: { modules: [] } });
}

function show(value: Fixture, editable = false): NxViewerElement {
  const element = document.createElement("nx-viewer");
  document.body.append(element);
  element.folded = [];
  element.editable = editable;
  element.icons = (name) => {
    const drawn = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    drawn.dataset["icon"] = name;
    return drawn;
  };
  element.text = value.text;
  element.tree = value.tree;
  created.push(element);
  return element;
}

function drawn(element: NxViewerElement, key: string): HTMLElement {
  const found = [...element.shadowRoot!.querySelectorAll<HTMLElement>("[data-key]")].find((node) => node.dataset["key"] === key);
  assert.ok(found, `${key} is drawn`);
  return found;
}

function isOpen(element: NxViewerElement, key: string): boolean {
  return drawn(element, key).querySelector(":scope > .inspector") !== null;
}

function head(element: NxViewerElement, key: string): HTMLElement {
  return drawn(element, key).querySelector<HTMLElement>(":scope > .row-head")!;
}

function pin(element: NxViewerElement, key: string): HTMLButtonElement {
  return head(element, key).querySelector<HTMLButtonElement>(".row-pin")!;
}

function body(element: NxViewerElement): HTMLElement {
  return element.shadowRoot!.querySelector<HTMLElement>(".body")!;
}

function press(target: Element, key: string, init: KeyboardEventInit = {}): void {
  target.dispatchEvent(new window.KeyboardEvent("keydown", { key, bubbles: true, composed: true, cancelable: true, ...init }));
}

/** What a row's head reads after its kind: the one value it is named by, if any. */
function named(element: NxViewerElement, key: string): string | undefined {
  return head(element, key).querySelector<HTMLElement>(":scope > .row-title")?.textContent ?? undefined;
}

describe("the state of inspectors", () => {
  it("opens with the selection, keeps pins, and closes the selected one on request", () => {
    let state = emptyDetails();
    assert.equal(detailsOpen(state, "star", "star"), true, "the selected element's opens");
    state = changeDetails(state, "star", "pin", "star");
    assert.equal(detailsOpen(state, "star", "circle"), true, "a pin keeps it open");
    state = changeDetails(state, "star", "pin", "circle");
    assert.equal(detailsOpen(state, "star", "circle"), false, "unpinned, it closes");
    state = changeDetails(state, "star", "toggle", "star");
    assert.deepEqual(state, { pinned: [], suppressed: "star" }, "closing the selected one suppresses it");
    assert.equal(detailsOpen(state, "star", "star"), false);
    state = changeDetails(state, "star", "toggle", "star");
    assert.equal(detailsOpen(state, "star", "star"), true, "and the next toggle opens it again");
    state = changeDetails(changeDetails(state, "star", "toggle", "star"), "star", "pin", "star");
    assert.deepEqual(state, { pinned: ["star"], suppressed: undefined }, "pinning a closed one opens it");
  });

  it("keeps composition through content and leaves out property values, signatures and visualizers", async () => {
    const value = await treeOf([
      {
        uri: "nx:///compact.nx",
        source: 'type Paint = { color:string }\ntype Item = { fill:Paint }\ntype Row = { content items:Item+ }\nlet root() = <Row><Item fill=<Paint color="red"/> /></Row>'
      }
    ]);
    const index = indexTree(value.tree, value.text);
    for (const [at, node] of value.tree.nodes.entries()) {
      if (node.role === "element") {
        assert.equal(isComposition(index, at), node.name !== "Paint");
        if (node.name === "Paint") {
          assert.equal(value.tree.nodes[index.byKey.get(compositionOwner(index, node.key)!)!]!.name, "Item");
        }
      }
    }
  });

  it("maps unique source moves and clears ambiguous duplicate list edits", async () => {
    const source = 'type Item = { name:string }\nlet root() = {\n<Item name="a"/>\n<Item name="b"/>\n}';
    const before = await treeOf([{ uri: "nx:///items.nx", source }]);
    const a = before.tree.nodes.find((node) => node.role === "element")!;
    const swapped = await treeOf([{ uri: "nx:///items.nx", source: source.replace('name="a"', 'name="swap"').replace('name="b"', 'name="a"').replace('name="swap"', 'name="b"') }]);
    const state = reconcileDetails(indexTree(before.tree, before.text), indexTree(swapped.tree, swapped.text), { pinned: [a.key] });
    assert.notEqual(state.pinned[0], a.key);
    const duplicate = await treeOf([{ uri: "nx:///items.nx", source: source.replace('name="b"', 'name="a"') }]);
    const added = await treeOf([{ uri: "nx:///items.nx", source: duplicate.text.replace(/}$/, '<Item name="a"/>\n}') }]);
    const ambiguous = reconcileDetails(indexTree(duplicate.tree, duplicate.text), indexTree(added.tree, added.text), { pinned: [a.key] });
    assert.deepEqual(ambiguous.pinned, []);
  });

  it("ignores comments when reconciling list identities", async () => {
    const source = 'type Item = { name:string }\nlet root() = {\n<Item name="a"/>\n<Item name="b"/>\n}';
    const before = await treeOf([{ uri: "nx:///comments.nx", source }]);
    const after = await treeOf([{ uri: "nx:///comments.nx", source: source.replace('<Item name="b"/>', '// A note\n<Item name="b"/>') }]);
    const key = before.tree.nodes.find((node) => node.role === "element")!.key;
    const state = reconcileDetails(indexTree(before.tree, before.text), indexTree(after.tree, after.text), { pinned: [key] });
    assert.deepEqual(state.pinned, [key]);
  });
});

describe("labels", () => {
  it("derives a label from a property's name", () => {
    assert.deepEqual(
      [["outerRadius"], ["imageURL"], ["max_length"], ["isRequired", true], ["x1"], ["isRequired"], ["has"]].map(([name, boolean]) =>
        derivedLabel(name as string, boolean === true)
      ),
      ["Outer radius", "Image URL", "Max length", "Required", "X1", "Is required", "Has"]
    );
  });
});

describe("rows", () => {
  it("reads each element of the composition as a row: icon, kind, the value it is named by, and pin", async () => {
    const element = show(await panel());
    const row = drawn(element, "panel.value");
    assert.ok(row.matches(".row-element"));
    assert.equal(row.getAttribute("role"), "treeitem");
    assert.equal(row.getAttribute("aria-expanded"), "true", "it holds cards, which fold");
    assert.equal(head(element, "panel.value").querySelector(".sheet-kind")!.textContent, "Panel");
    assert.equal(named(element, "panel.value"), undefined, "no title, text or id names it, so nothing else it sets shows");
    assert.equal(pin(element, "panel.value").getAttribute("aria-pressed"), "false");
    assert.equal(pin(element, "panel.value").getAttribute("aria-label"), "Pin Panel");
    const card = drawn(element, "panel.value.children[0]");
    assert.equal(card.querySelector(":scope > .row-head > .fold"), null, "a row with no content has no chevron");
    assert.equal(card.hasAttribute("aria-expanded"), false);
  });

  it("names a row by its title alone, with no label, and copies it without its keys", async () => {
    const element = show(await panel());
    assert.equal(named(element, "panel.value.children[0]"), "Hello", "not its padding or tone");
    assert.equal(named(element, "panel.value.children[1]"), "Bye");
    assert.equal(head(element, "panel.value.children[0]").querySelector<HTMLElement>(".row-title")!.hasAttribute("title"), false, "no browser tooltip over a card's");
    for (const part of head(element, "panel.value.children[0]").querySelectorAll("[data-key]")) {
      assert.fail(`a row's value is a copy, without its keys: ${(part as HTMLElement).dataset["key"]}`);
    }
  });

  it("edits the value a row is named by in its inspector, on a click, opening the inspector first", async () => {
    const element = show(await panel(), true);
    const edits: NxEditDetail[] = [];
    element.addEventListener(NX_EDIT_EVENT, (event) => edits.push((event as CustomEvent<NxEditDetail>).detail));
    head(element, "panel.value.children[1]").querySelector<HTMLElement>(".row-title")!.click();
    assert.equal(element.selection, "panel.value.children[1].title", "its property is selected");
    assert.equal(isOpen(element, "panel.value.children[1]"), true, "in its element's open inspector");
    const field = element.shadowRoot!.querySelector<HTMLElement>(".field")!;
    assert.ok(field.closest(".inspector"), "its field opens in the inspector");
    const selection = element.shadowRoot!.ownerDocument.getSelection?.();
    if (selection !== null && selection !== undefined && selection.rangeCount > 0) {
      assert.ok(selection.isCollapsed, "nothing is selected, so typing adds to the value");
    }
    field.textContent = "Later";
    press(field, "Enter");
    assert.deepEqual(edits, [{ key: "panel.value.children[1].title.value", edit: { kind: "setValue", value: { kind: "string", value: "Later" } } }]);
  });
});

describe("inspectors", () => {
  it("opens the selected row's inspector under it and closes it when another is selected", async () => {
    const element = show(await panel());
    assert.equal(isOpen(element, "panel.value"), false, "nothing open until something is selected");
    element.selection = "panel.value";
    assert.equal(isOpen(element, "panel.value"), true);
    element.selection = "panel.value.children[0]";
    assert.equal(isOpen(element, "panel.value"), false);
    assert.equal(isOpen(element, "panel.value.children[0]"), true);
    element.selection = "panel.value.x";
    assert.equal(isOpen(element, "panel.value"), true, "a property's selection opens its element's inspector");
  });

  it("shows every property as a field, its label beside its value, in visualizer order", async () => {
    const element = show(await panel());
    element.selection = "panel.value";
    const inspector = drawn(element, "panel.value").querySelector<HTMLElement>(":scope > .inspector")!;
    const fields = [...inspector.querySelectorAll<HTMLElement>(":scope > .inspector-fields > .prop-field")].map(
      (field) => field.dataset["name"] ?? field.dataset["key"]!.split(".").at(-1)
    );
    assert.deepEqual(fields, ["x", "y", "width", "badge"]);
    assert.equal(inspector.querySelector(".group-head"), null, "no groups");
    const x = drawn(element, "panel.value.x");
    assert.deepEqual([...x.children].map((child) => child.className.split(" ")[0]), ["field-lead", "sheet-value"], "the label, then the value");
    assert.equal(x.querySelector(".field-lead")!.textContent, "X", "a label derived from its name");
    assert.equal(x.querySelector<HTMLElement>(".field-lead")!.hasAttribute("title"), false, "its card names it as source writes it, with no browser tooltip");
    const width = drawn(element, "panel.value.width");
    assert.equal(width.querySelector(".field-lead svg"), null, "a label alone, with no icon");
    assert.equal(width.querySelector(".field-lead")!.textContent, "Breadth");
    assert.equal(width.querySelector<HTMLElement>(".field-lead")!.dataset["hover"], "panel.value.width", "its card names it as source writes it");
  });

  it("shows what an element sets, and a last line that reveals the rest as dashed fields", async () => {
    const element = show(await panel());
    element.selection = "panel.value.children[1]";
    const row = (): HTMLElement => drawn(element, "panel.value.children[1]");
    const names = (): (string | undefined)[] => [...row().querySelectorAll<HTMLElement>(":scope > .inspector .prop-field > .field-lead")].map((lead) => lead.textContent ?? undefined);
    assert.deepEqual(names(), ["Title", "Elevated"], "only what the Card sets");
    const more = (): HTMLButtonElement => row().querySelector<HTMLButtonElement>(":scope > .inspector > .inspector-more")!;
    assert.equal(more().textContent, "Show 3 unset");
    assert.equal(more().getAttribute("aria-expanded"), "false");
    more().click();
    const unset = [...row().querySelectorAll<HTMLElement>(":scope > .inspector .dimmed-row")];
    assert.deepEqual(unset.map((field) => field.dataset["name"]), ["padding", "tone", "imageURL"]);
    assert.deepEqual(names(), ["Title", "Padding", "Tone", "Elevated", "Image URL"], "each in its place");
    assert.equal(unset[2]!.querySelector(".field-lead")!.textContent, "Image URL", "a caption derived from its name");
    assert.equal(unset[0]!.textContent!.replace(/\s+/g, " ").trim(), "Padding8");
    assert.ok(drawn(element, "panel.value.children[1].elevated").classList.contains("boolean-field"), "a boolean reads as a checkbox");
    assert.equal(more().textContent, "Hide unset");
    assert.equal(more().getAttribute("aria-expanded"), "true");
    // Closing the inspector forgets it: it opens again on what the Card sets.
    element.selection = "panel.value.children[0]";
    element.selection = "panel.value.children[1]";
    assert.deepEqual(names(), ["Title", "Elevated"]);
  });

  it("always shows the title property, so a click on the row's value has a field to open", async () => {
    const source = [
      'import "@nx/visualizer"',
      "type Note = { text?:string pinned:boolean = false }",
      "visualizer Note = <ElementVisualizer title=text />",
      "let note = <Note />",
      ""
    ].join("\n");
    const element = show(await treeOf([{ uri: "nx:///note.nx", identity: "note.nx", source }], { visualizers: { modules: [] } }));
    element.selection = "note.value";
    const inspector = drawn(element, "note.value").querySelector(":scope > .inspector")!;
    assert.deepEqual([...inspector.querySelectorAll<HTMLElement>(".prop-field > .field-lead")].map((lead) => lead.textContent), ["Text"]);
    assert.equal(inspector.querySelector(".inspector-more")!.textContent, "Show 1 unset");
  });

  it("marks an action value, a component's State.Update included, with the action icon", async () => {
    const source = [
      "external component <Button label:string emits { Tapped { } } />",
      "external component <Row content children:Button* />",
      "component <Counter emits { Finished { total:int } } /> = {",
      "  state { count:int = 0 }",
      "  <Row>",
      "    <Button label=\"Add\" onTapped=<State.Update count={count + 1} /> />",
      "    <Button label=\"Finish\" onTapped=<Counter.Finished total={count} /> />",
      "  </Row>",
      "}",
      ""
    ].join("\n");
    const element = show(await treeOf([{ uri: "nx:///counter.nx", identity: "counter.nx", source }]));
    const icons = (key: string): string[] => {
      element.selection = key;
      return [...drawn(element, key).querySelectorAll<HTMLElement>(".nested-head > .element-icon svg")].map((svg) => svg.dataset["icon"]!);
    };
    const buttons = [...element.shadowRoot!.querySelectorAll<HTMLElement>(".row-element[aria-label='Button']")].map((row) => row.dataset["key"]!);
    assert.equal(buttons.length, 2);
    assert.deepEqual(icons(buttons[0]!), ["zap"], "State.Update reads as an action");
    assert.deepEqual(icons(buttons[1]!), ["zap"], "an emitted action too");
  });

  it("unfolds an element value into a nested group of its fields, edited in that element", async () => {
    const element = show(await panel(), true);
    const edits: NxEditDetail[] = [];
    element.addEventListener(NX_EDIT_EVENT, (event) => edits.push((event as CustomEvent<NxEditDetail>).detail));
    element.selection = "panel.value";
    const badge = drawn(element, "panel.value.badge");
    assert.ok(badge.classList.contains("nested-field"));
    badge.querySelector<HTMLElement>(".value-fold")!.click();
    assert.ok(drawn(element, "panel.value.badge").classList.contains("open"));
    assert.equal(drawn(element, "panel.value.badge").querySelector(".nested-kind")!.textContent, "Badge", "open, its kind reads");
    drawn(element, "panel.value.badge.value.count.value").click();
    const field = element.shadowRoot!.querySelector<HTMLElement>(".field")!;
    field.textContent = "4";
    press(field, "Enter");
    assert.deepEqual(edits, [{ key: "panel.value.badge.value.count.value", edit: { kind: "setValue", value: { kind: "number", text: "4" } } }]);
  });

  it("reads a short element value's properties on its line, each edited and scrubbed there", async () => {
    const element = show(await panel(), true);
    const edits: NxEditDetail[] = [];
    element.addEventListener(NX_EDIT_EVENT, (event) => edits.push((event as CustomEvent<NxEditDetail>).detail));
    element.selection = "panel.value";
    const badge = () => drawn(element, "panel.value.badge");
    assert.equal(badge().querySelector(".nested-kind"), null, "the kind its type names reads no more");
    const inline = [...badge().querySelectorAll<HTMLElement>(".inline-prop")];
    assert.deepEqual(inline.map((field) => [field.querySelector(".inline-label")?.textContent, field.querySelector(".inline-value")?.textContent]), [["Label", "new"], ["Count", "3"]]);
    assert.ok(inline[1]!.querySelector(".inline-label")!.classList.contains("scrubbable"), "a number's label scrubs it");
    drawn(element, "panel.value.badge.value.count.value").click();
    const field = element.shadowRoot!.querySelector<HTMLElement>(".field")!;
    assert.ok(badge().contains(field), "the field opens in place, the value still folded");
    field.textContent = "4";
    press(field, "Enter");
    assert.deepEqual(edits, [{ key: "panel.value.badge.value.count.value", edit: { kind: "setValue", value: { kind: "number", text: "4" } } }]);
  });

  it("keeps the braces source writes around an expression, quiet, and around a reference", async () => {
    const source = ["type Box = { w:int name:string label?:string }", 'let title = "Hi"', 'let box = <Box w={1 + 2} name={ title } label="plain" />', ""].join("\n");
    const element = show(await treeOf([{ uri: "nx:///box.nx", identity: "box.nx", source }], { visualizers: { modules: [] } }));
    element.selection = "box.value";
    const cell = (name: string) => drawn(element, `box.value.${name}`).querySelector<HTMLElement>(":scope > .sheet-value")!;
    assert.equal(cell("w").textContent, "{1 + 2}");
    assert.equal(cell("name").textContent, "{title}", "the spaces inside go");
    assert.equal(cell("label").textContent, "plain", "a literal has none");
    assert.deepEqual([...cell("w").querySelectorAll(".value-brace")].map((brace) => brace.classList.contains("block-brace")), [true, true]);
  });

  it("leads a reference to a color with its swatch, and its menu goes to the let it names", async () => {
    const source = ["type Tag = { name:string tint?:color }", "let blue = #2563eb", 'let tag = <Tag name="t" tint={blue} />', ""].join("\n");
    const element = show(await treeOf([{ uri: "nx:///tag.nx", identity: "tag.nx", source }], { visualizers: { modules: [] } }), true);
    element.selection = "tag.value";
    const tint = drawn(element, "tag.value.tint");
    const cell = tint.querySelector<HTMLElement>(":scope > .sheet-value")!;
    assert.ok(cell.firstElementChild!.matches(".swatch"), "the swatch comes first, before the brace");
    assert.equal([...cell.childNodes].filter((node) => !(node as Element).matches?.(".row-anchor")).map((node) => node.textContent).join(""), "{blue}");
    element.selection = "tag.value.tint";
    drawn(element, "tag.value.tint").querySelector<HTMLElement>(":scope > * > .row-anchor > .row-menu")!.click();
    const items = [...element.shadowRoot!.querySelectorAll<HTMLElement>(".menu-item")];
    assert.deepEqual(items.map((item) => item.querySelector(".menu-label")!.textContent), ["Go to blue", "Remove", "Show in Source"]);
    items[0]!.click();
    assert.match(element.selection ?? "", /^blue/);
  });

  it("finds a reference's swatch and Go to through what it resolves to, not its name", async () => {
    const source = [
      "type Tag = { name:string tint?:color }",
      "let blue = #2563eb",
      'let <Badge blue:color /> = <Tag name="t" tint={blue} />',
      'let tag = <Tag name="u" tint={blue} />',
      ""
    ].join("\n");
    const fixture = await treeOf([{ uri: "nx:///shadow.nx", identity: "shadow.nx", source }], { visualizers: { modules: [] } });
    const element = show(fixture, true);
    element.selection = "tag.value";
    assert.ok(drawn(element, "tag.value.tint").querySelector(":scope > .sheet-value > .swatch"), "the top-level let's swatch");
    const local = fixture.tree.nodes.find((node) => node.role === "reference" && node.name === "blue" && !node.key.startsWith("tag."))!;
    assert.equal(local.declaration, undefined, "the tree resolves the parameter as a local");
    const attribute = fixture.tree.nodes[local.parent!]!;
    element.selection = attribute.key;
    const drawnLocal = drawn(element, attribute.key);
    assert.equal(drawnLocal.querySelector(".swatch"), null, "a parameter named like a let has no swatch");
    const menu = drawnLocal.querySelector<HTMLElement>(".row-menu");
    assert.ok(menu !== null, "the parameter's property has a menu");
    menu.click();
    const labels = [...element.shadowRoot!.querySelectorAll<HTMLElement>(".menu-item .menu-label")].map((label) => label.textContent);
    assert.ok(labels.length > 0 && !labels.includes("Go to blue"), "and no Go to");
  });

  it("gives no label or value in an inspector a browser tooltip", async () => {
    const source = ["type Fill = { tint:color on:boolean = false size:int = 1 }", "let fill = <Fill tint=#33669980 on=true size=2 />", ""].join("\n");
    const element = show(await treeOf([{ uri: "nx:///tips.nx", identity: "tips.nx", source }], { visualizers: { modules: [] } }), true);
    element.selection = "fill.value";
    const inspector = element.shadowRoot!.querySelector<HTMLElement>(".inspector")!;
    assert.deepEqual([...inspector.querySelectorAll(".inspector-fields [title]")].map((node) => node.className), []);
    drawn(element, "fill.value.on.value").click();
    assert.deepEqual([...inspector.querySelectorAll(".inspector-fields [title]")].map((node) => node.className), [], "nor once toggled");
    const unset = ["type Flags = { on:boolean = false tint?:color size:int = 1 }", "let flags = <Flags size=2 />", ""].join("\n");
    const other = show(await treeOf([{ uri: "nx:///flags.nx", identity: "flags.nx", source: unset }], { visualizers: { modules: [] } }), true);
    other.selection = "flags.value";
    other.shadowRoot!.querySelector<HTMLElement>(".inspector-more")!.click();
    const shown = other.shadowRoot!.querySelector<HTMLElement>(".inspector")!;
    assert.ok(shown.querySelector(".dimmed-row"), "the unset properties show");
    assert.deepEqual([...shown.querySelectorAll("[title]")].map((node) => node.className), [], "nor on unset ones, or the line that shows them");
  });

  it("reads an element value inline past the size limits when its visualizer says inline=true", async () => {
    const source = [
      'import "@nx/visualizer"',
      "type Box = { a:int b:int c:int d:int e:int }",
      "type Holder = { box?:Box }",
      "visualizer Box = <ElementVisualizer inline=true />",
      "let holder = <Holder box=<Box a=1 b=2 c=3 d=4 e=5 /> />",
      ""
    ].join("\n");
    const element = show(await treeOf([{ uri: "nx:///wide.nx", identity: "wide.nx", source }], { visualizers: { modules: [] } }));
    element.selection = "holder.value";
    const labels = [...drawn(element, "holder.value.box").querySelectorAll(".inline-label")].map((label) => label.textContent);
    assert.deepEqual(labels, ["A", "B", "C", "D", "E"]);
  });

  it("reads only the case of an inline union value, and scrubs its numbers on their labels", async () => {
    const source = [
      "type Paint =",
      "  | solid { color:color }",
      "  | dashed { width:int gap:int }",
      "type Shape = { fill?:Paint stroke?:Paint }",
      "let shape = <Shape fill=<Paint.solid color=#ff0000 /> stroke=<Paint.dashed width=2 gap=4 /> />",
      ""
    ].join("\n");
    const element = show(await treeOf([{ uri: "nx:///paint.nx", identity: "paint.nx", source }], { visualizers: { modules: [] } }), true);
    const edits: NxEditDetail[] = [];
    element.addEventListener(NX_EDIT_EVENT, (event) => edits.push((event as CustomEvent<NxEditDetail>).detail));
    element.selection = "shape.value";
    const stroke = drawn(element, "shape.value.stroke");
    assert.equal(stroke.querySelector(".nested-kind")!.textContent, "dashed", "the case, not Paint.dashed");
    const width = [...stroke.querySelectorAll<HTMLElement>(".inline-label")].find((label) => label.textContent === "Width")!;
    const at = (type: string, clientX: number) =>
      width.dispatchEvent(new window.MouseEvent(type, { bubbles: true, composed: true, cancelable: true, clientX }));
    at("pointerdown", 0);
    for (const x of [4, 7, 10]) {
      at("pointermove", x);
    }
    at("pointerup", 10);
    const commits = edits.filter((edit) => edit.live !== true);
    assert.ok(edits.length > commits.length, "live while dragging");
    assert.equal(commits.length, 1, "one commit on release");
    assert.match(commits[0]!.key, /^shape\.value\.stroke\.value\.width/);
    assert.deepEqual(commits[0]!.edit, { kind: "setValue", value: { kind: "number", text: "5" } }, "a step for every three pixels from the press");
  });

  it("keeps an element value folded when it is not short, or its visualizer says so", async () => {
    const source = [
      'import "@nx/visualizer"',
      "type Badge = { label:string count:int = 0 }",
      "type Box = { a:int b:int c:int d:int e:int }",
      "type Holder = { badge?:Badge box?:Box }",
      "visualizer Badge = <ElementVisualizer inline=false />",
      'let holder = <Holder badge=<Badge label="new" count=3 /> box=<Box a=1 b=2 c=3 d=4 e=5 /> />',
      ""
    ].join("\n");
    const element = show(await treeOf([{ uri: "nx:///holder.nx", identity: "holder.nx", source }], { visualizers: { modules: [] } }));
    element.selection = "holder.value";
    for (const key of ["holder.value.badge", "holder.value.box"]) {
      assert.equal(drawn(element, key).querySelector(".inline-prop"), null, key);
      assert.ok(drawn(element, key).querySelector(".nested-kind"), key);
    }
  });

  it("closes and reopens the selected row's inspector on a click on its row or Enter, the selection kept", async () => {
    const element = show(await panel());
    const fired: NxDetailsDetail[] = [];
    element.addEventListener(NX_DETAILS_EVENT, (event) => fired.push((event as CustomEvent<NxDetailsDetail>).detail));
    element.selection = "panel.value";
    head(element, "panel.value").querySelector<HTMLElement>(".sheet-kind")!.click();
    assert.equal(isOpen(element, "panel.value"), false);
    assert.equal(element.selection, "panel.value");
    assert.deepEqual(fired.at(-1), { state: { pinned: [], suppressed: "panel.value" }, selection: "panel.value" });
    head(element, "panel.value").querySelector<HTMLElement>(".sheet-kind")!.click();
    assert.equal(isOpen(element, "panel.value"), true);
    press(body(element), "Enter");
    assert.equal(isOpen(element, "panel.value"), false, "Enter closes it");
    press(body(element), "Enter");
    assert.equal(isOpen(element, "panel.value"), true, "and opens it again");
    assert.match(element.shadowRoot!.querySelector('[role="status"]')!.textContent ?? "", /Panel properties shown/);
  });

  it("keeps a pinned inspector open as the selection moves, and restores state without an echo", async () => {
    const element = show(await panel());
    const fired: NxDetailsDetail[] = [];
    element.addEventListener(NX_DETAILS_EVENT, (event) => fired.push((event as CustomEvent<NxDetailsDetail>).detail));
    element.selection = "panel.value.children[0]";
    pin(element, "panel.value.children[0]").click();
    assert.equal(pin(element, "panel.value.children[0]").getAttribute("aria-pressed"), "true");
    element.selection = "panel.value.children[1]";
    assert.equal(isOpen(element, "panel.value.children[0]"), true);
    assert.equal(isOpen(element, "panel.value.children[1]"), true);
    assert.deepEqual(fired.at(-1)!.state.pinned, ["panel.value.children[0]"]);
    const count = fired.length;
    element.detailsState = element.detailsState;
    assert.equal(fired.length, count, "assigning equal state fires nothing");
    element.detailsState = undefined;
    assert.equal(isOpen(element, "panel.value.children[0]"), false, "undefined clears it");
    assert.equal(fired.length, count);
  });

  it("folds children apart from the inspector", async () => {
    const element = show(await panel());
    element.selection = "panel.value";
    head(element, "panel.value").querySelector<HTMLElement>(".fold")!.click();
    assert.equal(isOpen(element, "panel.value"), true, "folding leaves the inspector open");
    assert.equal(drawn(element, "panel.value").querySelector(".sheet-held"), null, "and hides the cards");
    element.selection = "panel.value.children[0]";
    assert.equal(element.folded.includes("panel.value"), false, "selecting a card unfolds its holder");
  });

  it("keeps state across an empty tree and drops keys a new document lacks", async () => {
    const value = await panel();
    const element = show(value);
    element.detailsState = { pinned: ["panel.value"] };
    element.tree = undefined;
    element.tree = value.tree;
    assert.deepEqual(element.detailsState.pinned, ["panel.value"]);
    const other = await fixture("user");
    element.text = other.text;
    element.tree = other.tree;
    assert.deepEqual(element.detailsState.pinned, []);
  });

  it("keeps an inspector holding an invalid entry open", async () => {
    const element = show(await panel(), true);
    element.selection = "panel.value";
    drawn(element, "panel.value.x.value").click();
    const field = element.shadowRoot!.querySelector<HTMLElement>(".field")!;
    field.textContent = "abc";
    head(element, "panel.value").querySelector<HTMLElement>(".sheet-kind")!.click();
    assert.equal(isOpen(element, "panel.value"), true, "a click on the row keeps it open");
    assert.equal(element.shadowRoot!.querySelector(".field"), field, "with the field");
    element.selection = "panel.value.children[0]";
    assert.equal(isOpen(element, "panel.value"), true, "and so does another selection");
    assert.equal(element.selection, "panel.value", "which does not happen");
    assert.equal(isOpen(element, "panel.value.children[0]"), false);
  });

  it("answers a refused host selection with the one it keeps, and drops the entry on a press outside", async () => {
    const element = show(await panel(), true);
    element.selection = "panel.value";
    drawn(element, "panel.value.x.value").click();
    const field = element.shadowRoot!.querySelector<HTMLElement>(".field")!;
    field.textContent = "abc";
    const selected: string[] = [];
    element.addEventListener("nx-select", (event) => selected.push((event as CustomEvent<{ key: string }>).detail.key));
    element.selection = "panel.value.children[0]";
    assert.deepEqual(selected, ["panel.value.x"], "the host hears the selection kept");
    const outside = document.createElement("button");
    document.body.append(outside);
    outside.dispatchEvent(new window.MouseEvent("pointerdown", { bubbles: true, composed: true }));
    outside.remove();
    assert.equal(element.shadowRoot!.querySelector(".field"), null, "the entry is dropped");
    assert.equal(drawn(element, "panel.value.x.value").textContent, "1");
  });

  it("goes from a selected row to its pin with Tab, and from there into its inspector", async () => {
    const element = show(await panel(), true);
    element.selection = "panel.value";
    assert.equal(pin(element, "panel.value").tabIndex, 0);
    assert.equal(pin(element, "panel.value.children[0]").tabIndex, -1);
    body(element).focus();
    press(body(element), "Tab");
    assert.equal(element.shadowRoot!.activeElement, pin(element, "panel.value"), "the pin takes focus");
    press(pin(element, "panel.value"), "Tab");
    const field = element.shadowRoot!.querySelector<HTMLElement>(".field");
    assert.ok(field, "the first value it can edit opens");
    assert.equal(element.selection, "panel.value.x");
    assert.equal(pin(element, "panel.value").tabIndex, 0, "the pin of the row the selection is in stays a stop");
  });

  it("walks the rows with Up and Down from a focused pin", async () => {
    const element = show(await panel(), true);
    element.selection = "panel.value";
    pin(element, "panel.value").focus();
    press(pin(element, "panel.value"), "ArrowDown");
    assert.equal(element.selection, "panel.value.children[0]");
    assert.equal(element.shadowRoot!.activeElement, body(element));
  });
});
