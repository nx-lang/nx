import { document } from "./dom.js";
import assert from "node:assert/strict";
import { after, describe, it } from "node:test";
import { changeDetails, compositionOwner, detailsOpen, emptyDetails, isComposition, reconcileDetails } from "../src/compact.js";
import { NX_DETAILS_EVENT, NX_EDIT_EVENT, NxViewerElement } from "../src/index.js";
import { renderSource } from "../src/source.js";
import { indexTree } from "../src/tree.js";
import { fixture, nxHost, treeOf, type Fixture } from "./fixtures.js";

const created: NxViewerElement[] = [];
after(async () => {
  created.forEach((element) => element.remove());
  (await nxHost()).dispose();
});
function show(value: Fixture, editable = false): NxViewerElement {
  const element = document.createElement("nx-viewer");
  element.compact = true;
  element.folded = [];
  element.editable = editable;
  element.text = value.text;
  element.tree = value.tree;
  document.body.append(element);
  created.push(element);
  return element;
}
function drawn(element: NxViewerElement, key: string): HTMLElement {
  const result = [...element.shadowRoot!.querySelectorAll<HTMLElement>("[data-key]")].find((node) => node.dataset["key"] === key);
  assert.ok(result, key);
  return result;
}
function control(element: NxViewerElement, key: string, action: string): HTMLButtonElement {
  return drawn(element, key).querySelector<HTMLButtonElement>(`:scope > .sheet-head .compact-${action}`)!;
}
function open(element: NxViewerElement, key: string): boolean {
  return !drawn(element, key).querySelector<HTMLElement>(":scope > .compact-properties")!.hidden;
}

