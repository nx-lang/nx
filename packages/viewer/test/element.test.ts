import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it } from "node:test";

import { fixture, nxHost, type Fixture } from "./fixtures.js";
import type { NxSelectDetail, NxViewerElement } from "../src/index.js";

const { NX_SELECT_EVENT, NxViewerElement: ViewerClass, nodeAtSpan } = await import("../src/index.js");

const created: NxViewerElement[] = [];

after(async () => {
  for (const element of created) {
    element.remove();
  }
  (await nxHost()).dispose();
});

function show({ tree, text }: Fixture): NxViewerElement {
  const element = document.createElement("nx-viewer");
  document.body.append(element);
  element.text = text;
  element.tree = tree;
  created.push(element);
  return element;
}

function root(element: NxViewerElement): ShadowRoot {
  assert.ok(element.shadowRoot, "the element has an open shadow root");
  return element.shadowRoot;
}

/** The original rendering of the node with `key`. */
function rendered(element: NxViewerElement, key: string): HTMLElement {
  const found = Array.from(root(element).querySelectorAll<HTMLElement>("[data-key]")).find(
    (candidate) => candidate.dataset["key"] === key
  );
  assert.ok(found, `the node ${key} is rendered`);
  return found;
}

function keyWhere(fixture: Fixture, test: (node: Fixture["tree"]["nodes"][number]) => boolean): string {
  const node = fixture.tree.nodes.find(test);
  assert.ok(node, "the fixture has the node the test looks for");
  return node.key;
}

/** Text with whitespace folded, as a reader sees it. */
function read(element: Element): string {
  return (element.textContent ?? "").replace(/\s+/g, " ").trim();
}

describe("<nx-viewer>", () => {
  it("shows a document's reading on a plain page", async () => {
    const element = show(await fixture("tasks"));
    assert.ok(root(element).querySelector("pre.source"), "the reading is rendered");
    assert.equal(root(element).querySelectorAll(".line-element").length, 2, "elements on one line read as lines");
  });

  it("keeps its markup and styles in a shadow root", async () => {
    const element = show(await fixture("user"));
    assert.equal(document.querySelectorAll(".source").length, 0, "nothing leaks into the page");
    assert.ok(root(element).querySelector("style"), "the element's styles are in its shadow root");
    assert.equal(rendered(element, "User").getRootNode(), root(element), "the rendering lives in the shadow root");
  });

  it("mutes and badges a stale tree, and still selects", async () => {
    const element = show(await fixture("tasks"));
    element.stale = true;
    assert.equal(element.getAttribute("stale"), "");
    assert.equal(root(element).querySelector<HTMLElement>(".badge")!.hidden, false);
    const line = root(element).querySelector<HTMLElement>(".line-element")!;
    line.click();
    assert.equal(element.selection, line.dataset["key"]);
  });

  it("takes properties set before the element was defined", async () => {
    const { tree, text } = await fixture("tasks");
    const tag = "nx-viewer-defined-late";
    const element = document.createElement(tag) as NxViewerElement;
    document.body.append(element);
    Object.assign(element, { text, tree, stale: true });
    customElements.define(tag, class extends ViewerClass {});
    created.push(element);
    assert.ok(element instanceof ViewerClass, "the element upgraded");
    assert.equal(element.tree, tree);
    assert.ok(root(element).querySelector(".line-element"), "the tree set early is rendered");
    assert.equal(element.hasAttribute("stale"), true);
    assert.equal(root(element).querySelector<HTMLElement>(".badge")!.hidden, false, "the stale badge shows");
  });
});

