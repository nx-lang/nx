import { clipboardWrites, document } from "./dom.js";

import assert from "node:assert/strict";
import { after, before, describe, it } from "node:test";

import { createHighlighterCore } from "shiki/core";
import { createOnigurumaEngine } from "shiki/engine/oniguruma";
import githubDark from "shiki/themes/github-dark.mjs";
import githubLight from "shiki/themes/github-light.mjs";

import { card, greeting, nested, person, tasks, user } from "./fixtures.js";
import type { NxValueElement, NxValueHighlighter, NxValueNavigateDetail } from "../src/index.js";

const elementModule = await import("../src/index.js");
const { NX_VALUE_NAVIGATE_EVENT, nxShikiLanguage, DEFAULT_THEMES } = elementModule;
const { loadHighlighter } = await import("../src/highlight.js");

const created: NxValueElement[] = [];

after(() => {
  for (const element of created) {
    element.remove();
  }
});

function show(value = user): NxValueElement {
  const element = document.createElement("nx-value");
  document.body.append(element);
  element.value = value;
  created.push(element);
  return element;
}

function shadow(element: NxValueElement): ShadowRoot {
  assert.ok(element.shadowRoot, "the element has an open shadow root");
  return element.shadowRoot;
}

function code(element: NxValueElement): HTMLElement {
  return shadow(element).querySelector("code")!;
}

function nodeElement(element: NxValueElement, index: number): HTMLElement {
  return shadow(element).querySelector<HTMLElement>(`.node[data-node="${index}"]`)!;
}

/** The visible text, leaving out what folding hides and the buttons it adds. */
function visibleText(root: Element): string {
  let text = "";
  const walk = (node: Node) => {
    if (node.nodeType === 3) {
      text += node.textContent;
      return;
    }
    const element = node as HTMLElement;
    if (element.hidden || element.tagName === "BUTTON") {
      return;
    }
    element.childNodes.forEach(walk);
  };
  walk(root);
  return text;
}

