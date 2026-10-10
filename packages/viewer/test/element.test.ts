import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it } from "node:test";

import { fixture, nxHost, type Fixture } from "./fixtures.js";
import type { NxSelectDetail, NxViewerElement } from "../src/index.js";

const { NX_SELECT_EVENT, inWords, eventWords, nodeAtSpan } = await import("../src/index.js");

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
    assert.equal(element.selection, "User.id");
  });

  it("takes properties set before the element was defined", async () => {
    const { tree, text } = await fixture("user");
    const element = document.createElement("div") as unknown as NxViewerElement;
    Object.assign(element, { tree, text });
    assert.equal(element.tree, tree);
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

  it("offers the source of whatever is selected", async () => {
    const element = show(await fixture("operators"));
    element.selection = "sum.body.left";
    const button = root(element).querySelector<HTMLButtonElement>(".selection-source")!;
    assert.equal(button.hidden, false);
    button.click();
    assert.equal(root(element).querySelector(".source-panel pre")!.textContent, "(a + b) * 2");
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
    assert.equal(read(card.querySelector(".kind")!), "Single choice");
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

  it("renders an agent's prompt as prose", async () => {
    const element = show(await fixture("agent"));
    const bodies = Array.from(root(element).querySelectorAll(".card .text-body")).map(read);
    assert.ok(bodies.includes("You are the support assistant. Answer from the documents."), bodies.join(" | "));
  });
});

describe("ghost rows", () => {
  it("shows an omitted default as a ghost row", async () => {
    const element = show(await fixture("questionFlow"));
    const card = rendered(element, "roleQuestion.value");
    const ghost = Array.from(card.querySelectorAll(":scope > .rows > .row.ghost")).find(
      (row) => read(row.querySelector(".label")!) === "required"
    );
    assert.ok(ghost, "required shows as a ghost");
    assert.equal(read(ghost.querySelector(".ghost-value")!), "true");
    assert.equal(read(ghost.querySelector(".tag")!), "default");
    assert.equal(ghost.closest("[data-key]"), card, "a ghost carries no key of its own");
  });

  it("hides ghosts when asked", async () => {
    const element = show(await fixture("questionFlow"));
    element.ghosts = false;
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
    assert.ok(rendered(element, key).classList.contains("selected"));
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

  it("finds the node a value origin names", async () => {
    const flow = await fixture("questionFlow");
    const at = flow.tree.nodes.findIndex((node) => node.key === "roleQuestion.value");
    const { startByte, endByte } = flow.tree.nodes[at]!.range;
    assert.equal(nodeAtSpan(flow.tree, startByte, endByte), at);
    assert.equal(nodeAtSpan(flow.tree, startByte, endByte + 1), undefined);
  });
});