describe("the reading", () => {
  it("reads code that is not an element as written, with every node marked and none twice", async () => {
    const tasks = await fixture("tasks");
    const element = show(tasks);
    const source = root(element).querySelector<HTMLElement>("pre.source")!;
    assert.ok(source.textContent!.startsWith(tasks.text.slice(0, tasks.text.indexOf("<Task"))), "the type and the function's head read as written");
    assert.ok(source.textContent!.includes("\n  Task  title=Ship the site  priority=1\n"), "an element on one line reads as its name and properties");
    const keys = Array.from(source.querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
    assert.equal(new Set(keys).size, keys.length);
    assert.deepEqual([...keys].sort(), tasks.tree.nodes.map((node) => node.key).sort(), "every node has its span");
    const type = rendered(element, "Task");
    assert.ok(type.textContent!.startsWith("type Task = {"));
  });

  it("colors the text by the ranges its host gives, and selects a node without an outline", async () => {
    const tasks = await fixture("tasks");
    const element = show(tasks);
    const start = tasks.text.indexOf("Task");
    Object.assign(element, {
      colors: [{ start, end: start + 4, light: "#111111", dark: "#eeeeee", fontStyle: 2 }],
    });
    const colored = root(element).querySelector<HTMLElement>(".source .t")!;
    assert.equal(colored.textContent, "Task");
    assert.equal(colored.style.getPropertyValue("--l"), "#111111");
    assert.equal(colored.style.getPropertyValue("--d"), "#eeeeee");
    assert.equal(colored.style.fontWeight, "bold");

    const events: string[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail.key));
    rendered(element, "Task.title").click();
    assert.deepEqual(events, ["Task.title"], "a field of a type is selected on its own");
    assert.ok(rendered(element, "Task.title").classList.contains("selected"));
  });

  it("marks prose and indentation, so a host can set each in a font of its own", async () => {
    const tasks = await fixture("tasks");
    const element = show(tasks);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.querySelector(".prose"), null, "a string is code unless its type says a person reads it");
    const indents = Array.from(source.querySelectorAll(".indent"), (indent) => indent.textContent!);
    assert.ok(indents.length > 0 && indents.every((indent) => /^[ \t]+$/.test(indent)), "indentation is whitespace only");
    assert.equal(indents.join("").length, (tasks.text.match(/^[ \t]+/gm) ?? []).join("").length, "every line's is marked");
  });

  it("sets text shown to people apart, by the type of what holds it and never by how it looks", async () => {
    const fixtureText = await fixture("uitext");
    const element = show(fixtureText);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.deepEqual(Array.from(source.querySelectorAll(".prose"), read), ["// A note for whoever reads the source."]);
    const shown = Array.from(source.querySelectorAll<HTMLElement>(".uitext"), (node) => node.textContent);
    assert.deepEqual(shown, [
      '"Fine"', // a field's default
      '"Hello, "', // in a function that returns uitext
      "First choice", // a uitext property, on an element's line without its quotes
      "Pick me", // a property whose type is an alias of uitext
      "Shown to people", // an element's text, bound to a uitext content property
      '"Second "', // inside an expression a uitext property holds
      '"choice"',
    ]);
    assert.ok(!shown.includes("a") && !shown.includes('"plain"'), "a string property's value stays code");
    const fine = Array.from(source.querySelectorAll<HTMLElement>(".uitext")).find((node) => node.textContent === '"Fine"')!;
    assert.equal(fine.querySelectorAll(".uitext-quote").length, 2, "written with its quotes, it sets them apart from its words");
    const block = fine.querySelector<HTMLElement>(".uitext-block")!;
    assert.equal(block.style.getPropertyValue("--column"), String(fixtureText.text.split("\n")[3]!.indexOf('"Fine"') + 1), "the words' column is known");
  });

  it("sets a string its viewer formats as an identifier apart as one quiet token", async () => {
    const identifiers = await fixture("identifiers");
    const element = show(identifiers);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const tokens = Array.from(source.querySelectorAll<HTMLElement>(".id-value"), (node) => node.textContent);
    assert.deepEqual(tokens, ["welcome"], "only the property the viewer names, without its quotes; next is a plain string");
  });

  it("shows the color a string its viewer formats as a color names, as a square before it", async () => {
    const colors = await fixture("colors");
    const element = show(colors);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const squares = Array.from(source.querySelectorAll<HTMLElement>(".swatch"));
    assert.equal(squares.length, 1, "only a color the type's viewer formats, and only one that names a color");
    assert.equal(squares[0]!.parentElement!.textContent, "#b6695d");
    assert.equal(squares[0]!.getAttribute("aria-hidden"), "true");
  });

  it("starts an element on a line of its own with the icon its viewer names, or its kind's", async () => {
    const icons = await fixture("icons");
    const element = show(icons);
    const source = (): HTMLElement => root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source().querySelector(".element-icon"), null, "no icons until the host draws them");
    const asked: string[] = [];
    element.icons = (name) => {
      asked.push(name);
      const drawn = element.ownerDocument.createElementNS("http://www.w3.org/2000/svg", "svg");
      drawn.dataset["icon"] = name;
      return drawn;
    };
    const marks = Array.from(source().querySelectorAll<HTMLElement>(".element-icon"));
    assert.deepEqual(
      marks.map((mark) => [mark.getAttribute("aria-label"), mark.querySelector("svg")!.dataset["icon"]]),
      [["Choice", "circle-dot"], ["Note", "box"], ["Choice", "circle-dot"]],
      "the viewer's icon, then a record's; an element sharing its line has none"
    );
    assert.ok(asked.includes("circle-dot"));
    assert.equal(marks[0]!.textContent, "", "an icon adds no text");
    const names = Array.from(source().querySelectorAll<HTMLElement>(".line-element"), (line) => line.querySelector(".line-kind")?.textContent);
    assert.deepEqual(names, ["ElementViewer", "Choice", "Note", "Note", undefined], "a place typed Choice+ leaves its Choice unnamed; its icon names it");
  });

  it("reads an element written over several lines as a group, without its tags", async () => {
    const groups = await fixture("groups");
    const element = show(groups);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const sheet = source.querySelector<HTMLElement>(".sheet")!;
    assert.ok(sheet, "the element reads as a group");
    const head = sheet.querySelector<HTMLElement>(".sheet-head")!;
    assert.equal(head.textContent, "Row  gap=wide", "its first line's attributes follow its name");
    assert.ok(!source.textContent!.includes("</Row>"), "no closing tag");
    const items = Array.from(sheet.querySelectorAll<HTMLElement>(".sheet-item"));
    assert.deepEqual(items.map((item) => item.textContent), ["Item  name=a", "Item  name=b"]);
    assert.ok(items[1]!.classList.contains("after-blank"), "a blank line between items is kept as room");
  });

  it("reads an element on one line that holds content without its tags", async () => {
    const content = await fixture("content");
    const element = show(content);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const line = source.querySelector<HTMLElement>(".line-element")!;
    assert.equal(line.textContent, "Label  tone=quiet  Hello {who}", "its properties, then what it holds as written");
    assert.ok(!source.textContent!.includes("</Label>"), "no closing tag");
  });

  it("moves an element's id to where its viewer puts it, as its value alone", async () => {
    const ids = await fixture("elementIds");
    const element = show(ids);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    // At the beginning, after the group's name; at the end, after the line's title.
    assert.equal(read(source.querySelector(".sheet-head")!), "Step welcome");
    assert.equal(read(source.querySelector(".sheet + * .line-element, .sheet-item:last-child .line-element") ?? source.querySelectorAll(".line-element")[2]!), "Choice Yes yes");
    const chips = Array.from(source.querySelectorAll<HTMLElement>(".id-part"));
    assert.deepEqual(chips.map((chip) => chip.textContent), ["welcome", "yes"]);
    // Each chip keeps the keys of the attribute and its value, so selection still finds them.
    const keys = Array.from(source.querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
    assert.deepEqual([...keys].sort(), ids.tree.nodes.map((node) => node.key).sort(), "every node is still there once");
  });

  it("reads an element written an attribute to a line as a group with a row to each", async () => {
    const ids = await fixture("elementIds");
    const element = show(ids);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const sheet = source.querySelector<HTMLElement>(".sheet")!;
    assert.ok(sheet, "the Step written over several lines is a sheet");
    assert.equal(read(sheet.querySelector(".sheet-head")!), "Step welcome", "its name and id, with no angle bracket");
    const rows = Array.from(sheet.querySelectorAll<HTMLElement>(".sheet-row"));
    assert.deepEqual(rows.map((row) => [read(row.querySelector(".sheet-name")!), read(row.querySelector(".sheet-value")!)]), [["title", "Welcome"]]);
    assert.ok(!sheet.textContent!.includes("/>") && !sheet.textContent!.includes('"'), "no closer, no quotes");
    assert.equal(source.querySelectorAll(".sheet").length, 1, "the Choice written on one line stays a line");
    const keys = Array.from(source.querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
    assert.deepEqual([...keys].sort(), ids.tree.nodes.map((node) => node.key).sort(), "every node is still there once");
  });

  it("keeps offsets right after a character outside the basic plane", async () => {
    const emoji = await fixture("emoji");
    const element = show(emoji);
    const line = root(element).querySelector<HTMLElement>(".line-element")!;
    assert.equal(line.textContent, "<Label  text=😀 hello  title=after/>", "the value after the emoji is cut where it begins");
  });
});

