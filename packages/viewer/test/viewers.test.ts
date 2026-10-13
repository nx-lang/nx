/**
 * Viewer declarations as the reading shows them: a kind's name, a card's parts, labels, phrases,
 * the details strip, layouts, case labels and value formats, and a viewer declaration itself.
 */
import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, describe, it } from "node:test";

import { fixture, nxHost, type Fixture } from "./fixtures.js";
import type { NxViewerElement } from "../src/index.js";

await import("../src/index.js");

const created: NxViewerElement[] = [];

after(async () => {
  for (const element of created) {
    element.remove();
  }
  (await nxHost()).dispose();
});

function show({ tree, text }: Fixture, images = true): NxViewerElement {
  const element = document.createElement("nx-viewer");
  document.body.append(element);
  element.images = images;
  element.text = text;
  element.tree = tree;
  created.push(element);
  return element;
}

function root(element: NxViewerElement): ShadowRoot {
  assert.ok(element.shadowRoot, "the element has an open shadow root");
  return element.shadowRoot;
}

/** Text with whitespace folded, as a reader sees it. */
function read(element: Element | null | undefined): string {
  return (element?.textContent ?? "").replace(/\s+/g, " ").trim();
}

/** The cards whose kind reads `kind`. */
function cards(element: NxViewerElement, kind: string): HTMLElement[] {
  return Array.from(root(element).querySelectorAll<HTMLElement>(".card")).filter(
    (card) => read(card.querySelector(":scope > .card-header > .kind")) === kind
  );
}

/** The key of the attribute `name` in the fixture's first element of kind `kind`. */
function attributeKey(fixture: Fixture, kind: string, name: string): string {
  const element = fixture.tree.nodes.findIndex((node) => node.role === "element" && node.name === kind);
  const attribute = fixture.tree.nodes.find(
    (node) => node.role === "attribute" && node.name === name && node.parent === element
  );
  assert.ok(attribute, `${kind} sets ${name}`);
  return attribute.key;
}

/** The labels of a card's own rows, outside its details strip. */
function rowLabels(card: HTMLElement): string[] {
  return Array.from(card.querySelectorAll(":scope > .rows > .row")).map((row) =>
    read(row.querySelector(":scope > .label") ?? row.querySelector(":scope > .row-value"))
  );
}

function detailsOf(card: HTMLElement): HTMLDetailsElement | null {
  return card.querySelector<HTMLDetailsElement>(":scope > details.details");
}

