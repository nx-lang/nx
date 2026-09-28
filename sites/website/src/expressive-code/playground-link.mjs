/**
 * "Open in playground" on the site's `nx` code blocks.
 *
 * <para>Every block the docs check treats as a complete program, unmarked or marked `invalid`, gets
 * a link to `/play#code=<payload>` for exactly its text. The payload is the playground's address
 * encoding: the UTF-8 text, raw DEFLATE, unpadded base64url. It is computed here at build time, so
 * the page needs no script to follow it. `fragment` and `output` blocks are not programs, so they get
 * no link.</para>
 *
 * <para>This is the website's own copy of the encoder, as any site linking into the playground
 * would have. The playground's `src/share/fixture.json` holds the two to one encoding.</para>
 */
import { deflateRawSync } from "node:zlib";
import { definePlugin } from "@astrojs/starlight/expressive-code";
import { h } from "@astrojs/starlight/expressive-code/hast";

/** Where the playground lives on the site. */
export const PLAYGROUND_PATH = "/play";

/** The fence words that mark a block as something other than a complete program. */
const NOT_A_PROGRAM = new Set(["fragment", "output"]);

/** The playground's payload for `source`. */
export function encodeSource(source) {
  return deflateRawSync(Buffer.from(source, "utf8")).toString("base64url");
}

/** The playground address that opens with `source`. */
export function playgroundHref(source) {
  return `${PLAYGROUND_PATH}#code=${encodeSource(source)}`;
}

/** Whether a block with this language and meta gets the link. */
export function opensInPlayground(language, meta) {
  if (language !== "nx") {
    return false;
  }
  const words = (meta ?? "").split(/\s+/);
  return !words.some((word) => NOT_A_PROGRAM.has(word));
}

/** The Expressive Code plugin that adds the link below each block that gets one. */
export function openInPlayground() {
  return definePlugin({
    name: "nx-open-in-playground",
    baseStyles: `
      .nx-open-in-playground {
        display: block;
        margin-block-start: 0.25rem;
        text-align: end;
        font-size: var(--sl-text-xs, 0.8125rem);
      }
      .nx-open-in-playground a {
        color: var(--sl-color-text-accent, inherit);
        text-decoration: none;
      }
      .nx-open-in-playground a:hover {
        text-decoration: underline;
      }
    `,
    hooks: {
      postprocessRenderedBlock: ({ codeBlock, renderData }) => {
        if (!opensInPlayground(codeBlock.language, codeBlock.meta)) {
          return;
        }
        renderData.blockAst.children.push(
          h("div", { className: ["nx-open-in-playground"] }, [
            h("a", { href: playgroundHref(codeBlock.code) }, "Open in playground ↗"),
          ]),
        );
      },
    },
  });
}