describe("selection", () => {
  it("selects the unit clicked, or outside every unit the smallest node, and reports it", async () => {
    const tasks = await fixture("tasks");
    const element = show(tasks);
    const events: NxSelectDetail[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail));
    const line = root(element).querySelector<HTMLElement>(".line-element")!;
    line.querySelector<HTMLElement>(".line-property")!.click();
    assert.equal(events.at(-1)!.key, line.dataset["key"], "a word of a line chooses the line");
    assert.equal(events.at(-1)!.role, "element");
    assert.match(root(element).querySelector('[role="status"]')!.textContent ?? "", /^Selected Task/);
    rendered(element, "Task.title").click();
    assert.equal(events.at(-1)!.key, "Task.title", "a field of a type is the smallest node there");
    assert.ok(rendered(element, "Task.title").classList.contains("selected"));
  });

  it("selects from the host without an event", async () => {
    const element = show(await fixture("user"));
    let fired = 0;
    element.addEventListener(NX_SELECT_EVENT, () => (fired += 1));
    element.selection = "User.id";
    assert.equal(fired, 0);
    assert.equal(element.selection, "User.id");
    assert.ok(rendered(element, "User.id").classList.contains("selected"));
  });

  it("clears the selection for a key the tree does not hold", async () => {
    const element = show(await fixture("user"));
    element.selection = "User.id";
    element.selection = "Nothing.here";
    assert.equal(element.selection, undefined);
    assert.equal(root(element).querySelectorAll(".selected").length, 0);
  });

  it("marks the selected node for assistive technology", async () => {
    const element = show(await fixture("user"));
    element.selection = "User.id";
    assert.equal(rendered(element, "User.id").getAttribute("aria-current"), "true");
    element.selection = "User.name";
    assert.equal(rendered(element, "User.id").hasAttribute("aria-current"), false);
  });

  it("finds the node a value origin names", async () => {
    const flow = await fixture("questionFlow");
    const at = flow.tree.nodes.findIndex((node) => node.key === "roleQuestion.value");
    const { startByte, endByte } = flow.tree.nodes[at]!.range;
    assert.equal(nodeAtSpan(flow.tree, startByte, endByte), at);
    assert.equal(nodeAtSpan(flow.tree, startByte, endByte + 1), undefined);
  });
});

