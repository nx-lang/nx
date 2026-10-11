/**
 * Inline markdown as CommonMark renders it, checked on examples from the emphasis section of the
 * CommonMark specification and on the text a reader is most likely to meet in agent prompts.
 */
import { document } from "./dom.js";

import assert from "node:assert/strict";
import { describe, it } from "node:test";

const { renderText } = await import("../src/text.js");

/** The markup `source` renders to as markdown, with the keyed spans of its runs unwrapped. */
function inline(source: string): string {
  const body = renderText(document, [{ kind: "text", key: "k", value: source, raw: false }], true, "none");
  const markup = (node: Node): string => {
    if (node.nodeType === 3) {
      return node.textContent ?? "";
    }
    const element = node as Element;
    const inner = Array.from(element.childNodes).map(markup).join("");
    const tag = element.tagName.toLowerCase();
    return tag === "em" || tag === "strong" || tag === "code" ? `<${tag}>${inner}</${tag}>` : inner;
  };
  return markup(body.querySelector("p")!);
}

describe("inline markdown", () => {
  const cases: [string, string][] = [
    ["*foo bar*", "<em>foo bar</em>"],
    ["a * foo bar*", "a * foo bar*"],
    ["foo*bar*", "foo<em>bar</em>"],
    ["5*6*78", "5<em>6</em>78"],
    ["_foo bar_", "<em>foo bar</em>"],
    ["foo_bar_", "foo_bar_"],
    ["**foo bar**", "<strong>foo bar</strong>"],
    ["__foo bar__", "<strong>foo bar</strong>"],
    ["__foo__bar", "__foo__bar"],
    ["**foo**bar", "<strong>foo</strong>bar"],
    ["*foo**bar**baz*", "<em>foo<strong>bar</strong>baz</em>"],
    ["*foo**bar*", "<em>foo**bar</em>"],
    ["**foo*bar*baz**", "<strong>foo<em>bar</em>baz</strong>"],
    ["***foo***", "<em><strong>foo</strong></em>"],
    ["foo***bar***baz", "foo<em><strong>bar</strong></em>baz"],
    ["*a **b** c*", "<em>a <strong>b</strong> c</em>"],
    ["*(*foo*)*", "<em>(<em>foo</em>)</em>"],
    ['**foo "*bar*" foo**', '<strong>foo "<em>bar</em>" foo</strong>'],
    ["*foo`*`", "*foo<code>*</code>"],
    ["`a*b*c`", "<code>a*b*c</code>"],
    ["``a`b``", "<code>a`b</code>"],
    ["`unclosed", "`unclosed"],
    ["`` foo ` bar ``", "<code>foo ` bar</code>"],
    ["` `` `", "<code>``</code>"],
    ["` a`", "<code> a</code>"],
    ["find_plans_for_team and snake__case__x", "find_plans_for_team and snake__case__x"],
    ["2 * 3 * 4, x ** y ** z and x __ y __ z", "2 * 3 * 4, x ** y ** z and x __ y __ z"],
    ["Use __init__ or ** for powers ** here", "Use <strong>init</strong> or ** for powers ** here"],
  ];
  for (const [source, expected] of cases) {
    it(`renders ${source}`, () => {
      assert.equal(inline(source), expected);
    });
  }
});
