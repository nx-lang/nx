import { dom, document } from "./dom.js";

import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { renderMarkdown } from "../src/markdown.js";

/** `markdown` rendered, as the markup it builds. */
function html(markdown: string): string {
  const holder = document.createElement("div");
  holder.append(renderMarkdown(document, markdown));
  return holder.innerHTML;
}

describe("the built-in Markdown", () => {
  it("renders paragraphs, emphasis and inline code", () => {
    assert.equal(html("What the *shape* draws.\nOn **two** lines.\n\nUse `Circle`."), "<p>What the <em>shape</em> draws.\nOn <strong>two</strong> lines.</p><p>Use <code>Circle</code>.</p>");
    assert.equal(html("a_b_c and _word_ and __strong__"), "<p>a_b_c and <em>word</em> and <strong>strong</strong></p>");
    assert.equal(html("``a `b` c`` and \\*not\\*"), "<p><code>a `b` c</code> and *not*</p>");
    assert.equal(html("2 * 3 * 4"), "<p>2 * 3 * 4</p>", "a marker with spaces around it is text");
  });

  it("renders fenced code and lists", () => {
    assert.equal(html("Example:\n\n```nx\n<Shape Type=Circle />\n```\n"), "<p>Example:</p><pre><code>&lt;Shape Type=Circle /&gt;</code></pre>");
    assert.equal(html("- one\n- *two*\n  goes on\n\n3. three\n4. four"), "<ul><li>one</li><li><em>two</em>\ngoes on</li></ul><ol start=\"3\"><li>three</li><li>four</li></ol>");
  });

  it("links only to the web, opening outside the page", () => {
    assert.equal(
      html("See [the *docs*](https://nxlang.org/docs \"title\")."),
      '<p>See <a href="https://nxlang.org/docs" target="_blank" rel="noopener noreferrer">the <em>docs</em></a>.</p>'
    );
    assert.equal(html("[run](javascript:alert(1)) [mail](mailto:a@b.c) [here](#x)"), "<p>run mail here</p>");
  });

  it("reads raw HTML as its text and runs nothing", () => {
    const holder = document.createElement("div");
    holder.append(renderMarkdown(document, 'An <img src=x onerror="alert(1)"> and <script>alert(2)</script>'));
    assert.equal(holder.querySelector("img, script"), null, "no element from the doc's markup");
    assert.equal(holder.textContent, 'An <img src=x onerror="alert(1)"> and <script>alert(2)</script>');
    assert.equal(dom.window.document.querySelectorAll("img").length, 0);
  });
});
