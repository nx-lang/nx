/**
 * Named values: a property offers the named values of its type in a popover, choosing one writes
 * its name, and a name reads as a token that opens them again or is replaced by its value.
 */
import { document } from "./dom.js";

import assert from "node:assert/strict";
import { after, afterEach, describe, it } from "node:test";

import type { NxEditDetail, NxViewerElement } from "../src/index.js";
import { nxHost, treeOf, type Fixture } from "./fixtures.js";

const { NX_EDIT_EVENT } = await import("../src/index.js");

const window = document.defaultView!;
const created: NxViewerElement[] = [];
afterEach(() => {
  for (const element of created.splice(0)) {
    element.remove();
  }
});
after(async () => (await nxHost()).dispose());

const CARD = [
  'import "@nx/design"',
  "",
  'component <Card padding: Spacing = 16 background?: Color radius: Radius = 0 title: string = "" fill: Color = #ffffff /> = {',
  "  <div />",
  "}",
  "",
  "component <Badge tint: Color = #000000 /> = {",
  "  <Card fill={env.color.accent} />",
  "}",
  "",
  "let gap: Spacing = 6",
  "",
  "let wide: Color = #e11d48",
  "",
  "let root() = <Card padding=8 radius={env.radius.md} fill={standardPalette.green500} />",
  "",
  "let tokened() = <Card padding={env.space.md} background={env.color.surface} />",
  "",
  "let tight: Spacing = {env.space.sm}",
  "",
  "let sum: Spacing = { gap + 2 }",
  "",
  "let summed() = <Card padding={sum} />",
  "",
  "let tint: Color = {env.color.accent}",
  "",
  "let tinted() = <Card fill={tint} />",
  ""
].join("\n");

async function card(): Promise<Fixture> {
  return treeOf([{ uri: "nx:///card.nx", identity: "card.nx", source: CARD }], { visualizers: { modules: [] }, suggestions: true });
}

function show({ tree, text }: Fixture, editable = true): NxViewerElement {
  const element = document.createElement("nx-viewer");
  document.body.append(element);
  element.folded = [];
  element.editable = editable;
  element.text = text;
  element.tree = tree;
  element.detailsState = { pinned: tree.nodes.filter((node) => node.role === "element").map((node) => node.key) };
  for (;;) {
    const more = element.shadowRoot!.querySelector<HTMLElement>('.inspector-more[aria-expanded="false"]');
    if (more === null) {
      break;
    }
    more.click();
  }
  created.push(element);
  return element;
}

function root(element: NxViewerElement): ShadowRoot {
  return element.shadowRoot!;
}

function rendered(element: NxViewerElement, key: string): HTMLElement {
  const found = [...root(element).querySelectorAll<HTMLElement>("[data-key]")].find((node) => node.dataset["key"] === key);
  assert.ok(found, `${key} is drawn`);
  return found;
}

function button(element: NxViewerElement, key: string): HTMLElement | undefined {
  return [...root(element).querySelectorAll<HTMLElement>(".named-values")].find((node) => node.dataset["namedKey"] === key);
}

function press(target: Element, key: string, init: KeyboardEventInit = {}): void {
  target.dispatchEvent(new window.KeyboardEvent("keydown", { key, bubbles: true, composed: true, cancelable: true, ...init }));
}

function popover(element: NxViewerElement): HTMLElement {
  return root(element).querySelector<HTMLElement>(".popover")!;
}

/** What the open popover lists: each group's label, then each shown row's name and preview. */
function listed(element: NxViewerElement): string[] {
  const lines: string[] = [];
  const list = popover(element).querySelector(".named-list")!;
  for (const child of list.querySelectorAll<HTMLElement>(":scope > .named-row, :scope > .named-section")) {
    if (child.hidden) {
      continue;
    }
    const rows = child.matches(".named-section") ? [...child.querySelectorAll<HTMLElement>(".named-row")] : [child];
    if (child.matches(".named-section")) {
      lines.push(`# ${child.querySelector(".named-group")!.textContent}`);
    }
    for (const row of rows) {
      if (!row.hidden) {
        const name = row.querySelector(".named-name")!.textContent;
        const preview = row.querySelector(".named-preview")!.textContent;
        lines.push(`${row.classList.contains("current") ? "* " : ""}${name}${preview === "" ? "" : ` ${preview}`}`);
      }
    }
  }
  return lines;
}

