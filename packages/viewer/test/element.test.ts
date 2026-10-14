import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it } from "node:test";

import { fixture, nxHost, type Fixture } from "./fixtures.js";
import type { NxSelectDetail, NxViewerElement } from "../src/index.js";

const { NX_SELECT_EVENT, NxViewerElement: ViewerClass, inWords, eventWords, nodeAtSpan } = await import("../src/index.js");

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

describe("words", () => {
  it("splits kinds at their capitals into sentence case", () => {
    assert.equal(inWords("SingleChoice"), "Single choice");
    assert.equal(inWords("URLField"), "URL field");
    assert.equal(inWords("User.Update"), "User update");
    assert.equal(inWords("Step"), "Step");
  });

  it("reads handler names as events", () => {
    assert.equal(eventWords("onIntegerAnswered"), "integer answered");
    assert.equal(eventWords("onTapped"), "tapped");
  });
});

describe("<nx-viewer>", () => {
  it("shows a document's reading view on a plain page", async () => {
    const element = show(await fixture("questionFlow"));
    assert.ok(root(element).querySelector(".document"), "the document is rendered");
    assert.ok(root(element).querySelectorAll(".card").length > 10, "elements read as cards");
  });

  it("keeps its markup and styles in a shadow root", async () => {
    const style = document.createElement("style");
    style.textContent = "div, span, button { display: none !important; color: red !important; }";
    document.head.append(style);
    try {
      const element = show(await fixture("user"));
      assert.equal(document.querySelectorAll(".card, .declaration").length, 0, "nothing leaks into the page");
      assert.ok(root(element).querySelector("style"), "the element's styles are in its shadow root");
      const declaration = rendered(element, "User");
      assert.equal(declaration.getRootNode(), root(element), "the rendering lives in the shadow root");
      assert.notEqual(getComputedStyle(declaration).display, "none", "the page's styles do not reach it");
    } finally {
      style.remove();
    }
  });

  it("mutes and badges a stale tree, and still selects", async () => {
    const element = show(await fixture("user"));
    element.stale = true;
    assert.equal(element.getAttribute("stale"), "");
    assert.equal(root(element).querySelector<HTMLElement>(".badge")!.hidden, false);
    rendered(element, "User.id").click();
    assert.equal(element.selection, "User");
  });

  it("still hovers a stale tree", async () => {
    const element = show(await fixture("questionFlow"));
    element.stale = true;
    const label = rendered(element, "roleQuestion.value.id").querySelector<HTMLElement>(".label")!;
    assert.match(read(element.hoverFor(label)!), /^id string/);
  });

  it("takes properties set before the element was defined", async () => {
    const { tree, text } = await fixture("questionFlow");
    const tag = "nx-viewer-defined-late";
    const element = document.createElement(tag) as NxViewerElement;
    document.body.append(element);
    Object.assign(element, { text, tree, ghosts: true, stale: true });
    customElements.define(tag, class extends ViewerClass {});
    created.push(element);
    assert.ok(element instanceof ViewerClass, "the element upgraded");
    assert.equal(element.tree, tree);
    assert.ok(root(element).querySelectorAll(".card").length > 0, "the tree set early is rendered");
    assert.ok(root(element).querySelectorAll(".ghost").length > 0, "ghosts set early are shown");
    assert.equal(element.hasAttribute("stale"), true);
    assert.equal(root(element).querySelector<HTMLElement>(".badge")!.hidden, false, "the stale badge shows");
  });
});

describe("the reading is lossless", () => {
  it("shows the exact source of a card", async () => {
    const flow = await fixture("questionFlow");
    const element = show(flow);
    const key = keyWhere(flow, (node) => node.role === "element" && node.name === "Choice" && node.key.endsWith("choices[3]"));
    rendered(element, key).querySelector<HTMLButtonElement>(".source-toggle")!.click();
    assert.equal(element.shownSource, key);
    assert.equal(root(element).querySelector(".source-panel pre")!.textContent, '<Choice value="other" label="Something else" />');
  });

  it("slices a literal after an emoji by bytes", async () => {
    const emoji = await fixture("emoji");
    const element = show(emoji);
    const key = keyWhere(emoji, (node) => node.role === "literal" && node.key.endsWith("title.value"));
    element.showSource(key);
    assert.equal(root(element).querySelector(".source-panel pre")!.textContent, '"after"');
  });

  it("shows an unparsed region as its text, marked", async () => {
    const element = show(await fixture("unparsed"));
    const region = root(element).querySelector(".unparsed")!;
    assert.match(read(region), /Could not be read/);
    assert.match(region.querySelector("pre")!.textContent ?? "", /let b = = =/);
  });

  it("folds imports into a details strip", async () => {
    const element = show(await fixture("questionFlow"));
    const strip = root(element).querySelector("details.details")!;
    assert.equal(strip.querySelectorAll("[data-role='import']").length, 3);
    assert.equal(read(strip.querySelector("summary")!), "Imports · 3");
  });
});