describe("the keyboard", () => {
  function key(target: Element, name: string): void {
    target.dispatchEvent(new document.defaultView!.KeyboardEvent("keydown", { key: name, bubbles: true, composed: true }));
  }

  it("selects the reading's elements and rows, and walks them as a tree", async () => {
    const element = show(await fixture("groups"));
    const events: string[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail.key));
    const body = root(element).querySelector<HTMLElement>(".body")!;
    const sheet = body.querySelector<HTMLElement>(".sheet")!;
    const [first, second] = Array.from(body.querySelectorAll<HTMLElement>(".line-element"));
    // A click on a word of a line chooses the line, and one on the header, its element.
    first!.querySelector<HTMLElement>(".line-property")!.click();
    sheet.querySelector<HTMLElement>(".sheet-head .line-name")!.click();
    key(body, "ArrowRight");
    key(body, "ArrowDown");
    key(body, "ArrowLeft");
    assert.deepEqual(events, [first!.dataset["key"], sheet.dataset["key"], first!.dataset["key"], second!.dataset["key"], sheet.dataset["key"]]);
    assert.ok(body.querySelector(".highlights"), "the selection is drawn behind the text");
  });

  it("takes focus and announces what the arrow keys select", async () => {
    const element = show(await fixture("tasks"));
    const body = root(element).querySelector<HTMLElement>(".body")!;
    assert.equal(body.tabIndex, 0, "the reading takes focus");
    assert.equal(body.getAttribute("role"), "region");
    key(body, "ArrowDown");
    const announcer = root(element).querySelector('[role="status"]')!;
    assert.equal(announcer.getAttribute("aria-live"), "polite");
    assert.match(announcer.textContent ?? "", /^Selected Task/, "the selection is announced");
  });
});
