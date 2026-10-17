import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it, type TestContext } from "node:test";

import { fixture, nxHost, type Fixture } from "./fixtures.js";
import type { NxFoldDetail, NxSelectDetail, NxViewerElement } from "../src/index.js";

const { NX_FOLD_EVENT, NX_SELECT_EVENT, NxViewerElement: ViewerClass, nodeAtSpan } = await import("../src/index.js");

const created: NxViewerElement[] = [];

after(async () => {
  for (const element of created) {
    element.remove();
  }
  (await nxHost()).dispose();
});

/** The element showing `fixture`, everything unfolded unless `defaults` asks for its own folding. */
function show({ tree, text }: Fixture, defaults = false): NxViewerElement {
  const element = document.createElement("nx-viewer");
  document.body.append(element);
  if (!defaults) {
    element.folded = [];
  }
  element.text = text;
  element.tree = tree;
  created.push(element);
  return element;
}

/**
 * Lays the reading out as jsdom does not, for the test `t`: a line or a group's header of more than
 * `columns` characters takes two lines, so its element gives each property a row.
 */
function narrow(t: TestContext, columns: number): void {
  const prototype = document.defaultView!.HTMLElement.prototype;
  const { getClientRects } = prototype;
  Object.defineProperty(prototype, "clientWidth", { configurable: true, get: () => columns * 8 });
  // What runs past its line, as a folded group's header does rather than wrap.
  Object.defineProperty(prototype, "scrollWidth", {
    configurable: true,
    get(this: HTMLElement) {
      return (this.textContent ?? "").replace(/\s+/g, " ").length * 8;
    }
  });
  prototype.getClientRects = function (this: HTMLElement) {
    // A folded group's header stays on one line however long, as its style keeps it.
    const long = (this.textContent ?? "").replace(/\s+/g, " ").length > columns && !this.matches(".folded > .sheet-head");
    const height = long ? 36 : 18;
    return [{ top: 0, bottom: height, left: 0, right: 100, width: 100, height, x: 0, y: 0, toJSON: () => ({}) }] as unknown as DOMRectList;
  };
  t.after(() => {
    prototype.getClientRects = getClientRects;
    delete (prototype as { clientWidth?: number }).clientWidth;
    delete (prototype as { scrollWidth?: number }).scrollWidth;
  });
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

  it("sets a string its visualizer formats as an identifier apart as one quiet token", async () => {
    const identifiers = await fixture("identifiers");
    const element = show(identifiers);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const tokens = Array.from(source.querySelectorAll<HTMLElement>(".id-value"), (node) => node.textContent);
    assert.deepEqual(tokens, ["welcome"], "only the property the visualizer names, without its quotes; next is a plain string");
  });

  it("shows the color a string its visualizer formats as a color names, as a square before it", async () => {
    const colors = await fixture("colors");
    const element = show(colors);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const squares = Array.from(source.querySelectorAll<HTMLElement>(".swatch"));
    assert.equal(squares.length, 1, "only a color the type's visualizer formats, and only one that names a color");
    assert.equal(squares[0]!.parentElement!.textContent, "#b6695d");
    assert.equal(squares[0]!.getAttribute("aria-hidden"), "true");
  });

  it("reads text shown to people and a color swatch through aliases another module declares", async () => {
    // As a host's catalog does: the aliases in one module, the color format in a host visualizer
    // module, and the document importing only the first.
    const imported = await fixture("importedAliases");
    const element = show(imported);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.deepEqual(
      Array.from(source.querySelectorAll<HTMLElement>(".uitext-body"), (node) => node.textContent),
      ["Welcome back"]
    );
    const squares = Array.from(source.querySelectorAll<HTMLElement>(".swatch"));
    assert.equal(squares.length, 1);
    assert.equal(squares[0]!.parentElement!.textContent, "#2E7D32");
  });

  it("starts an element on a line of its own with the icon its visualizer names, or its kind's", async () => {
    const icons = await fixture("icons");
    const element = show(icons);
    const source = (): HTMLElement => root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source().querySelector(".element-icon"), null, "no icons until the host draws them");
    const asked: string[] = [];
    element.icons = (name) => {
      asked.push(name);
      if (name === "no-such-icon") {
        return undefined;
      }
      const drawn = element.ownerDocument.createElementNS("http://www.w3.org/2000/svg", "svg");
      drawn.dataset["icon"] = name;
      return drawn;
    };
    const marks = Array.from(source().querySelectorAll<HTMLElement>(".element-icon"));
    assert.deepEqual(
      marks.map((mark) => [mark.getAttribute("aria-label"), mark.querySelector("svg")!.dataset["icon"]]),
      [["Choice", "circle-dot"], ["Note", "box"], ["Tip", "box"], ["Status.closed", "tag"], ["Choice", "circle-dot"]],
      "the visualizer's icon, a record's, a record's for an icon the host cannot draw, and a union case's; an element sharing its line has none"
    );
    assert.ok(asked.includes("circle-dot") && asked.includes("no-such-icon"));
    assert.equal(marks[0]!.textContent, "", "an icon adds no text");
    const names = Array.from(source().querySelectorAll<HTMLElement>(".line-element"), (line) => line.querySelector(".line-kind")?.textContent);
    assert.deepEqual(names, ["ElementVisualizer", "ElementVisualizer", "Choice", "Note", "Tip", "Status.closed", "Note", undefined], "a place typed Choice+ leaves its Choice unnamed; its icon names it");
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

  it("keeps an element with one property and the text it holds a line, however narrow the pane", async (t) => {
    narrow(t, 10);
    const element = show(await fixture("content"));
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.querySelector(".sheet"), null, "a row would not shorten it");
    assert.equal(source.querySelector<HTMLElement>(".line-element")!.textContent, "Label  tone=quiet  Hello {who}");
  });

  it("measures the reading again once the pane's width settles, not on each step", async (t) => {
    let width = 400;
    const prototype = document.defaultView!.HTMLElement.prototype;
    Object.defineProperty(prototype, "clientWidth", { configurable: true, get: () => width });
    const resized: (() => void)[] = [];
    const observer = Object.getOwnPropertyDescriptor(globalThis, "ResizeObserver");
    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      writable: true,
      value: class {
        constructor(callback: () => void) {
          resized.push(callback);
        }
        observe(): void {}
        disconnect(): void {}
      }
    });
    t.mock.timers.enable({ apis: ["setTimeout"] });
    t.after(() => {
      delete (prototype as { clientWidth?: number }).clientWidth;
      if (observer === undefined) {
        delete (globalThis as { ResizeObserver?: unknown }).ResizeObserver;
      } else {
        Object.defineProperty(globalThis, "ResizeObserver", observer);
      }
    });
    const element = show(await fixture("groups"));
    const resize = resized.at(-1)!;
    const drawn = (): Element | null => root(element).querySelector(".source");
    const first = drawn();
    for (const step of [380, 360, 340]) {
      width = step;
      resize();
    }
    assert.equal(drawn(), first, "not drawn again while the width is changing");
    t.mock.timers.tick(150);
    const second = drawn();
    assert.notEqual(second, first, "drawn again once it settles");
    t.mock.timers.tick(1000);
    assert.equal(drawn(), second, "and only once");
  });

  it("moves an element's id to where its visualizer puts it, as its value alone", async () => {
    const ids = await fixture("elementIds");
    const element = show(ids);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    // At the beginning, after the name; at the end, after the title.
    const lines = Array.from(source.querySelectorAll<HTMLElement>('[data-key="steps"] .line-element'), (line) => read(line));
    assert.deepEqual(lines, ["Step welcome Welcome", "Choice Yes yes"]);
    const chips = Array.from(source.querySelectorAll<HTMLElement>(".id-part"));
    assert.deepEqual(chips.map((chip) => chip.textContent), ["welcome", "yes"]);
    // Each chip keeps the keys of the attribute and its value, so selection still finds them.
    const keys = Array.from(source.querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
    assert.deepEqual([...keys].sort(), ids.tree.nodes.map((node) => node.key).sort(), "every node is still there once");
  });

  it("reads an element whose properties fit on one line as a line, however the source breaks it", async () => {
    const ids = await fixture("elementIds");
    const element = show(ids);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.querySelectorAll(".sheet").length, 0, "the Step written an attribute to a line is no group");
    const step = source.querySelector<HTMLElement>('[data-key="steps"] .line-element')!;
    assert.equal(read(step), "Step welcome Welcome", "its name, id and title, with no angle bracket");
    assert.ok(!step.textContent!.includes("/>") && !step.textContent!.includes('"'), "no closer, no quotes");
    const keys = Array.from(source.querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
    assert.deepEqual([...keys].sort(), ids.tree.nodes.map((node) => node.key).sort(), "every node is still there once");
  });

  it("puts a group's properties on its header when they fit there, however the source breaks them", async () => {
    const wrapped = await fixture("wrappedAttributes");
    const element = show(wrapped);
    const sheet = root(element).querySelector<HTMLElement>(".source .sheet")!;
    assert.equal(read(sheet.querySelector(".sheet-head")!), "Shape kind=circle width=52 color=#6652AD stroke=1");
    assert.equal(sheet.querySelectorAll(".sheet-row").length, 0, "no rows");
    assert.equal(read(sheet.querySelector(".sheet-held .line-element")!), "Item name=a note=n", "what it holds reads under it");
    assert.ok(!sheet.textContent!.includes("</Shape>"), "no closing tag");
  });

  it("gives each property a row where an element's properties do not fit on one line", async (t) => {
    const wrapped = await fixture("wrappedAttributes");
    narrow(t, 30);
    const element = show(wrapped);
    const sheets = Array.from(root(element).querySelectorAll<HTMLElement>(".source .sheet"));
    assert.equal(read(sheets[0]!.querySelector(".sheet-head")!), "Shape", "too long for its header: a row to each");
    const rows = Array.from(sheets[0]!.querySelectorAll<HTMLElement>(":scope > .sheet-row"));
    assert.deepEqual(rows.map((row) => [read(row.querySelector(".sheet-name")!), read(row.querySelector(".sheet-value")!)]), [
      ["kind", "circle"],
      ["width", "52"],
      ["color", "#6652AD"],
      ["stroke", "1"],
    ]);
    assert.equal(read(sheets[0]!.querySelector(".sheet-held .line-element")!), "Item name=a note=n", "short enough for a line");
    assert.equal(read(sheets[1]!.querySelector(".sheet-head")!), "Shape kind=square", "one property needs no row");
  });

  it("folds a group whose properties do not fit to its name and what it hides, not its properties", async (t) => {
    const wrapped = await fixture("wrappedAttributes");
    narrow(t, 30);
    const element = show(wrapped);
    const shape = root(element).querySelector<HTMLElement>(".source .sheet")!.dataset["key"]!;
    element.folded = [shape];
    const sheet = root(element).querySelector<HTMLElement>(`[data-key="${shape}"]`)!;
    assert.ok(sheet.classList.contains("folded"));
    assert.equal(read(sheet.querySelector(".sheet-head")!), "Shape 4 properties · 1 child", "a header cut, not wrapped, still measured");
  });

  it("keeps a folded group's properties on its header when they fit there", async (t) => {
    const wrapped = await fixture("wrappedAttributes");
    narrow(t, 60);
    const element = show(wrapped);
    const shape = root(element).querySelector<HTMLElement>(".source .sheet")!.dataset["key"]!;
    element.folded = [shape];
    const head = root(element).querySelector<HTMLElement>(`[data-key="${shape}"] > .sheet-head`)!;
    assert.equal(read(head), "Shape kind=circle width=52 color=#6652AD stroke=1 1 child");
  });

  it("moves a folded group's one property to a row when its header does not fit", async (t) => {
    const wrapped = await fixture("wrappedAttributes");
    narrow(t, 10);
    const element = show(wrapped);
    const shape = wrapped.tree.nodes.filter((node) => node.role === "element" && node.name === "Shape")[1]!.key;
    element.folded = [shape];
    const head = root(element).querySelector<HTMLElement>(`[data-key="${shape}"] > .sheet-head`)!;
    assert.equal(read(head), "Shape 1 property · 1 child", "cut, not wrapped, it would run off the pane");
  });

  it("counts a folded group's children as it shows them open, every branch of an if and a for's body", async () => {
    const counts = await fixture("childCounts");
    const element = show(counts);
    const panels = counts.tree.nodes.filter((node) => node.role === "element" && node.name === "Panel").map((node) => node.key);
    element.folded = panels;
    const head = (key: string): string => read(root(element).querySelector(`[data-key="${key}"] > .sheet-head`)!);
    // The Item, the if's three, the for's one and the match's two: not the note.
    assert.equal(head(panels[0]!), "Panel 7 children");
    assert.equal(head(panels[1]!), "Panel 2 children", "an expression giving elements is one");
  });

  it("keeps an element written as its tags over several lines in its item's lines", async () => {
    const wrapped = await fixture("wrappedAttributes");
    const element = show(wrapped);
    // An element written as its tags is a unit of its own, as a tag line, where the host draws icons.
    element.icons = () => element.ownerDocument.createElementNS("http://www.w3.org/2000/svg", "svg");
    const written = root(element).querySelector<HTMLElement>(".sheet-item.as-written .tag-line")!;
    assert.ok(written, "the Item with a comment in an attribute reads as written");
    assert.ok(written.classList.contains("over-lines"), "it is marked as written over several lines, so it stays inline");
    assert.ok(written.textContent!.includes('\n      note="n"'), "its later line keeps the source's indentation");
  });

  it("puts an id last in a group's header when its visualizer puts it at the end", async (t) => {
    const ids = await fixture("groupIdAtEnd");
    narrow(t, 20);
    const element = show(ids);
    const head = root(element).querySelector<HTMLElement>('[data-key="choice"] .sheet-head')!;
    assert.equal(read(head), "Choice yes", "after the name, as the properties have rows");
    assert.ok(head.lastElementChild!.classList.contains("id-part"), "the id closes the header");
  });

  it("keeps an element's text type with its name", async () => {
    const texts = await fixture("textTypes");
    const element = show(texts);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(read(source.querySelector(".line-element")!), "<Note:markdown Hello *there*/>");
    assert.equal(read(source.querySelector(".sheet-head")!), "Note:markdown");
  });

  it("reads an element with a comment inside an attribute as written, losing no text and no node", async () => {
    const comments = await fixture("attributeComments");
    const element = show(comments);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    for (const comment of ["/* why */", "/* c */", "/* key */"]) {
      assert.ok(source.textContent!.includes(comment), `${comment} is kept`);
    }
    assert.equal(source.querySelectorAll(".sheet, .id-part").length, 0, "no group, and the id stays in place");
    const keys = Array.from(source.querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
    assert.deepEqual([...keys].sort(), comments.tree.nodes.map((node) => node.key).sort(), "every node is there once");
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

  it("tells a host what starts at the top of what scrolls it, and scrolls a node to where it asks", async () => {
    const groups = await fixture("groups");
    const port = document.createElement("div");
    port.style.overflowY = "auto";
    document.body.append(port);
    const element = document.createElement("nx-viewer");
    port.append(element);
    created.push(element);
    element.folded = [];
    element.text = groups.text;
    element.tree = groups.tree;
    // jsdom lays nothing out: each node is drawn a line of 18px below the one before it, in the
    // order the reading holds them, and the port shows 100px of them from where it is scrolled to.
    let scrolled = 0;
    const prototype = document.defaultView!.HTMLElement.prototype;
    const { getBoundingClientRect, getClientRects } = prototype;
    const lineOf = (target: HTMLElement): number =>
      Array.from(root(element).querySelectorAll("[data-key]")).indexOf(target);
    Object.defineProperty(port, "clientHeight", { value: 100 });
    port.scrollBy = ((options: ScrollToOptions) => (scrolled += options.top ?? 0)) as typeof port.scrollBy;
    prototype.getBoundingClientRect = function (this: HTMLElement) {
      const line = this === port ? -1 : lineOf(this);
      const top = this === port ? 0 : line * 18 - scrolled;
      return { top, bottom: top + 18, left: 0, right: 100, width: 100, height: 18, x: 0, y: top, toJSON: () => ({}) } as DOMRect;
    };
    prototype.getClientRects = function (this: HTMLElement) {
      return (lineOf(this) >= 0 ? [this.getBoundingClientRect()] : []) as unknown as DOMRectList;
    };
    try {
      const keys = Array.from(root(element).querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
      assert.deepEqual(element.keyAtTop(), { key: keys[0], top: 0 });
      element.scrollToKey(keys[5]!, 36);
      assert.equal(scrolled, 5 * 18 - 36);
      assert.equal(element.topOf(keys[5]!), 36);
      assert.deepEqual(element.keyAtTop(), { key: keys[3], top: 0 }, "the first node at or below the top");
      // Folded, a group draws none of what it holds: a node in it is found where its group is.
      const row = keyWhere(groups, (node) => node.role === "element" && node.name === "Row");
      const item = keyWhere(groups, (node) => node.role === "element" && node.name === "Item");
      element.folded = [row];
      assert.equal(root(element).querySelector(`[data-key="${item}"]`), null, "the item is not drawn");
      assert.equal(element.topOf(item), element.topOf(row));
      assert.equal(element.topOf("Nothing.here"), undefined);
    } finally {
      prototype.getBoundingClientRect = getBoundingClientRect;
      prototype.getClientRects = getClientRects;
      port.remove();
    }
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

  it("moves from the unit that holds a selection the host makes inside it", async (t) => {
    const rows = await fixture("rows");
    narrow(t, 10);
    const element = show(rows);
    const events: string[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail.key));
    const body = root(element).querySelector<HTMLElement>(".body")!;
    const z = keyWhere(rows, (node) => node.role === "literal" && node.value === '"z"');
    const attribute = (name: string): string => keyWhere(rows, (node) => node.role === "attribute" && node.name === name);
    const item = keyWhere(rows, (node) => node.role === "element" && node.name === "Item");
    const panel = keyWhere(rows, (node) => node.role === "element" && node.name === "Panel");
    for (const move of ["ArrowLeft", "ArrowDown", "ArrowUp"]) {
      element.selection = z;
      key(body, move);
    }
    assert.deepEqual(events, [attribute("note"), item, attribute("title")], "the row holding it, then the units around that row");
    // Outside every unit, as a type's field, down selects the first unit after it, and up finds
    // none.
    element.selection = keyWhere(rows, (node) => node.role === "field" && node.name === "name");
    key(body, "ArrowUp");
    key(body, "ArrowDown");
    assert.deepEqual(events.slice(3), [panel]);
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

describe("folding", () => {
  it("folds the document's supporting declarations to one line until the reader opens them", async (t) => {
    const folding = await fixture("folding");
    narrow(t, 10);
    const element = show(folding, true);
    const source = (): HTMLElement => root(element).querySelector<HTMLElement>(".source")!;
    const run = source().querySelector<HTMLElement>(".fold-run")!;
    assert.ok(run.classList.contains("folded"), "everything before the value is one folded run");
    assert.equal(run.textContent!.trim(), "1 import · 2 types · 1 function · 1 visualizer");
    assert.ok(source().textContent!.includes("let tasks"), "the value is open");
    assert.ok(source().querySelector(".sheet:not(.folded)"), "and so is its group");
    // Opening the run shows each declaration folded to its first line.
    const folds: (readonly string[])[] = [];
    element.addEventListener(NX_FOLD_EVENT, (event) => folds.push((event as CustomEvent<NxFoldDetail>).detail.folded));
    run.querySelector<HTMLElement>(".fold-run-head")!.click();
    assert.ok(!source().querySelector(".fold-run")!.classList.contains("folded"));
    const declaration = source().querySelector<HTMLElement>(".fold-declaration.folded")!;
    assert.equal(declaration.textContent, "type Task = { …");
    assert.equal(folds.length, 1, "the host hears of it");
    assert.ok(!folds[0]!.includes(run.querySelector<HTMLElement>(".fold")!.dataset["foldKey"]!));
  });

  it("folds a group to its header and title, and selecting inside it unfolds it", async (t) => {
    const folding = await fixture("folding");
    narrow(t, 10);
    const element = show(folding);
    const group = (): HTMLElement => root(element).querySelector<HTMLElement>(".sheet")!;
    group().querySelector<HTMLElement>(":scope > .sheet-head > .fold")!.click();
    assert.ok(group().classList.contains("folded"));
    assert.equal(group().querySelector(".fold-title")!.textContent, "Ship the site", "its title's value says what it is");
    assert.equal(group().querySelector(".sheet-row"), null, "its rows are not drawn");
    const priority = folding.tree.nodes.find((node) => node.role === "attribute" && node.name === "priority")!;
    element.selection = priority.key;
    assert.ok(!group().classList.contains("folded"), "a selection inside a folded group opens it");
    // From the keyboard, left folds the selected group and right opens it, as a tree's nodes do.
    element.selection = group().dataset["key"];
    const body = root(element).querySelector<HTMLElement>(".body")!;
    const press = (key: string): void => {
      body.dispatchEvent(new document.defaultView!.KeyboardEvent("keydown", { key, bubbles: true, composed: true }));
    };
    press("ArrowLeft");
    assert.ok(group().classList.contains("folded"), "left folds it");
    press("ArrowRight");
    assert.ok(!group().classList.contains("folded"), "right opens it");
    element.collapseAll();
    assert.ok(element.folded.length >= 3, "everything that folds is folded");
    element.expandAll();
    assert.deepEqual(element.folded, []);
  });

  it("counts components with no body and leaves element functions open", async () => {
    const element = show(await fixture("supporting"), true);
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.querySelector(".fold-run")!.textContent!.trim(), "1 type · 2 components");
    const toggles = [...source.querySelectorAll<HTMLElement>(".fold")].map((toggle) => toggle.dataset["foldKey"]);
    assert.deepEqual(toggles, ["run:Task", "Badge", "Card", "tasks"], "an exported element function is the document's own");
  });

  it("folds groups and declarations but not if or for blocks, and draws a folded unit's chevron", async () => {
    const element = show(await fixture("nesting"));
    const toggles = (): HTMLElement[] => [...root(element).querySelectorAll<HTMLElement>(".fold")];
    assert.deepEqual(
      toggles().map((toggle) => toggle.dataset["foldKey"]),
      ["run:Item", "root", "root.body", "root.body.items[0]"]
    );
    element.folded = ["root.body.items[0]"];
    const folded = toggles().filter((toggle) => toggle.classList.contains("folded"));
    assert.deepEqual(
      folded.map((toggle) => toggle.dataset["foldKey"]),
      ["root.body.items[0]"],
      "what is folded shows its chevron"
    );
    element.folded = undefined;
    assert.ok(element.folded.includes("run:Item"), "undefined goes back to the defaults");
    assert.ok(!element.folded.includes("root.body"));
  });

  it("folds and opens everything inside a unit on an Alt+click", async () => {
    const element = show(await fixture("nesting"));
    const altClick = (key: string): void => {
      const toggle = [...root(element).querySelectorAll<HTMLElement>(".fold")].find(
        (candidate) => candidate.dataset["foldKey"] === key
      )!;
      toggle.dispatchEvent(new document.defaultView!.MouseEvent("click", { altKey: true, bubbles: true, composed: true }));
    };
    altClick("root.body");
    assert.deepEqual([...element.folded].sort(), ["root.body", "root.body.items[0]"]);
    altClick("root.body");
    assert.deepEqual(element.folded, [], "everything inside opens too");
  });

  it("keeps a fold that hides the selection, and selects what holds it", async () => {
    const element = show(await fixture("nesting"));
    const item = "root.body.items[0].items[0]";
    const groupToggle = (): HTMLElement =>
      [...root(element).querySelectorAll<HTMLElement>(".fold")].find(
        (candidate) => candidate.dataset["foldKey"] === "root.body"
      )!;
    element.selection = item;
    groupToggle().click();
    assert.deepEqual(element.folded, ["root.body"], "a click folds the group over its selected row");
    assert.equal(element.selection, "root.body");
    element.expandAll();
    element.selection = item;
    element.collapseAll();
    assert.ok(element.folded.includes("root.body.items[0]"), "collapsing everything keeps it folded");
    assert.equal(element.selection, "root", "the outermost fold left drawn holds the selection");
    element.expandAll();
    element.selection = item;
    element.folded = ["root.body.items[0]"];
    assert.deepEqual(element.folded, ["root.body.items[0]"], "a host's folds stay as set");
    assert.equal(element.selection, "root.body.items[0]");
  });

  it("clears a selection a folded run hides, and tells the host", async () => {
    const element = show(await fixture("nesting"));
    const folds: NxFoldDetail[] = [];
    element.addEventListener(NX_FOLD_EVENT, (event) => folds.push((event as CustomEvent<NxFoldDetail>).detail));
    element.selection = "Panel";
    root(element).querySelector<HTMLElement>(".fold-run-head")!.click();
    assert.ok(element.folded.includes("run:Item"), "the run stays folded");
    assert.equal(element.selection, undefined);
    assert.equal(folds.at(-1)!.selection, undefined, "the host hears the selection is gone");
    element.expandAll();
    element.selection = "root.body.items[0].items[0]";
    element.collapseAll();
    assert.equal(folds.at(-1)!.selection, "root", "a fold that moves the selection says where");
  });
});