describe("elements read as cards", () => {
  it("heads a survey question by its kind and lists its properties in declaration order", async () => {
    const flow = await fixture("questionFlow");
    const element = show(flow);
    const card = rendered(element, "roleQuestion.value");
    assert.equal(read(card.querySelector(".kind")!), "SingleChoice");
    assert.equal(read(card.querySelector(".card-name")!), "roleQuestion");
    const labels = Array.from(card.querySelectorAll<HTMLElement>(":scope > .rows > .row:not(.ghost) > .label")).map(read);
    assert.deepEqual(labels, ["id", "label", "allowsOther", "layout", "choices"]);
    const choices = card.querySelectorAll(":scope > .rows > .row.content-row .item-list > .card");
    assert.equal(choices.length, 4);
    assert.equal(read(choices[0]!.querySelector(".kind")!), "Choice");
  });

  it("marks a kind no declaration describes", async () => {
    const element = show(await fixture("unknownKind"));
    const card = rendered(element, "mystery.value");
    assert.equal(read(card.querySelector(".kind")!), "Mystery");
    assert.ok(card.querySelector(".kind.unresolved"));
    assert.equal(read(card.querySelector(".unresolved-marker")!), "unresolved");
  });
});

describe("the source view", () => {
  it("shows the document's own text, with every node marked and none written twice", async () => {
    const tasks = await fixture("tasks");
    const element = show(tasks);
    element.view = "source";
    const source = root(element).querySelector<HTMLElement>("pre.source")!;
    assert.equal(source.textContent, tasks.text, "the text is the source, character for character");
    assert.equal(root(element).querySelector(".declaration, .card"), null, "there are no cards");
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
      view: "source",
      colors: [{ start, end: start + 4, light: "#111111", dark: "#eeeeee", fontStyle: 2 }],
    });
    const colored = root(element).querySelector<HTMLElement>(".source .t")!;
    assert.equal(colored.textContent, "Task");
    assert.equal(colored.style.getPropertyValue("--l"), "#111111");
    assert.equal(colored.style.getPropertyValue("--d"), "#eeeeee");
    assert.equal(colored.style.fontWeight, "bold");
    assert.equal(root(element).querySelector(".source")!.textContent, tasks.text, "coloring changes no text");

    const events: string[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail.key));
    rendered(element, "Task.title").click();
    assert.deepEqual(events, ["Task.title"], "a field is selected on its own in the source view");
    assert.ok(rendered(element, "Task.title").classList.contains("selected"));
    element.view = "cards";
    assert.ok(root(element).querySelector(".declaration"), "the cards come back");
  });

  it("marks prose and indentation, so a host can set each in a font of its own", async () => {
    const tasks = await fixture("tasks");
    const element = show(tasks);
    element.view = "source";
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.querySelector(".prose"), null, "a string is code unless its type says a person reads it");
    const indents = Array.from(source.querySelectorAll(".indent"), (indent) => indent.textContent!);
    assert.ok(indents.length > 0 && indents.every((indent) => /^[ \t]+$/.test(indent)), "indentation is whitespace only");
    assert.equal(indents.join("").length, (tasks.text.match(/^[ \t]+/gm) ?? []).join("").length, "every line's is marked");
    assert.equal(source.textContent, tasks.text, "marking adds and removes nothing");
  });

  it("gives an attribute's equals sign room, whether or not the source puts a space beside it", async () => {
    const spacing = await fixture("spacing");
    const element = show(spacing);
    element.view = "source";
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.textContent, spacing.text, "the text is unchanged");
    const attribute = (name: string): HTMLElement =>
      Array.from(source.querySelectorAll<HTMLElement>('[data-role="attribute"]')).find((node) => node.textContent!.startsWith(name))!;
    const classes = (name: string, selector: string): number => attribute(name).querySelectorAll(selector).length;
    // `title="Ship"`: the sign takes the room, and there is no space to give any up.
    assert.equal(classes("title", ".attribute-equals"), 1);
    assert.equal(classes("title", ".attribute-space"), 0);
    // `priority = 1`: the sign takes the room, and the space on each side gives its own up.
    assert.equal(classes("priority", ".attribute-equals"), 1);
    assert.deepEqual(Array.from(attribute("priority").querySelectorAll(".attribute-space"), (space) => space.textContent), [" ", " "]);
    // `done  =  {true}`: set off by more than one space, it stays as its author spaced it.
    assert.equal(classes("done", ".attribute-equals"), 0);
    // A default's `=` in a declaration is not an attribute's.
    assert.equal(rendered(element, "Task").querySelector(".attribute-equals"), null);
  });

  it("sets text shown to people apart, by the type of what holds it and never by how it looks", async () => {
    const fixtureText = await fixture("uitext");
    const element = show(fixtureText);
    element.view = "source";
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.textContent, fixtureText.text, "the text is unchanged");
    assert.deepEqual(Array.from(source.querySelectorAll(".prose"), read), ["// A note for whoever reads the source."]);
    const shown = Array.from(source.querySelectorAll<HTMLElement>(".uitext"), (node) => node.textContent);
    assert.deepEqual(shown, [
      '"Fine"', // a field's default
      '"Hello, "', // in a function that returns uitext
      '"First choice"', // a uitext property
      '"Pick me"', // a property whose type is an alias of uitext
      "Shown to people", // an element's text, bound to a uitext content property
      '"Second "', // inside an expression a uitext property holds
      '"choice"',
    ]);
    assert.ok(!shown.includes('"a"') && !shown.includes('"plain"'), "a string property's value stays code");
    const first = Array.from(source.querySelectorAll<HTMLElement>(".uitext")).find((node) => node.textContent === '"First choice"')!;
    assert.equal(first.querySelectorAll(".uitext-quote").length, 2, "its quotes are set apart from its words");
    const block = first.querySelector<HTMLElement>(".uitext-block")!;
    assert.equal(block.style.getPropertyValue("--column"), String('  <Choice value="a" label="'.length), "the words' column is known");
  });

  it("sets a string its viewer formats as an identifier apart as one quiet token", async () => {
    const identifiers = await fixture("identifiers");
    const element = show(identifiers);
    element.view = "source";
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.textContent, identifiers.text, "the text is unchanged");
    const tokens = Array.from(source.querySelectorAll<HTMLElement>(".id-value"), (node) => node.textContent);
    assert.deepEqual(tokens, ['"welcome"'], "only the property the viewer names; next is a plain string");
    assert.equal(source.querySelector(".id-value")!.querySelectorAll(".id-quote").length, 2);
    element.view = "cards";
    assert.ok(root(element).querySelector(".value.formatted.id"), "the card reading formats it too");
  });

  it("shows the color a string its viewer formats as a color names, as a square before it", async () => {
    const colors = await fixture("colors");
    const element = show(colors);
    element.view = "source";
    const source = root(element).querySelector<HTMLElement>(".source")!;
    assert.equal(source.textContent, colors.text, "the text is unchanged");
    const squares = Array.from(source.querySelectorAll<HTMLElement>(".swatch"));
    assert.equal(squares.length, 1, "only a color the type's viewer formats, and only one that names a color");
    assert.equal(squares[0]!.parentElement!.textContent, '"#b6695d"');
    assert.equal(squares[0]!.getAttribute("aria-hidden"), "true");
    element.sheets = true;
    const sheet = root(element).querySelector<HTMLElement>(".source .swatch");
    assert.ok(sheet, "a one-line element shows it too");
    assert.equal(sheet!.parentElement!.textContent, "#b6695d");
  });

  it("starts an element on a line of its own with the icon its viewer names, or its kind's", async () => {
    const icons = await fixture("icons");
    const element = show(icons);
    element.view = "source";
    element.sheets = true;
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
    element.view = "source";
    element.sheets = true;
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

  it("moves an element's id to where its viewer puts it, as its value alone", async () => {
    const ids = await fixture("elementIds");
    const element = show(ids);
    element.view = "source";
    const source = root(element).querySelector<HTMLElement>(".source")!;
    const reading = source.textContent!;
    // At the beginning, after the name; the line it stood on goes with it.
    assert.ok(reading.includes("<Step welcome\n    title="), reading);
    assert.ok(!reading.includes('id="welcome"'));
    // At the end, after the attributes that stay, and closing the tag with no space between.
    assert.ok(reading.includes('<Choice label="Yes" yes/>'), reading);
    const chips = Array.from(source.querySelectorAll<HTMLElement>(".id-part"));
    assert.deepEqual(chips.map((chip) => chip.textContent), ["welcome", "yes"]);
    // Each chip keeps the keys of the attribute and its value, so selection still finds them.
    const keys = Array.from(source.querySelectorAll<HTMLElement>("[data-key]"), (node) => node.dataset["key"]!);
    assert.deepEqual([...keys].sort(), ids.tree.nodes.map((node) => node.key).sort(), "every node is still there once");
  });

  it("reads an element written an attribute to a line as a property sheet, when asked", async () => {
    const ids = await fixture("elementIds");
    const element = show(ids);
    Object.assign(element, { view: "source", sheets: true });
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
    element.sheets = false;
    assert.equal(root(element).querySelector(".sheet"), null, "and not when not asked");
  });

  it("keeps offsets right after a character outside the basic plane", async () => {
    const emoji = await fixture("emoji");
    const element = show(emoji);
    element.view = "source";
    assert.equal(root(element).querySelector(".source")!.textContent, emoji.text);
  });
});