describe("compact disclosure", () => {
  it("upgrades compact state assigned before custom element definition", async () => {
    const value = await fixture("tasks");
    const key = value.tree.nodes.find((node) => node.role === "element")!.key;
    const tag = "nx-compact-defined-late";
    const element = document.createElement(tag) as NxViewerElement;
    Object.assign(element, { text: value.text, tree: value.tree, compact: true, detailsState: { opened: [], pinned: [key] } });
    document.body.append(element);
    customElements.define(tag, class extends NxViewerElement {});
    created.push(element);
    assert.equal(open(element, key), true);
    assert.deepEqual(element.detailsState.pinned, [key]);
  });

  it("separates deliberate disclosure, pins, and temporary selection", () => {
    let state = emptyDetails();
    assert.equal(detailsOpen(state, "star", "star"), true);
    state = changeDetails(state, "star", "pin", "star");
    assert.equal(detailsOpen(state, "star", "circle"), true);
    assert.deepEqual(state.opened, []);
    state = changeDetails(state, "star", "pin", "circle");
    assert.equal(detailsOpen(state, "star", "circle"), false);
    state = changeDetails(state, "star", "details", "circle");
    state = changeDetails(state, "star", "pin", "circle");
    state = changeDetails(state, "star", "pin", "circle");
    assert.equal(detailsOpen(state, "star", "circle"), true);
    state = changeDetails(state, "star", "details", "star");
    assert.equal(detailsOpen(state, "star", "star"), false);
  });

  it("keeps composition through content and excludes nested property records", async () => {
    const value = await treeOf([{ uri: "nx:///compact.nx", source: 'type Paint = { color:string }\ntype Item = { fill:Paint }\ntype Row = { content items:Item+ }\nlet root() = <Row><Item fill=<Paint color="red"/> /></Row>' }]);
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

  it("follows selection, keeps pins, and restores state without echo", async () => {
    const value = await fixture("tasks");
    const [a, b] = value.tree.nodes.filter((node) => node.role === "element");
    const element = show(value);
    assert.equal(element.compact, true);
    assert.equal(open(element, a!.key), false);
    element.selection = a!.key;
    assert.equal(open(element, a!.key), true);
    element.selection = b!.key;
    assert.equal(open(element, a!.key), false);
    control(element, a!.key, "pin").click();
    assert.equal(open(element, a!.key), true);
    let count = 0;
    element.addEventListener(NX_DETAILS_EVENT, () => count++);
    element.detailsState = element.detailsState;
    assert.equal(count, 0);
    control(element, a!.key, "pin").click();
    assert.equal(count, 1);
    assert.equal(open(element, a!.key), false);
    control(element, a!.key, "details").click();
    element.selection = a!.key;
    element.selection = b!.key;
    assert.equal(open(element, a!.key), true);
  });

  it("closes selected details without reopening, and reveals an exact host-selected property", async () => {
    const value = await fixture("tasks");
    const a = value.tree.nodes.find((node) => node.role === "element")!;
    const property = value.tree.nodes.find((node) => node.parent === value.tree.nodes.indexOf(a) && node.role === "attribute")!;
    const element = show(value);
    element.selection = property.key;
    control(element, a.key, "details").click();
    assert.equal(element.selection, a.key);
    assert.equal(open(element, a.key), false);
    element.selection = a.key;
    assert.equal(open(element, a.key), false);
    element.selection = property.key;
    assert.equal(open(element, a.key), true);
    assert.equal(element.selection, property.key);
  });

  it("folds children independently and preserves a selected container property", async () => {
    const value = await fixture("groups");
    const row = value.tree.nodes.find((node) => node.role === "element" && node.name === "Row")!;
    const item = value.tree.nodes.find((node) => node.role === "element" && node.name === "Item")!;
    const property = value.tree.nodes.find((node) => node.parent === value.tree.nodes.indexOf(row) && node.name === "gap")!;
    const element = show(value);
    element.selection = property.key;
    drawn(element, row.key).querySelector<HTMLButtonElement>(":scope > .sheet-head .fold")!.click();
    assert.equal(element.selection, property.key);
    assert.equal(open(element, row.key), true);
    assert.equal(element.shadowRoot!.querySelectorAll('.compact-sheet').length, 1);
    element.selection = item.key;
    assert.equal(element.folded?.includes(row.key), false);
    assert.equal(open(element, item.key), true);
    assert.equal(drawn(element, item.key).querySelector(":scope > .sheet-head .fold"), null);
    control(element, item.key, "pin").click();
    element.collapseAll();
    element.expandAll();
    assert.equal(open(element, item.key), true);
  });

  it("retains state across an empty tree and clears deleted owners", async () => {
    const value = await fixture("tasks");
    const a = value.tree.nodes.find((node) => node.role === "element")!;
    const element = show(value);
    control(element, a.key, "pin").click();
    element.tree = undefined;
    element.tree = value.tree;
    assert.deepEqual(element.detailsState.pinned, [a.key]);
    const empty = await fixture("user");
    element.text = empty.text;
    element.tree = empty.tree;
    assert.deepEqual(element.detailsState.pinned, []);
  });

  it("uses real editors and retains invalid drafts when details close", async () => {
    const value = await fixture("tasks");
    const a = value.tree.nodes.find((node) => node.role === "element")!;
    const priority = value.tree.nodes.find((node) => node.name === "priority" && node.role === "attribute")!;
    const element = show(value, true);
    element.selection = a.key;
    const number = drawn(element, priority.key).querySelector<HTMLElement>(".editable-value")!;
    number.click();
    const field = element.shadowRoot!.querySelector<HTMLElement>('.field')!;
    assert.ok(field);
    field.textContent = "invalid";
    control(element, a.key, "details").click();
    assert.equal(open(element, a.key), true);
    assert.equal(element.shadowRoot!.querySelector('.field'), field);
    field.textContent = "4";
    let edits = 0;
    element.addEventListener(NX_EDIT_EVENT, () => edits++);
    control(element, a.key, "details").click();
    assert.equal(edits, 1);
    assert.equal(open(element, a.key), false);
  });

  it("maps unique source moves and clears ambiguous duplicate list edits", async () => {
    const source = 'type Item = { name:string }\nlet root() = {\n<Item name="a"/>\n<Item name="b"/>\n}';
    const before = await treeOf([{ uri: "nx:///items.nx", source }]);
    const a = before.tree.nodes.find((node) => node.role === "element")!;
    const swapped = await treeOf([{ uri: "nx:///items.nx", source: source.replace('name="a"', 'name="swap"').replace('name="b"', 'name="a"').replace('name="swap"', 'name="b"') }]);
    const state = reconcileDetails(indexTree(before.tree, before.text), indexTree(swapped.tree, swapped.text), { opened: [], pinned: [a.key] });
    assert.notEqual(state.pinned[0], a.key);
    const duplicate = await treeOf([{ uri: "nx:///items.nx", source: source.replace('name="b"', 'name="a"') }]);
    const added = await treeOf([{ uri: "nx:///items.nx", source: duplicate.text.replace(/}$/, '<Item name="a"/>\n}') }]);
    const ambiguous = reconcileDetails(indexTree(duplicate.tree, duplicate.text), indexTree(added.tree, added.text), { opened: [], pinned: [a.key] });
    assert.deepEqual(ambiguous.pinned, []);
  });

  it("ignores comments when reconciling list identities", async () => {
    const source = 'type Item = { name:string }\nlet root() = {\n<Item name="a"/>\n<Item name="b"/>\n}';
    const before = await treeOf([{ uri: "nx:///comments.nx", source }]);
    const after = await treeOf([{ uri: "nx:///comments.nx", source: source.replace('<Item name="b"/>', '// A note\n<Item name="b"/>') }]);
    const key = before.tree.nodes.find((node) => node.role === "element")!.key;
    const state = reconcileDetails(indexTree(before.tree, before.text), indexTree(after.tree, after.text), { opened: [], pinned: [key] });
    assert.deepEqual(state.pinned, [key]);
  });

  it("renders every fixture node once with all compact details disclosed", async () => {
    for (const name of ["tasks", "groups", "elementIds", "icons", "uitext"]) {
      const { tree, text } = await fixture(name);
      const keys = tree.nodes.filter((node) => node.role === "element").map((node) => node.key);
      const rendered = renderSource(document, indexTree(tree, text), [], { compact: true, details: new Set(keys) });
      const counts = new Map<string, number>();
      rendered.querySelectorAll<HTMLElement>("[data-key]").forEach((node) => counts.set(node.dataset["key"]!, (counts.get(node.dataset["key"]!) ?? 0) + 1));
      for (const node of tree.nodes) {
        assert.equal(counts.get(node.key), 1, `${name}: ${node.key}`);
      }
    }
  });
});