describe("viewers", () => {
  it("heads a card with the name its viewer gives the kind, and without one the kind's name as written", async () => {
    const plain = show(await fixture("survey"));
    assert.equal(cards(plain, "SingleChoice").length, 1);
    const renamed = show(await fixture("surveyRenamed"));
    assert.equal(cards(renamed, "Pick one").length, 1);
    assert.equal(cards(renamed, "SingleChoice").length, 0);
  });

  it("shows a question's title, its required phrase, and its key in details", async () => {
    const survey = await fixture("survey");
    const element = show(survey);
    const card = cards(element, "SingleChoice")[0]!;

    const title = card.querySelector<HTMLElement>(":scope > .card-header > .part.title")!;
    assert.equal(read(title), "Which describes your work best?");
    assert.equal(title.dataset["key"], attributeKey(survey, "SingleChoice", "label"));

    // `required` is left to its default, which reads as the phrase.
    const phrase = card.querySelector(":scope > .rows > .row.ghost.phrase-row");
    assert.equal(read(phrase), "Required");
    assert.equal(phrase?.querySelector(".tag"), null, "italic alone marks the default");
    assert.ok(!rowLabels(card).includes("required"), rowLabels(card).join(", "));
    assert.ok(!rowLabels(card).includes("label"), "the title is not shown again as a row");

    const details = detailsOf(card);
    assert.ok(details, "the card has a details strip");
    const idRow = details.querySelector<HTMLElement>(`[data-key="${attributeKey(survey, "SingleChoice", "id")}"]`);
    assert.ok(idRow, "the id row is in the details strip");
    assert.equal(read(idRow.querySelector(".label")), "Key");
  });

  it("reads a boolean set to a value with a phrase as the phrase, keyed by its literal", async () => {
    const optional = await fixture("surveyOptional");
    const element = show(optional);
    const card = cards(element, "SingleChoice")[0]!;
    const literal = optional.tree.nodes.find((node) => node.role === "literal" && node.value === "false")!;
    const phrase = card.querySelector<HTMLElement>(`.value.phrase[data-key="${literal.key}"]`);
    assert.ok(phrase, "the literal reads as the phrase");
    assert.equal(read(phrase), "Optional");
    assert.equal(phrase.dataset["property"], "required");
    const hover = element.hoverFor(phrase);
    assert.match(read(hover), /^required boolean/);
  });

  it("orders rows by the viewer, then the rest in declaration order", async () => {
    const element = show(await fixture("survey"));
    const card = cards(element, "SingleChoice")[0]!;
    // The properties the viewers name come first: `required`, which reads as its phrase, and `id`,
    // which is in details. `label` is the title. The rest follow in declaration order.
    assert.deepEqual(rowLabels(card), ["Required", "layout", "allowsOther", "choices"]);
  });

  it("reads choices as rows with their label, description and value as a badge", async () => {
    const element = show(await fixture("survey"));
    const choices = cards(element, "Choice");
    assert.equal(choices.length, 4);
    const first = choices[0]!;
    assert.ok(first.classList.contains("row-layout"));
    const header = first.querySelector(":scope > .card-header")!;
    assert.equal(read(header.querySelector(".part.title")), "Engineer");
    assert.equal(read(header.querySelector(".part.subtitle")), "Builds things");
    assert.equal(read(header.querySelector(".part.badge")), "eng");
    // Everything else is folded.
    assert.equal(first.querySelector(":scope > .rows"), null);
    const details = detailsOf(first)!;
    assert.deepEqual(
      Array.from(details.querySelectorAll(".row > .label")).map((label) => read(label)),
      ["color", "imageUrl"]
    );
  });

  it("labels a case in its pill, with its name and example in the hover", async () => {
    const element = show(await fixture("survey"));
    const card = cards(element, "SingleChoice")[0]!;
    const pill = card.querySelector<HTMLElement>(".pill.case.labelled")!;
    assert.equal(read(pill), "a list");
    const hover = element.hoverFor(pill);
    assert.ok(hover);
    assert.match(read(hover), /^list · a case of ChoiceLayout/);
  });

  it("shows an image URL as a picture with the URL beside it, and never loads another scheme", async () => {
    const element = show(await fixture("survey"));
    const [engineer, designer] = cards(element, "Choice");
    const picture = engineer!.querySelector<HTMLImageElement>("img.picture")!;
    assert.equal(picture.getAttribute("src"), "https://example.com/a.png");
    assert.equal(read(picture.parentElement!.querySelector(".picture-url")), "https://example.com/a.png");
    assert.equal(designer!.querySelector("img"), null);
    assert.equal(designer!.querySelector("a"), null);
    assert.ok(read(designer).includes("javascript:alert(1)"));
  });

  it("shows an image URL as a link and loads nothing when images are off", async () => {
    const element = show(await fixture("survey"), false);
    const engineer = cards(element, "Choice")[0]!;
    assert.equal(engineer.querySelector("img"), null);
    const link = engineer.querySelector<HTMLAnchorElement>(".value.formatted.image a")!;
    assert.equal(link.getAttribute("href"), "https://example.com/a.png");
  });

  it("shows a color with a swatch", async () => {
    const element = show(await fixture("survey"));
    const engineer = cards(element, "Choice")[0]!;
    const color = engineer.querySelector<HTMLElement>(".value.formatted.color")!;
    assert.equal(read(color), "#3366ff");
    assert.ok(color.querySelector(".swatch"));
  });

  it("formats links, prose and plain text, and leaves a reference alone", async () => {
    const element = show(await fixture("formats"));
    const card = cards(element, "Contact")[0]!;
    const href = (format: string) => card.querySelector(`.value.formatted.${format} a`)?.getAttribute("href");
    assert.equal(href("url"), "https://example.com");
    assert.equal(href("email"), "mailto:ada@example.com");
    assert.equal(href("phone"), "tel:+15550100199");
    assert.equal(read(card.querySelector(".value.formatted.markdown em")), "notes");
    assert.equal(read(card.querySelector(".value.formatted.text")), "plain");
    // `other={name}` is an embed of a reference, which no format changes.
    assert.equal(card.querySelectorAll(".value.formatted.url").length, 1);
  });

  it("shows an inline element in its line, with the rest folded", async () => {
    const element = show(await fixture("inlineTag"));
    // The color's viewer gives it only a format, so its default shows when every default does.
    element.ghosts = true;
    const tag = cards(element, "Tag")[0]!;
    assert.equal(tag.tagName, "SPAN");
    assert.ok(tag.classList.contains("inline-layout"));
    assert.equal(read(tag.querySelector(".part.title")), "news");
    // The line holds only the title; the subtitle and badge read as rows in details.
    assert.equal(tag.querySelector(".part.subtitle, .part.badge"), null);
    const details = detailsOf(tag);
    assert.deepEqual(
      Array.from(details?.querySelectorAll(".row:not(.ghost) > .label") ?? []).map((label) => read(label)),
      ["note", "code"]
    );
    assert.ok(details?.querySelector(".row.ghost"), "the color default is in details");
  });

  it("shows a value whose format or layout it does not know as plain text in a card", async () => {
    const survey = await fixture("survey");
    const tree = structuredClone(survey.tree);
    for (const declaration of tree.declarations) {
      const viewer = declaration.viewer as { layout?: string; properties?: { format?: string }[] } | undefined;
      if (viewer === undefined) {
        continue;
      }
      viewer.layout = "bogus";
      for (const property of viewer.properties ?? []) {
        property.format = "bogus";
      }
    }
    const element = show({ ...survey, tree });
    const engineer = cards(element, "Choice")[0]!;
    assert.ok(!engineer.classList.contains("bogus-layout"));
    assert.ok(engineer.querySelector(":scope > .rows, :scope > .card-header > .rows"), "an unknown layout folds nothing away");
    assert.ok(read(engineer).includes("Engineer"));
    for (const value of Array.from(root(element).querySelectorAll(".value.formatted"))) {
      assert.notEqual(read(value), "", "a formatted value keeps its text");
    }
  });

  it("reads a viewer declaration as a card headed viewer and its target, with a line to an element", async () => {
    const element = show(await fixture("viewerDeclaration"));
    const declaration = root(element).querySelector<HTMLElement>(".document > .declaration.viewer-declaration");
    assert.ok(declaration, "a viewer declaration is shown at the top level, unfolded");
    assert.equal(declaration.closest("details"), null);
    assert.equal(read(declaration.querySelector(".declaration-kind")), "viewer");
    assert.equal(read(declaration.querySelector(".declaration-name")), "Choice");
    const strip = declaration;
    const top = declaration.querySelector<HTMLElement>(":scope > .declaration-body > .viewer-element")!;
    const line = top.querySelector(":scope > .viewer-line")!;
    assert.equal(read(line.querySelector(".viewer-kind")), "ElementViewer");
    const title = Array.from(line.querySelectorAll(".viewer-attribute")).find(
      (attribute) => read(attribute.querySelector(".label")) === "title"
    );
    assert.ok(title, "the line names the title");
    assert.equal(read(title), "title=label");
    assert.equal(strip.querySelector(".row.ghost"), null, "a viewer shows no default nobody wrote");
  });
});