describe("an element's properties", () => {
  it("read beside its heading when they are few and short, and as rows when they are not", async () => {
    const tasks = await fixture("tasks");
    const element = show(tasks);
    const [short, long] = Array.from(root(element).querySelectorAll<HTMLElement>(".card"));
    const inline = short!.querySelector<HTMLElement>(":scope > .card-header > .rows.inline-rows");
    assert.ok(inline, "two short properties read on the heading's line");
    assert.equal(read(inline), "title=Ship the site priority=1");
    assert.equal(short!.querySelector(":scope > .rows"), null);
    assert.ok(inline.querySelector(".row[data-key]"), "each still carries its key");
    assert.ok(long!.querySelector(":scope > .rows > .row"), "a long value keeps a row to each property");
    assert.equal(long!.querySelector(".inline-rows"), null);
  });
});

describe("values", () => {
  it("drops a string's quotes and shows a case as a pill", async () => {
    const element = show(await fixture("questionFlow"));
    const label = rendered(element, "roleQuestion.value.choices[1].label.value");
    assert.equal(label.textContent, "Design");
    assert.ok(label.classList.contains("string"));
    const layout = rendered(element, "roleQuestion.value.layout.value");
    assert.ok(layout.classList.contains("pill"));
    assert.equal(read(layout), "chips");
  });

  it("shows a boolean as a check with its value as its name", async () => {
    const element = show(await fixture("questionFlow"));
    const allowsOther = rendered(element, "roleQuestion.value.allowsOther.value");
    assert.equal(allowsOther.textContent, "✓");
    assert.equal(allowsOther.getAttribute("aria-label"), "true");
  });

  it("renders a markdown body with its embeds as slots", async () => {
    const element = show(await fixture("typedText"));
    const body = root(element).querySelector(".text-body.markdown")!;
    assert.ok(body.querySelector("h3"), "the heading is a heading");
    assert.equal(body.querySelectorAll("li").length, 2);
    assert.equal(body.querySelectorAll(".embed").length, 2);
    assert.match(read(body), /Tasks for owner/);
  });

  it("keeps underscores inside words and spaced asterisks as written", async () => {
    const element = show(await fixture("markdownText"));
    const body = root(element).querySelector(".card .text-body")!;
    assert.match(read(body), /Call find_plans_for_team or snake__case__x when asked\. Price is 2 \* 3 \* 4 dollars, or x \*\* y \*\* z and x __ y __ z\./);
    assert.deepEqual(Array.from(body.querySelectorAll("em")).map(read), ["really", "important"]);
    assert.deepEqual(Array.from(body.querySelectorAll("strong")).map(read), ["bold", "strong"]);
  });

  it("renders an agent's prompt as prose", async () => {
    const element = show(await fixture("agent"));
    const bodies = Array.from(root(element).querySelectorAll(".card .text-body")).map(read);
    assert.ok(bodies.includes("You are the support assistant. Answer from the documents."), bodies.join(" | "));
  });
});