function edits(element: NxViewerElement): NxEditDetail[] {
  const found: NxEditDetail[] = [];
  element.addEventListener(NX_EDIT_EVENT, (event) => found.push((event as CustomEvent<NxEditDetail>).detail));
  return found;
}

function choose(element: NxViewerElement, name: string): void {
  const row = [...popover(element).querySelectorAll<HTMLElement>(".named-row")].find((candidate) => candidate.dataset["name"] === name);
  assert.ok(row, `${name} is listed`);
  row.click();
}

describe("named values", () => {
  it("lists a padding's default and the spacing scale, and no radius", async () => {
    const element = show(await card());
    button(element, "root.body.padding")!.click();
    const rows = listed(element);
    assert.equal(rows[0], "Default 16");
    assert.equal(rows[1], "# Environment");
    assert.deepEqual(
      rows.slice(2, 11),
      ["none", "xxs", "xs", "sm", "md", "lg", "xl", "xxl", "xxxl"].map((step, position) => rows[position + 2]!.startsWith(`space.${step} `) ? rows[position + 2]! : `space.${step}`)
    );
    assert.ok(!rows.some((row) => row.startsWith("radius.")), rows.join("\n"));
    assert.ok(rows.includes("# This document") && rows.includes("gap 6"), rows.join("\n"));
    assert.equal(popover(element).querySelector(".named-foot")!.textContent, "Values at their defaults, before any provide or env. override");
    const input = popover(element).querySelector("input")!;
    assert.equal(input.getAttribute("role"), "combobox");
    assert.equal(input.getAttribute("aria-activedescendant"), null, "no row is active until the reader picks one");
    assert.equal(button(element, "root.body.padding")!.getAttribute("aria-haspopup"), "dialog");
    assert.equal(popover(element).querySelector(".named-list .named-empty"), null, "the listbox holds options alone");
    input.value = "zzzz";
    input.dispatchEvent(new window.Event("input"));
    const empty = popover(element).querySelector<HTMLElement>(".named-empty")!;
    assert.equal(empty.getAttribute("role"), "status");
    assert.equal(empty.textContent, "No named values match");
    input.value = "space md";
    input.dispatchEvent(new window.Event("input"));
    assert.equal(popover(element).querySelector(".named-detail")!.textContent, "", "a doc that restates the value is left out");
    assert.equal(popover(element).querySelector(".named-row.active")!.getAttribute("title"), null, "no hover card repeats the detail");
  });

  it("walks the rows with the arrows, and Enter with none active changes nothing", async () => {
    const element = show(await card());
    const fired = edits(element);
    button(element, "root.body.padding")!.click();
    const input = popover(element).querySelector("input")!;
    press(input, "Enter");
    assert.deepEqual(fired, []);
    assert.equal(popover(element).hidden, false, "the popover stays open");
    press(input, "ArrowDown");
    assert.equal(popover(element).querySelector(".named-row.active")?.getAttribute("data-default"), "true");
    press(input, "ArrowDown");
    assert.equal(popover(element).querySelector<HTMLElement>(".named-row.active")?.dataset["name"], "env.space.none");
    press(input, "Enter");
    assert.deepEqual(fired, [{ key: "root.body.padding.value", edit: { kind: "setValue", value: { kind: "name", name: "env.space.none" } } }]);
  });

  it("walks from the marked row", async () => {
    const element = show(await card());
    button(element, "root.body.radius")!.click();
    const input = popover(element).querySelector("input")!;
    assert.equal(popover(element).querySelector<HTMLElement>(".named-row.active")?.dataset["name"], "env.radius.md");
    press(input, "ArrowDown");
    assert.equal(popover(element).querySelector<HTMLElement>(".named-row.active")?.dataset["name"], "env.radius.lg");
  });

  it("shows colors with their swatches and filters by words", async () => {
    const element = show(await card());
    button(element, "root.body.radius")!.click();
    assert.ok(listed(element).some((row) => row.startsWith("* radius.md")), "the current name is marked");
    press(popover(element).querySelector("input")!, "Escape");
    assert.equal(popover(element).hidden, true);
    button(element, "root.body.fill")!.click();
    const rows = listed(element);
    assert.ok(rows.some((row) => row.startsWith("color.accent #")), rows.join("\n"));
    assert.ok(rows.includes("# @nx/design"), rows.join("\n"));
    assert.ok(rows.some((row) => row.startsWith("standardPalette.blue600 #")), rows.join("\n"));
    assert.ok(popover(element).querySelector(".named-row .swatch") !== null);
    const input = popover(element).querySelector("input")!;
    input.value = "text mu";
    input.dispatchEvent(new window.Event("input"));
    const filtered = listed(element).filter((row) => !row.startsWith("#"));
    assert.ok(filtered.length > 0 && filtered.every((row) => /text/i.test(row) && /mu/i.test(row)), filtered.join("\n"));
    assert.ok(filtered.some((row) => row.startsWith("color.text.muted")));
    // A wide color reads by its nearest sRGB hex after a P3 tag; its exact value is in the line
    // under the list.
    input.value = "";
    input.dispatchEvent(new window.Event("input"));
    assert.ok(!listed(element).some((row) => row.includes("oklch(")), listed(element).join("\n"));
    const green = popover(element).querySelector<HTMLElement>('.named-row[data-name="standardPalette.green500"]')!;
    assert.equal(green.querySelector(".gamut-tag")?.textContent, "P3");
    assert.match(green.querySelector(".named-preview")!.textContent ?? "", /^P3#[0-9a-f]{6}$/);
    assert.match(green.dataset["detail"] ?? "", /^oklch\(.*\), outside sRGB/);
    // Search reads descriptions too: the focus color's is "The focus ring."
    input.value = "ring";
    input.dispatchEvent(new window.Event("input"));
    assert.ok(listed(element).some((row) => row.startsWith("color.focus")), listed(element).join("\n"));
  });

  it("offers nothing for a string, and Ctrl+Space opens nothing", async () => {
    const element = show(await card());
    assert.equal(button(element, "root.body.title"), undefined);
    element.selection = "root.body.title";
    press(root(element).querySelector(".body")!, " ", { ctrlKey: true });
    assert.equal(popover(element).hidden, true);
  });

  it("opens from Ctrl+Space and Alt+Escape on the selected property", async () => {
    const element = show(await card());
    element.selection = "root.body.padding";
    press(root(element).querySelector(".body")!, " ", { ctrlKey: true });
    assert.equal(popover(element).hidden, false);
    assert.ok(popover(element).classList.contains("named"));
    press(popover(element).querySelector("input")!, "Escape");
    assert.equal(popover(element).hidden, true);
    assert.equal(element.selection, "root.body.padding", "the selection stays");
    press(root(element).querySelector(".body")!, "Escape", { altKey: true });
    assert.equal(popover(element).hidden, false);
  });

  it("offers a component's own property inside it alone", async () => {
    const element = show(await card());
    button(element, "Badge.body.fill")!.click();
    const inside = listed(element);
    assert.ok(inside.includes("# Badge") && inside.some((row) => row.startsWith("tint")), inside.join("\n"));
    press(popover(element).querySelector("input")!, "Escape");
    button(element, "root.body.fill")!.click();
    assert.ok(!listed(element).includes("# Badge"), listed(element).join("\n"));
  });

  it("shows no button when not editable", async () => {
    const element = show(await card(), false);
    assert.equal(root(element).querySelector(".named-values"), null);
  });

  it("writes a chosen name, then reads it as a muted token", async () => {
    const element = show(await card());
    const fired = edits(element);
    button(element, "root.body.padding")!.click();
    choose(element, "env.space.md");
    assert.deepEqual(fired, [{ key: "root.body.padding.value", edit: { kind: "setValue", value: { kind: "name", name: "env.space.md" } } }]);
    const cell = rendered(element, "root.body.padding").querySelector(".sheet-value")!;
    assert.equal(cell.querySelector(".pending")?.textContent, "{env.space.md}");
    assert.ok(cell.classList.contains("reference-token"));
  });

  it("adds an unset property with a name, and Default leaves it unset", async () => {
    const element = show(await card());
    const fired = edits(element);
    button(element, "root.body.background")!.click();
    choose(element, "env.color.surface");
    assert.deepEqual(fired, [{ key: "root.body", edit: { kind: "addProperty", name: "background", value: { kind: "name", name: "env.color.surface" } } }]);
    // An unset property with a default: its Default row leaves it unset.
    button(element, "tokened.body.radius")!.click();
    const rows = [...popover(element).querySelectorAll<HTMLElement>(".named-row")];
    assert.equal(rows[0]!.dataset["default"], "true");
    rows[0]!.click();
    assert.equal(fired.length, 1, "Default adds nothing");
    assert.ok(button(element, "tokened.body.radius"), "the property is still unset");
  });

  it("keeps the selection when Escape closes a popover a click opened", async () => {
    const element = show(await card());
    rendered(element, "tokened.body.padding").querySelector<HTMLElement>(".reference-token")!.click();
    assert.equal(popover(element).hidden, false);
    press(popover(element).querySelector("input")!, "Escape");
    assert.equal(popover(element).hidden, true);
    assert.equal(element.selection, "tokened.body.padding");
  });

  it("opens from a click on a let's token", async () => {
    const element = show(await card());
    rendered(element, "tight.value").click();
    assert.equal(popover(element).hidden, false);
    assert.ok(listed(element).some((row) => row.startsWith("* space.sm")), listed(element).join("\n"));
  });

  it("previews a name by the value it names where it is written, not a parameter's of the same name", async () => {
    const element = show(await card());
    const swatch = rendered(element, "tinted.body.fill").querySelector<HTMLElement>(".swatch");
    assert.ok(swatch, "the token shows its swatch");
    assert.notEqual(swatch.style.backgroundColor, "rgb(0, 0, 0)", "not Badge's tint default");
    rendered(element, "tinted.body.fill").querySelector<HTMLElement>(".row-menu")!.click();
    const replace = [...root(element).querySelectorAll<HTMLElement>(".menu .menu-item")].find((item) => item.dataset["action"] === "replace");
    assert.ok(replace, "Replace with value is offered");
    assert.match(replace.textContent ?? "", /#155dfc/);
  });

  it("offers no Replace with value for a token without a preview", async () => {
    const element = show(await card());
    rendered(element, "summed.body.padding").querySelector<HTMLElement>(".row-menu")!.click();
    const items = [...root(element).querySelectorAll<HTMLElement>(".menu .menu-item")].map((item) => item.dataset["action"]);
    assert.ok(!items.includes("replace"), items.join(", "));
  });

  it("opens from a click on a token, its name marked, and swaps it", async () => {
    const element = show(await card());
    const fired = edits(element);
    const token = rendered(element, "tokened.body.background").querySelector<HTMLElement>(".reference-token")!;
    assert.ok(token.querySelector(".swatch"), "an environment color token shows its swatch");
    token.click();
    assert.ok(listed(element).some((row) => row.startsWith("* color.surface ")));
    choose(element, "env.color.surface");
    assert.deepEqual(fired, [], "the current name changes nothing");
    token.click();
    choose(element, "env.color.surfaceMuted");
    assert.deepEqual(fired, [{ key: "tokened.body.background.value", edit: { kind: "setValue", value: { kind: "name", name: "env.color.surfaceMuted" } } }]);
  });

  it("resets to the default from the Default row", async () => {
    const element = show(await card());
    const fired = edits(element);
    rendered(element, "tokened.body.padding").querySelector<HTMLElement>(".reference-token")!.click();
    const rows = [...popover(element).querySelectorAll<HTMLElement>(".named-row")];
    rows[0]!.click();
    assert.deepEqual(fired, [{ key: "tokened.body.padding", edit: { kind: "remove" } }]);
  });

  it("keeps a wide color token's name: no Replace with value", async () => {
    const element = show(await card());
    rendered(element, "root.body.fill").querySelector<HTMLElement>(".row-menu")!.click();
    const items = [...root(element).querySelectorAll<HTMLElement>(".menu .menu-item")].map((item) => item.querySelector(".menu-label")!.textContent);
    assert.ok(items.includes("Show in Source"), items.join(", "));
    assert.ok(!items.includes("Replace with value"), items.join(", "));
  });

  it("replaces a token with its value", async () => {
    const element = show(await card());
    const fired = edits(element);
    rendered(element, "tokened.body.padding").querySelector<HTMLElement>(".row-menu")!.click();
    const replace = [...root(element).querySelectorAll<HTMLElement>(".menu .menu-item")].find((item) => item.dataset["action"] === "replace");
    assert.ok(replace, "Replace with value is offered");
    replace.click();
    const field = root(element).querySelector<HTMLElement>(".field")!;
    assert.ok(field, "the number's field opens");
    field.textContent = "14";
    press(field, "Enter");
    assert.deepEqual(fired, [{ key: "tokened.body.padding.value", edit: { kind: "setValue", value: { kind: "number", text: "14" } } }]);
  });

  it("offers a let's named values after its value", async () => {
    const element = show(await card());
    assert.ok(button(element, "gap"), "a typed let has the button");
  });
});

const DRAWING = [
  'import "@nx/graphics" as g',
  'import "@nx/design"',
  "",
  "let drawing() = <g.Circle cx=10 cy=10 r=8 opacity=0.5 fill=<g.Paint.color color=#2563eb/> />",
  "",
  "let box() = <g.Rectangle width=10 height=10 rx=2 />",
  "",
  "let blue: g.Paint = <g.Paint.color color=#2563eb/>",
  "",
  "let tinted() = <g.Circle cx=1 cy=1 r=1 fill=<g.Paint.color color={env.color.accent}/> />",
  ""
].join("\n");

describe("named values in graphics", () => {
  it("keeps numbers typed by design aliases editable", async () => {
    const element = show(await treeOf([{ uri: "nx:///drawing.nx", identity: "drawing.nx", source: DRAWING }], { visualizers: { modules: [] }, suggestions: true }));
    assert.ok(rendered(element, "box.body.rx.value").classList.contains("edit-number"), "rx edits as a number");
    assert.ok(rendered(element, "drawing.body.opacity.value").classList.contains("edit-number"), "opacity edits as a number");
    assert.ok(button(element, "box.body.rx"), "rx offers radii");
    button(element, "box.body.rx")!.click();
    assert.ok(listed(element).some((row) => row.startsWith("radius.md ")), listed(element).join("\n"));
  });

  it("binds a paint written as an element to a named paint", async () => {
    const element = show(await treeOf([{ uri: "nx:///drawing.nx", identity: "drawing.nx", source: DRAWING }], { visualizers: { modules: [] }, suggestions: true }));
    const fill = button(element, "drawing.body.fill");
    assert.ok(fill, "a paint element offers named paints");
    const changes = edits(element);
    fill.click();
    assert.ok(listed(element).includes("blue"), listed(element).join("\n"));
    choose(element, "blue");
    assert.deepEqual(changes, [{ key: "drawing.body.fill.value", edit: { kind: "setValue", value: { kind: "name", name: "blue" } } }]);
  });

  it("offers a line property's named values from Ctrl+Space and a click on its token", async () => {
    const element = show(await treeOf([{ uri: "nx:///drawing.nx", identity: "drawing.nx", source: DRAWING }], { visualizers: { modules: [] }, suggestions: true }));
    element.selection = "drawing.body.fill.value.color";
    press(root(element).querySelector(".body")!, " ", { ctrlKey: true });
    assert.equal(popover(element).hidden, false);
    assert.ok(listed(element).some((row) => row.startsWith("color.accent ")), listed(element).join("\n"));
    press(popover(element).querySelector("input")!, "Escape");
    root(element).querySelector<HTMLElement>('[data-field="tinted.body.fill.value.color"] [data-hover="tinted.body.fill.value.color.value"]')!.click();
    assert.equal(popover(element).hidden, false);
    assert.ok(listed(element).some((row) => row.startsWith("* color.accent ")), listed(element).join("\n"));
  });

  it("offers colors with their swatches for a paint's color, unfolded", async () => {
    const element = show(await treeOf([{ uri: "nx:///drawing.nx", identity: "drawing.nx", source: DRAWING }], { visualizers: { modules: [] }, suggestions: true }));
    const fold = [...root(element).querySelectorAll<HTMLElement>(".value-fold[data-value-key]")].find((node) => node.dataset["valueKey"] === "drawing.body.fill.value");
    assert.ok(fold, "the paint unfolds");
    fold.click();
    const color = button(element, "drawing.body.fill.value.color");
    assert.ok(color, "the paint's color offers named values");
    color.click();
    const rows = listed(element);
    assert.ok(rows.includes("# Environment") && rows.some((row) => row.startsWith("color.accent #155dfc")), rows.join("\n"));
    assert.ok(rows.includes("# @nx/design") && rows.some((row) => row.startsWith("standardPalette.blue600 #155dfc")), rows.join("\n"));
    assert.ok(popover(element).querySelector(".named-row .swatch") !== null);
  });
});
