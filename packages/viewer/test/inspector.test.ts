/**
 * Rows and their inspectors: an element of the composition reads as one row, and selecting it, or
 * pinning it, opens its inspector under it.
 */
import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, afterEach, describe, it, type TestContext } from "node:test";

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
  "visualizer Panel = <ElementVisualizer summary={width x}>",
  '  <PropertyVisualizer property=x short="X" />',
  '  <PropertyVisualizer property=y short="Y" />',
  '  <PropertyVisualizer property=width label="Breadth" icon="ruler" />',
  "</ElementVisualizer>",
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

/** What each summary item reads: its lead, then its value. */
function summary(element: NxViewerElement, key: string): string[] {
  return [...head(element, key).querySelectorAll<HTMLElement>(".row-summary > .summary-item:not([hidden])")].map((item) =>
    [...item.children].map((part) => part.textContent || (part.querySelector("svg")?.dataset["icon"] ?? "")).join(" ")
  );
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
  it("reads each element of the composition as a row: icon, kind, summary and pin", async () => {
    const element = show(await panel());
    const row = drawn(element, "panel.value");
    assert.ok(row.matches(".row-element"));
    assert.equal(row.getAttribute("role"), "treeitem");
    assert.equal(row.getAttribute("aria-expanded"), "true", "it holds cards, which fold");
    assert.equal(head(element, "panel.value").querySelector(".sheet-kind")!.textContent, "Panel");
    assert.deepEqual(summary(element, "panel.value"), ["ruler 30", "X 1"], "the visualizer's summary, in its order");
    assert.equal(pin(element, "panel.value").getAttribute("aria-pressed"), "false");
    assert.equal(pin(element, "panel.value").getAttribute("aria-label"), "Pin Panel");
    const card = drawn(element, "panel.value.children[0]");
    assert.equal(card.querySelector(":scope > .row-head > .fold"), null, "a row with no content has no chevron");
    assert.equal(card.hasAttribute("aria-expanded"), false);
  });

  it("summarizes the first properties written when no visualizer names any, a false boolean left out", async () => {
    const element = show(await panel());
    assert.deepEqual(summary(element, "panel.value.children[0]"), ["Title Hello", "Padding 8", "Tone quiet"]);
    assert.deepEqual(summary(element, "panel.value.children[1]"), ["Title Bye"], "elevated=false says nothing");
    const items = head(element, "panel.value.children[0]").querySelectorAll<HTMLElement>(".summary-item");
    assert.equal(items[0]!.title, "Title · title", "a tooltip with the label and the name as written");
    for (const part of head(element, "panel.value").querySelectorAll("[data-key]")) {
      assert.fail(`a summary copies values without their keys: ${(part as HTMLElement).dataset["key"]}`);
    }
  });

  it("cuts a summary that does not fit at a whole item, saying how many it left out", async (t: TestContext) => {
    const prototype = window.HTMLElement.prototype;
    Object.defineProperty(prototype, "clientWidth", {
      configurable: true,
      get(this: HTMLElement) {
        return this.matches(".row-summary") ? 100 : 400;
      }
    });
    // Each item takes 60 pixels, and the count 20.
    Object.defineProperty(prototype, "scrollWidth", {
      configurable: true,
      get(this: HTMLElement) {
        const items = this.querySelectorAll(":scope > .summary-item:not([hidden])").length;
        const more = this.querySelector(":scope > .summary-more:not([hidden])") === null ? 0 : 20;
        return items * 60 + more;
      }
    });
    t.after(() => {
      delete (prototype as { clientWidth?: number }).clientWidth;
      delete (prototype as { scrollWidth?: number }).scrollWidth;
    });
    const element = show(await panel());
    assert.deepEqual(summary(element, "panel.value.children[0]"), ["Title Hello"]);
    const more = head(element, "panel.value.children[0]").querySelector<HTMLElement>(".summary-more")!;
    assert.equal(more.hidden, false);
    assert.equal(more.textContent, "+2");
  });
});

describe("inspectors", () => {
  it("opens the selected row's inspector under it and closes it when another is selected", async () => {
    const element = show(await panel());
    assert.equal(isOpen(element, "panel.value"), false, "nothing open until something is selected");
    element.selection = "panel.value";
    assert.equal(isOpen(element, "panel.value"), true);
    assert.ok(head(element, "panel.value").querySelector<HTMLElement>(".row-summary")!.hidden, "its summary hides while it is open");
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
    assert.equal(x.querySelector(".field-lead")!.textContent, "X", "the label leads, not the short label");
    assert.equal(x.querySelector<HTMLElement>(".field-lead")!.title, "X · x", "its tooltip names it as source writes it");
    const width = drawn(element, "panel.value.width");
    assert.equal(width.querySelector<HTMLElement>(".field-lead svg")!.dataset["icon"], "ruler", "an icon before the label");
    assert.equal(width.querySelector(".field-lead")!.textContent, "Breadth");
    assert.equal(width.querySelector<HTMLElement>(".field-lead")!.title, "Breadth · width");
  });

  it("shows a property the element does not write as a dashed field with its default", async () => {
    const element = show(await panel());
    element.selection = "panel.value.children[1]";
    const unset = [...drawn(element, "panel.value.children[1]").querySelectorAll<HTMLElement>(".dimmed-row")];
    assert.deepEqual(unset.map((field) => field.dataset["name"]), ["padding", "tone", "imageURL"]);
    assert.equal(unset[2]!.querySelector(".field-lead")!.textContent, "Image URL", "a caption derived from its name");
    assert.equal(unset[0]!.textContent!.replace(/\s+/g, " ").trim(), "Padding8");
    const elevated = drawn(element, "panel.value.children[1].elevated");
    assert.ok(elevated.classList.contains("boolean-field"), "a boolean reads as a checkbox");
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
    assert.equal(badge.querySelector(".nested-kind")!.textContent, "Badge");
    badge.querySelector<HTMLElement>(".value-fold")!.click();
    assert.ok(drawn(element, "panel.value.badge").classList.contains("open"));
    drawn(element, "panel.value.badge.value.count.value").click();
    const field = element.shadowRoot!.querySelector<HTMLElement>(".field")!;
    field.textContent = "4";
    press(field, "Enter");
    assert.deepEqual(edits, [{ key: "panel.value.badge.value.count.value", edit: { kind: "setValue", value: { kind: "number", text: "4" } } }]);
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