describe("ghost rows", () => {
  it("shows every omitted default as a ghost row when asked", async () => {
    const element = show(await fixture("questionFlow"));
    element.ghosts = true;
    const card = rendered(element, "roleQuestion.value");
    const ghost = Array.from(card.querySelectorAll(":scope > .rows > .row.ghost")).find(
      (row) => read(row.querySelector(".label")!) === "required"
    );
    assert.ok(ghost, "required shows as a ghost");
    assert.equal(read(ghost.querySelector(".ghost-value")!), "true");
    assert.equal(read(ghost.querySelector(".tag")!), "default");
    assert.equal(ghost.closest("[data-key]"), card, "a ghost carries no key of its own");
  });

  it("hides the defaults no viewer names unless asked", async () => {
    const element = show(await fixture("questionFlow"));
    assert.equal(root(element).querySelectorAll(".row.ghost").length, 0);
  });
});

describe("logic reads as sentences", () => {
  it("labels a guarded step", async () => {
    const element = show(await fixture("guardedStep"));
    const condition = rendered(element, "steps.body");
    assert.equal(read(condition.querySelector(".when-label")!), "Only when step = 3 and role ≠ engineer");
    assert.ok(condition.querySelector(".when-label .value.string"), "engineer is in the value style");
    assert.ok(condition.querySelector(".branch .card"), "the step is the guarded content");
  });

  it("reads a string sum as a sentence with a slot", async () => {
    const element = show(await fixture("sentence"));
    const sentence = rendered(element, "thanks.body.text.value");
    assert.equal(read(sentence), "Thanks, name. Where can we reach you?");
    assert.equal(read(sentence.querySelector(".slot")!), "name");
  });

  it("reads a fallback", async () => {
    const element = show(await fixture("fallback"));
    assert.equal(read(rendered(element, "byline.body.text.value")), "nickname, otherwise Anonymous");
  });

  it("reads every operator by the list and keeps the source's parentheses", async () => {
    const element = show(await fixture("operators"));
    assert.equal(read(rendered(element, "sum.body")), "(a + b) × 2 − a ÷ b");
    assert.equal(read(rendered(element, "rest.body")), "remainder of a ÷ b");
    assert.equal(read(rendered(element, "compare.body")), "a ≤ b or a ≥ b and not (a < b)");
    assert.equal(read(rendered(element, "given.body")), "s is given");
    assert.equal(read(rendered(element, "negative.body")), "−a");
    assert.equal(read(rendered(element, "upTo.body")), "1 up to a");
    assert.equal(read(rendered(element, "through.body")), "1 through a");
  });

  it("reads a match as a table and a loop as for each", async () => {
    const element = show(await fixture("logic"));
    const match = rendered(element, "describe.body");
    assert.equal(read(match.querySelector(".when-label")!), "shape is");
    assert.equal(match.querySelectorAll(".arm").length, 2);
    assert.equal(read(match.querySelector(".else-arm .arm-when")!), "anything else");
    assert.equal(read(match.querySelector(".else-arm .arm-then")!), "angular");
    assert.equal(read(rendered(element, "doubled.body").querySelector(".when-label")!), "For each item in items");
  });
});