async function until(condition: () => boolean, what: string, timeout = 15_000): Promise<void> {
  const start = Date.now();
  while (!condition()) {
    if (Date.now() - start > timeout) {
      assert.fail(`timed out waiting for ${what}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}

function pointAt(target: Element): void {
  // jsdom has no PointerEvent; the element reads nothing but the event's target.
  target.dispatchEvent(new window.MouseEvent("pointermove", { bubbles: true }));
}

function hoverText(element: NxValueElement): string | undefined {
  const hover = shadow(element).querySelector<HTMLElement>(".hover")!;
  return hover.hidden ? undefined : hover.textContent ?? "";
}

async function hoverOf(element: NxValueElement, target: Element): Promise<string> {
  pointAt(target);
  await until(() => hoverText(element) !== undefined, "the hover");
  return hoverText(element)!;
}

/** Finds the first token or text container holding `text` exactly. */
function tokenWith(element: NxValueElement, text: string): HTMLElement {
  const token = Array.from(code(element).querySelectorAll<HTMLElement>(".t")).find(
    (span) => span.textContent === text
  );
  assert.ok(token, `a token reading ${JSON.stringify(text)}`);
  return token;
}

describe("one element shows an NX value in any host", () => {
  it("shows the value on a plain page once its value is set", () => {
    const element = show();
    assert.equal(element.tagName.toLowerCase(), "nx-value");
    assert.equal(code(element).textContent, user.text);
  });

  it("keeps its markup out of the page, so page styles and queries do not reach it", () => {
    const style = document.createElement("style");
    style.textContent = "pre, span, button { display: none; color: red; }";
    document.head.append(style);
    try {
      const element = show();
      assert.equal(element.children.length, 0, "nothing is rendered in the light DOM");
      assert.equal(document.querySelector("nx-value pre"), null);
      assert.ok(shadow(element).querySelector("pre"));
    } finally {
      style.remove();
    }
  });
});

describe("an element defined after its properties were set", () => {
  it("takes the properties a host set before the definition", () => {
    const element = document.createElement("nx-value-later") as HTMLElement & { value?: unknown; stale?: unknown };
    element.value = user;
    element.stale = true;
    document.body.append(element);
    const { NxValueElement } = elementModule;
    customElements.define("nx-value-later", class extends NxValueElement {});
    const upgraded = element as unknown as NxValueElement;
    created.push(upgraded);
    assert.equal(code(upgraded).textContent, user.text);
    assert.equal(upgraded.hasAttribute("stale"), true);
  });
});

describe("the value reads as NX", () => {
  it("shows the value uncolored until its highlighter loads, then colors it in place", async () => {
    const element = show(tasks);
    const record = tasks.nodes.findIndex((node) => node.role === "record");
    element.setFolded(record, true);
    await until(() => code(element).querySelector(".t") !== null, "the highlighter");
    assert.equal(element.isFolded(record), true, "folding survives the recoloring");
  });

  it("shows the text exactly", () => {
    assert.equal(code(show()).textContent, '<User id="1" name="Ada" />');
    assert.equal(visibleText(code(show(tasks))), tasks.text);
  });

  it("highlights with the published grammar, in light and dark variants", async () => {
    const element = show();
    await until(() => code(element).querySelector(".t") !== null, "the highlighter");

    const highlighter = await createHighlighterCore({
      themes: [githubLight, githubDark],
      langs: [nxShikiLanguage()],
      engine: createOnigurumaEngine(import("shiki/wasm"))
    });
    const expected = new Map<string, { light: string | undefined; dark: string | undefined }>();
    for (const line of highlighter.codeToTokensWithThemes(user.text, {
      lang: "nx",
      themes: { light: "github-light", dark: "github-dark" }
    })) {
      for (const token of line) {
        expected.set(token.content.trim(), {
          light: token.variants["light"]?.color,
          dark: token.variants["dark"]?.color
        });
      }
    }

    for (const text of ["User", "id", "name", '"1"', '"Ada"']) {
      const colors = expected.get(text);
      assert.ok(colors, `the grammar colors ${text}`);
      const token = tokenWith(element, text);
      assert.equal(token.style.getPropertyValue("--l").toLowerCase(), colors.light?.toLowerCase(), `light ${text}`);
      assert.equal(token.style.getPropertyValue("--d").toLowerCase(), colors.dark?.toLowerCase(), `dark ${text}`);
    }
    // The identifier and the string are colored differently, so the grammar is doing the work.
    assert.notEqual(tokenWith(element, "User").style.getPropertyValue("--l"), tokenWith(element, '"Ada"').style.getPropertyValue("--l"));
  });

  it("uses a shared highlighter rather than loading one", () => {
    const calls: string[] = [];
    const highlighter: NxValueHighlighter = {
      codeToTokensWithThemes(code) {
        calls.push(code);
        return [[{ content: "<User", offset: 0, variants: { light: { color: "#123456" }, dark: { color: "#654321" } } }]];
      }
    };
    const element = document.createElement("nx-value");
    element.highlighter = highlighter;
    element.value = user;
    created.push(element);

    assert.deepEqual(calls, [user.text]);
    const token = tokenWith(element, "<User");
    assert.equal(token.style.getPropertyValue("--l"), "#123456");
    assert.equal(token.style.getPropertyValue("--d"), "#654321");
  });

  it("shows a value longer than the coloring limit uncolored, and still folds it", () => {
    const { MAX_COLORED_CHARACTERS } = elementModule;
    const record = tasks.nodes.findIndex((node) => node.role === "record");
    const padding = " ".repeat(MAX_COLORED_CHARACTERS);
    // The same value with a long run of trailing spaces: past the limit, and nodes unchanged.
    const long = { text: tasks.text + padding, nodes: tasks.nodes };
    let calls = 0;
    const element = document.createElement("nx-value");
    element.highlighter = {
      codeToTokensWithThemes() {
        calls += 1;
        return [];
      }
    };
    element.value = long;
    created.push(element);
    assert.equal(calls, 0);
    assert.equal(code(element).querySelector(".t"), null);
    element.setFolded(record, true);
    assert.equal(element.isFolded(record), true);
  });

  it("follows the page's color scheme through light-dark()", () => {
    const style = shadow(show()).querySelector("style")!.textContent!;
    assert.match(style, /\.t \{ color: light-dark\(var\(--l, inherit\), var\(--d, inherit\)\); \}/);
  });
});

describe("long values fold", () => {
  const recordIndices = tasks.nodes
    .map((node, index) => ({ node, index }))
    .filter(({ node }) => node.role === "record" && node.type === "Task")
    .map(({ index }) => index);

  it("folds one record of a sequence to its first line and an ellipsis", () => {
    const element = show(tasks);
    const [first, second, third] = recordIndices.map((index) => nodeElement(element, index));
    const toggle = second!.querySelector<HTMLButtonElement>(":scope > .fold")!;
    toggle.click();

    assert.equal(toggle.getAttribute("aria-expanded"), "false");
    // The first line, then the ellipsis button, which visibleText leaves out.
    assert.equal(visibleText(second!), "<Task ");
    assert.equal(second!.querySelector<HTMLElement>(":scope > .folded .ellipsis")!.textContent, "…");
    assert.equal(visibleText(first!), tasks.text.slice(tasks.nodes[recordIndices[0]!]!.start, tasks.nodes[recordIndices[0]!]!.end));
    assert.equal(visibleText(third!), tasks.text.slice(tasks.nodes[recordIndices[2]!]!.start, tasks.nodes[recordIndices[2]!]!.end));
  });

  it("unfolds from the ellipsis as well as the toggle", () => {
    const element = show(tasks);
    const index = recordIndices[1]!;
    element.setFolded(index, true);
    nodeElement(element, index).querySelector<HTMLButtonElement>(":scope > .folded .ellipsis")!.click();
    assert.equal(element.isFolded(index), false);
  });

  it("starts with everything unfolded", () => {
    const element = show(tasks);
    assert.ok(tasks.nodes.every((_, index) => !element.isFolded(index)));
  });

  it("offers no toggle on the value as a whole, a top-level sequence or a record", () => {
    const sequence = show(tasks);
    assert.equal(nodeElement(sequence, 0).querySelector(":scope > .fold"), null);
    assert.ok(nodeElement(sequence, recordIndices[0]!).querySelector(":scope > .fold"));
    const record = show(person);
    assert.equal(nodeElement(record, 0).querySelector(":scope > .fold"), null);
  });

  it("puts toggles only beside nodes that start their line, at most one a line", () => {
    const element = show(nested);
    const toggled = nested.nodes
      .map((node, index) => ({ node, index }))
      .filter(({ index }) => nodeElement(element, index).querySelector(":scope > .fold") !== null);
    assert.ok(toggled.length > 0, "a nested value still folds");
    const lines = new Set<number>();
    for (const { node } of toggled) {
      const lineStart = nested.text.lastIndexOf("\n", node.start - 1) + 1;
      assert.match(nested.text.slice(lineStart, node.start), /^ *$/, `node at ${node.start} starts its line`);
      const line = nested.text.slice(0, node.start).split("\n").length;
      assert.ok(!lines.has(line), `one toggle on line ${line}`);
      lines.add(line);
    }
    // A record that starts partway along a line, `content={<li`, has none.
    const midLine = nested.nodes.findIndex((node, index) => node.role === "record" && index > 0 && !/^ *$/.test(nested.text.slice(nested.text.lastIndexOf("\n", node.start - 1) + 1, node.start)) && nested.text.slice(node.start, node.end).includes("\n"));
    assert.ok(midLine > 0);
    assert.equal(nodeElement(element, midLine).querySelector(":scope > .fold"), null);
  });

  it("offers no toggle on a one-line value", () => {
    assert.equal(shadow(show()).querySelector(".fold"), null);
  });

  it("copies the whole text while folded", async () => {
    const element = show(tasks);
    element.setFolded(recordIndices[0]!, true);
    shadow(element).querySelector<HTMLButtonElement>(".copy")!.click();
    await until(() => clipboardWrites.at(-1) === tasks.text, "the copy");
  });
});

describe("hover explains a value", () => {
  // Every element after this colors at once, so none re-renders while a test points at it.
  before(() => loadHighlighter(DEFAULT_THEMES));

  it("describes a property by default as the language service would", async () => {
    const element = show(tasks);
    const done = tasks.nodes.findIndex((node) => node.name === "done");
    assert.equal(await hoverOf(element, nodeElement(element, done)), "(property) done: boolean");
  });

  it("describes a sequence by its type and count", async () => {
    const element = show(tasks);
    const sequence = nodeElement(element, 0);
    const text = await hoverOf(element, sequence);
    assert.match(text, /Task\*/);
    assert.match(text, /3 items/);
  });

  it("describes an optional property with the mark on its name", async () => {
    const element = show(card);
    const subtitle = card.nodes.findIndex((node) => node.name === "subtitle");
    assert.equal(await hoverOf(element, nodeElement(element, subtitle)), "(property) subtitle?: string");
  });

  it("shows the host's description instead of the default", async () => {
    const element = show();
    const asked: number[] = [];
    element.describe = async (_node, index) => {
      asked.push(index);
      return "```nx\ntype User = { id:string name:string }\n```\n\nFrom the `host`.";
    };
    const text = await hoverOf(element, nodeElement(element, 0));
    assert.deepEqual(asked, [0]);
    assert.match(text, /type User = \{ id:string name:string \}/);
    assert.match(text, /From the host\./);
    assert.ok(shadow(element).querySelector(".hover p code"), "inline code is code");
  });

  it("falls back to the default when the host has nothing to say", async () => {
    const element = show();
    element.describe = () => undefined;
    assert.equal(await hoverOf(element, nodeElement(element, 0)), "User");
  });

  it("shows the same description when a node is focused from the keyboard", async () => {
    const element = show();
    nodeElement(element, 1).focus();
    await until(() => hoverText(element) !== undefined, "the hover");
    assert.equal(hoverText(element), "(property) id: string");
  });

  it("moves between nodes with the arrow keys", () => {
    const element = show();
    const pre = shadow(element).querySelector("pre")!;
    nodeElement(element, 0).focus();
    pre.dispatchEvent(new window.KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    assert.equal(shadow(element).activeElement, nodeElement(element, 1));
  });
});

describe("the element reports where a value is declared", () => {
  it("fires nx-value-navigate with the declaration when a type name is clicked", () => {
    const element = show();
    const events: NxValueNavigateDetail[] = [];
    element.addEventListener(NX_VALUE_NAVIGATE_EVENT, (event) => events.push((event as CustomEvent<NxValueNavigateDetail>).detail));
    nodeElement(element, 0).dispatchEvent(new window.MouseEvent("click", { bubbles: true }));

    assert.equal(events.length, 1);
    assert.equal(events[0]!.index, 0);
    assert.equal(events[0]!.node.type, "User");
    assert.deepEqual(events[0]!.declaration, user.nodes[0]!.declaration);
  });

  it("fires it with Enter on a focused node", () => {
    const element = show();
    let fired = 0;
    element.addEventListener(NX_VALUE_NAVIGATE_EVENT, () => (fired += 1));
    nodeElement(element, 2).focus();
    shadow(element).querySelector("pre")!.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    assert.equal(fired, 1);
  });

  it("fires nothing for a node with no declaration", () => {
    const element = show(greeting);
    let fired = 0;
    element.addEventListener(NX_VALUE_NAVIGATE_EVENT, () => (fired += 1));
    nodeElement(element, 0).dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
    assert.equal(fired, 0);
  });
});

describe("the element marks cut, stale and copied values", () => {
  it("shows a notice after cut text", () => {
    const element = show();
    const notice = shadow(element).querySelector<HTMLElement>(".notice")!;
    assert.equal(notice.hidden, true);
    element.truncated = true;
    assert.equal(notice.hidden, false);
    assert.match(notice.textContent!, /rest of the value was cut/);
  });

  it("mutes a stale value and badges it, and still hovers and folds", async () => {
    const element = show(tasks);
    element.setAttribute("stale", "");
    const badge = shadow(element).querySelector<HTMLElement>(".badge")!;
    assert.equal(badge.hidden, false);
    assert.equal(badge.textContent, "Out of date");
    assert.match(shadow(element).querySelector("style")!.textContent!, /:host\(\[stale\]\) code \{ opacity/);

    const done = tasks.nodes.findIndex((node) => node.name === "done");
    assert.equal(await hoverOf(element, nodeElement(element, done)), "(property) done: boolean");
    const record = tasks.nodes.findIndex((node) => node.role === "record");
    element.setFolded(record, true);
    assert.equal(element.isFolded(record), true);
  });

  it("copies the text and confirms it", async () => {
    const element = show();
    const copy = shadow(element).querySelector<HTMLButtonElement>(".copy")!;
    copy.click();
    await until(() => copy.textContent === "Copied", "the confirmation");
    assert.equal(clipboardWrites.at(-1), user.text);
  });
});