describe("handlers read as events", () => {
  it("reads a state update as set lines", async () => {
    const flow = await fixture("questionFlow");
    const element = show(flow);
    const key = keyWhere(flow, (node) => node.role === "attribute" && node.name === "onIntegerAnswered");
    const handler = rendered(element, key);
    assert.equal(read(handler.querySelector(".when")!), "When integer answered");
    const lines = Array.from(handler.querySelectorAll(".set-line")).map(read);
    assert.ok(lines.includes("set answered to answered + 1"), lines.join(" | "));
    assert.equal(handler.querySelector(".arrow"), null, "an update needs no arrow");
  });

  it("reads any other handler value after an arrow", async () => {
    const component = await fixture("component");
    const element = show(component);
    const key = keyWhere(component, (node) => node.role === "attribute" && node.flags?.includes("handler") === true && component.tree.nodes[component.tree.nodes.indexOf(node) + 1]?.role === "sequence");
    assert.ok(rendered(element, key).querySelector(".arrow"));
  });
});

describe("declarations, references and comments", () => {
  it("reads a union on one line when no case carries fields, and a default as it is written", async () => {
    const element = show(await fixture("members"));
    const mode = rendered(element, "ThemeMode");
    assert.equal(read(mode.querySelector(".declaration-kind")!), "Type");
    assert.equal(read(mode.querySelector(".declaration-header > .case-line")!), "light | dark");
    assert.equal(mode.querySelector(".group-label"), null);
    const state = rendered(element, "LoadState");
    assert.equal(state.querySelector(".case-line"), null, "a case with fields keeps a row to each case");
    const cases = Array.from(state.querySelectorAll(".union-case"), read);
    assert.deepEqual(cases, ["| idle", "| failed { message string }"], "a case reads after a bar, its field or two beside it");
    assert.equal(read(rendered(element, "Toggle.on")), "on boolean = true");
    assert.equal(read(rendered(element, "Toggle.note")), "note string optional", "optional reads after the type");
    assert.equal(read(rendered(element, "Toggle").querySelector(".declaration-header > .declaration-sign")!), "=");
    assert.equal(rendered(element, "Toggle").querySelector(".group-label"), null, "a record's fields need no heading");
  });

  it("reads a record type as its fields", async () => {
    const element = show(await fixture("user"));
    const declaration = rendered(element, "User");
    assert.equal(read(declaration.querySelector(".declaration-kind")!), "Type");
    const id = rendered(element, "User.id");
    assert.equal(read(id), "id string");
    const name = rendered(element, "User.name");
    assert.equal(read(name), "name string?");
    const doc = Array.from(declaration.querySelectorAll(".doc")).map(read);
    assert.ok(doc.includes("What to call them."), "the field's doc comment is prose");
  });

  it("shows a comment as a note where it stands", async () => {
    const element = show(await fixture("logic"));
    const note = rendered(element, "answer:comment[0]");
    assert.equal(read(note), "A note about the next declaration.");
    assert.ok(note.classList.contains("comment"));
  });

  it("reads a declaration's type in its header, and heads its groups only when it has several", async () => {
    const element = show(await fixture("headings"));
    const headings = (key: string): string[] => Array.from(rendered(element, key).querySelectorAll(".group-label"), read);
    const header = (key: string): string => {
      const copy = rendered(element, key).querySelector(".declaration-header")!.cloneNode(true) as HTMLElement;
      copy.querySelector(".source-toggle")?.remove();
      return read(copy);
    };
    assert.equal(header("total"), "Function total → int =");
    assert.deepEqual(headings("total"), ["Takes", "Returns"]);
    assert.equal(header("answer"), "Value answer : int =");
    assert.deepEqual(headings("answer"), [], "a lone group needs no heading");
    assert.equal(header("Count"), "Type Count = int");
    assert.equal(rendered(element, "Count").querySelector(".declaration-body"), null, "an alias is its header");
  });

  it("reads what a declaration extends in its header, with what it inherits folded behind a toggle", async () => {
    const element = show(await fixture("bases"));
    const header = (key: string): string => {
      const copy = rendered(element, key).querySelector(".declaration-header")!.cloneNode(true) as HTMLElement;
      copy.querySelectorAll("button").forEach((button) => button.remove());
      return read(copy);
    };
    assert.equal(header("Button"), "external Component Button extends Control =");
    const button = rendered(element, "Button");
    assert.equal(button.querySelector(".group-label:not(.inherited *)"), null, "the base needs no heading of its own");
    const inherited = button.querySelector<HTMLElement>(".inherited")!;
    assert.equal(inherited.hidden, true, "what is inherited stays folded");
    const toggle = button.querySelector<HTMLButtonElement>(".inherited-toggle")!;
    toggle.click();
    assert.equal(inherited.hidden, false);
    assert.equal(toggle.getAttribute("aria-expanded"), "true");
    assert.deepEqual(Array.from(inherited.children, read), ["From Control", "disabled boolean = false", "help string optional"]);
    // A base that declares nothing leaves nothing to unfold, and a declaration with nothing of its own has no body.
    const divider = rendered(element, "Divider");
    assert.equal(header("Divider"), "external Component Divider extends View");
    assert.equal(divider.querySelector(".inherited-toggle"), null);
    assert.equal(divider.querySelector(".declaration-body"), null);
  });

  it("reads a declaration's doc comment inside its card", async () => {
    const element = show(await fixture("user"));
    const doc = rendered(element, "User").querySelector(":scope > .doc");
    assert.equal(read(doc!), "A person the survey is about.");
    assert.ok(doc!.nextElementSibling?.classList.contains("declaration-header"), "the comment reads over the header");
    assert.equal(root(element).querySelector(".document > .doc"), null);
  });

  it("reads a comment written over several lines as one note", async () => {
    const element = show(await fixture("notesAndLists"));
    const notes = Array.from(root(element).querySelectorAll(".document > .notes, .document > .comment"));
    assert.deepEqual(
      notes.map((note) => Array.from(note.matches(".notes") ? note.children : [note], read)),
      [["The first line of a note", "and its second line."], ["A note of its own."], ["About a."], ["About b."]],
      "consecutive line comments join; a blank line, or a comment trailing code, keeps notes apart"
    );
    assert.ok(Array.from(notes[0]!.children).every((line) => line.matches(".comment[data-key]")));
  });

  it("separates the items of a list", async () => {
    const element = show(await fixture("notesAndLists"));
    const list = root(element).querySelector<HTMLElement>(".sequence");
    assert.ok(list);
    assert.equal(read(list), "red, green, blue");
  });

  it("shows a reference as a link naming its target", async () => {
    const element = show(await fixture("questionFlow"));
    const reference = root(element).querySelector<HTMLElement>(".reference .ref")!;
    assert.ok(reference.textContent);
  });
});

describe("hover and peek", () => {
  it("lists a union's cases with the current one marked", async () => {
    const element = show(await fixture("questionFlow"));
    const label = rendered(element, "roleQuestion.value.layout").querySelector<HTMLElement>(".label")!;
    const hover = element.hoverFor(label)!;
    const cases = Array.from(hover.querySelectorAll(".hover-cases .pill")).map(read);
    assert.deepEqual(cases, ["list", "grid", "chips"]);
    assert.equal(read(hover.querySelector(".pill.current")!), "chips");
    assert.match(read(hover), /Default: ChoiceLayout\.list/);
  });

  it("explains a property from the declaration table", async () => {
    const element = show(await fixture("questionFlow"));
    const label = rendered(element, "roleQuestion.value.id").querySelector<HTMLElement>(".label")!;
    assert.match(read(element.hoverFor(label)!), /^id string/);
  });

  it("names a reference's target", async () => {
    const flow = await fixture("questionFlow");
    const element = show(flow);
    const key = keyWhere(flow, (node) => node.role === "reference" && node.name === "teamSizeQuestion");
    const hover = element.hoverFor(rendered(element, key).querySelector<HTMLElement>(".ref")!)!;
    assert.match(read(hover), /^teamSizeQuestion · value/);
  });

  it("expands a question in place", async () => {
    const flow = await fixture("questionFlow");
    const element = show(flow);
    const key = keyWhere(flow, (node) => node.role === "reference" && node.name === "teamSizeQuestion" && node.key.includes("question"));
    const reference = rendered(element, key);
    reference.querySelector<HTMLButtonElement>(".expand")!.click();
    const expansion = reference.querySelector(".expansion")!;
    assert.equal(read(expansion.querySelector(".kind")!), "Integer");
    assert.equal(expansion.querySelectorAll("[data-key]").length, 0, "the copy carries no original keys");
    assert.ok(expansion.querySelectorAll("[data-ref-key]").length > 0);
    reference.querySelector<HTMLButtonElement>(".expand")!.click();
    assert.equal((expansion as HTMLElement).hidden, true, "expanding again folds it");
  });

  it("keeps every key once when references are expanded", async () => {
    for (const name of ["agent", "questionFlow"]) {
      const element = show(await fixture(name));
      const buttons = root(element).querySelectorAll<HTMLButtonElement>(".expand");
      assert.ok(buttons.length > 0, `${name} has a reference to expand`);
      for (const button of buttons) {
        button.click();
      }
      const keys = Array.from(root(element).querySelectorAll<HTMLElement>("[data-key]")).map((run) => run.dataset["key"]!);
      const repeated = keys.filter((key, position) => keys.indexOf(key) !== position);
      assert.deepEqual(repeated, [], `${name}: keys rendered twice after expanding`);
    }
  });

  it("explains a handler by the action it answers", async () => {
    const component = await fixture("component");
    const element = show(component);
    const key = keyWhere(component, (node) => node.role === "attribute" && node.name === "onValueChanged");
    const when = rendered(element, key).querySelector<HTMLElement>(".when")!;
    assert.equal(when.tabIndex, 0, "a handler's name takes focus");
    const hover = element.hoverFor(when)!;
    assert.equal(read(hover.querySelector(".hover-head")!), "onValueChanged · when SearchBox emits SearchBox.ValueChanged");
    assert.equal(read(hover.querySelector(".hover-note")!), "Carries value:string");
  });
});

describe("selection", () => {
  it("selects the smallest node clicked and reports it", async () => {
    const element = show(await fixture("questionFlow"));
    const events: NxSelectDetail[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail));
    const key = "roleQuestion.value.choices[2].label.value";
    rendered(element, key).click();
    assert.equal(events.length, 1);
    assert.equal(events[0]!.key, key);
    assert.equal(events[0]!.role, "literal");
    assert.match(root(element).querySelector('[role="status"]')!.textContent ?? "", /^Selected literal: /);
    assert.ok(rendered(element, key).classList.contains("selected"));
  });

  it("selects the declaration when a member or the header is clicked", async () => {
    const element = show(await fixture("user"));
    const events: string[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail.key));
    rendered(element, "User.id").click();
    rendered(element, "User.name").querySelector<HTMLElement>(".type")!.click();
    rendered(element, "User").querySelector<HTMLElement>(".declaration-name")!.click();
    assert.deepEqual(events, ["User", "User", "User"]);
    assert.ok(rendered(element, "User").classList.contains("selected"));
    // The host can still select a member, as it does for the editor's cursor.
    element.selection = "User.id";
    assert.ok(rendered(element, "User.id").classList.contains("selected"));
  });

  it("selects from the host without an event, opening what is folded", async () => {
    const flow = await fixture("questionFlow");
    const element = show(flow);
    let fired = 0;
    element.addEventListener(NX_SELECT_EVENT, () => (fired += 1));
    const key = keyWhere(flow, (node) => node.role === "import");
    element.selection = key;
    assert.equal(fired, 0);
    assert.equal(element.selection, key);
    assert.equal(root(element).querySelector<HTMLDetailsElement>("details.details")!.open, true);
    assert.ok(rendered(element, key).classList.contains("selected"));
  });

  it("clears the selection for a key the tree does not hold", async () => {
    const element = show(await fixture("user"));
    element.selection = "User.id";
    element.selection = "Nothing.here";
    assert.equal(element.selection, undefined);
    assert.equal(root(element).querySelectorAll(".selected").length, 0);
  });

  it("announces a selected card by its kind and a row as name and value", async () => {
    const flow = await fixture("questionFlow");
    const element = show(flow);
    const status = root(element).querySelector('[role="status"]')!;
    rendered(element, "roleQuestion.value").click();
    assert.equal(status.textContent, "Selected SingleChoice card, roleQuestion");
    rendered(element, "roleQuestion.value.layout").click();
    assert.equal(status.textContent, "Selected layout: chips");
    const choice = keyWhere(flow, (node) => node.role === "element" && node.name === "Choice");
    rendered(element, choice).click();
    assert.equal(status.textContent, "Selected Choice card");
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

  it("walks the selection through every node with the arrow keys", async () => {
    const element = show(await fixture("user"));
    const events: string[] = [];
    element.addEventListener(NX_SELECT_EVENT, (event) => events.push((event as CustomEvent<NxSelectDetail>).detail.key));
    const body = root(element).querySelector<HTMLElement>(".body")!;
    assert.equal(body.tabIndex, 0, "the reading takes focus");
    key(body, "ArrowDown");
    key(body, "ArrowDown");
    key(body, "ArrowDown");
    key(body, "ArrowUp");
    const order = Array.from(root(element).querySelectorAll<HTMLElement>("[data-key]")).map((run) => run.dataset["key"]!);
    // The type's card comes first, and its doc comment is the first thing in it.
    assert.deepEqual(events, [order[0], order[1], order[2], order[1]]);
    assert.equal(element.selection, order[1]);
    assert.equal(body.getAttribute("role"), "region");
    const announcer = root(element).querySelector('[role="status"]')!;
    assert.equal(announcer.getAttribute("aria-live"), "polite");
    assert.equal(announcer.textContent, "Selected doc comment: A person the survey is about.", "the selection is announced");
  });

  it("moves focus into the source panel and back, and closes it with Escape", async () => {
    const element = show(await fixture("user"));
    const toggle = rendered(element, "User").querySelector<HTMLButtonElement>(".source-toggle")!;
    toggle.focus();
    toggle.click();
    const close = root(element).querySelector<HTMLButtonElement>(".source-close")!;
    assert.equal(root(element).activeElement, close, "focus moves to the panel");
    key(close, "Escape");
    assert.equal(element.shownSource, undefined, "Escape closes the panel");
    assert.equal(root(element).activeElement, toggle, "focus returns to the source button");
  });

  it("shows a hover on focus, describes the target with it, and hides it when focus moves on", async () => {
    const element = show(await fixture("questionFlow"));
    const label = rendered(element, "roleQuestion.value.id").querySelector<HTMLElement>(".label")!;
    const hover = root(element).querySelector<HTMLElement>(".hover")!;
    label.focus();
    assert.equal(hover.hidden, false, "focus shows the hover at once");
    assert.equal(label.getAttribute("aria-describedby"), hover.id);
    rendered(element, "roleQuestion.value").focus();
    assert.equal(hover.hidden, true, "moving focus on hides it");
    assert.equal(label.hasAttribute("aria-describedby"), false);
    assert.equal(label.classList.contains("hovered"), false);
  });
});
