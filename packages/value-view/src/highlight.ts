/**
 * Coloring NX text with the grammar the editors use, through Shiki.
 *
 * <para>The element needs one thing from a highlighter: a list of colored ranges over the whole
 * text, in a light and a dark variant. Everything Shiki-specific stays in this file.</para>
 */
import nxGrammar from "@nx-lang/language/grammar" with { type: "json" };
import type { LanguageInput } from "shiki";

/** The language id the NX grammar is loaded under. */
export const NX_LANGUAGE_ID = "nx";

/** The light and dark Shiki themes a value is colored with. */
export interface NxValueThemes {
  readonly light: string;
  readonly dark: string;
}

/** The themes used when a host names none: the ones `@nx-lang/monaco` loads by default. */
export const DEFAULT_THEMES: NxValueThemes = { light: "github-light", dark: "github-dark" };

/**
 * The part of a Shiki highlighter the element uses. A `HighlighterCore` from `shiki/core` with the
 * NX grammar and both themes loaded satisfies it.
 */
export interface NxValueHighlighter {
  codeToTokensWithThemes(
    code: string,
    options: { lang: string; themes: Record<string, string> }
  ): ReadonlyArray<ReadonlyArray<ShikiToken>>;
}

interface ShikiToken {
  readonly content: string;
  readonly offset: number;
  readonly variants: Readonly<Record<string, { color?: string; fontStyle?: number }>>;
}

/** One colored range of the text, in UTF-16 offsets. */
export interface ColoredRange {
  readonly start: number;
  readonly end: number;
  readonly light?: string;
  readonly dark?: string;
  /** Shiki's font style bits: 1 italic, 2 bold, 4 underline. */
  readonly fontStyle?: number;
}

/** The published grammar as a Shiki language, under the id `nx`. */
export function nxShikiLanguage(): LanguageInput {
  return { ...(nxGrammar as object), name: NX_LANGUAGE_ID, aliases: ["NX"] } as LanguageInput;
}

/** Colors `text` as NX. Ranges are in text order and do not overlap; line breaks are not in any. */
export function colorize(
  highlighter: NxValueHighlighter,
  text: string,
  themes: NxValueThemes
): ColoredRange[] {
  const lines = highlighter.codeToTokensWithThemes(text, {
    lang: NX_LANGUAGE_ID,
    themes: { light: themes.light, dark: themes.dark }
  });
  const ranges: ColoredRange[] = [];
  for (const line of lines) {
    for (const token of line) {
      const light = token.variants["light"];
      const dark = token.variants["dark"];
      ranges.push({
        start: token.offset,
        end: token.offset + token.content.length,
        ...(light?.color === undefined ? {} : { light: light.color }),
        ...(dark?.color === undefined ? {} : { dark: dark.color }),
        ...(light?.fontStyle ? { fontStyle: light.fontStyle } : {})
      });
    }
  }
  return ranges;
}

const loading = new Map<string, Promise<NxValueHighlighter>>();
const loaded = new Map<string, NxValueHighlighter>();

const themeKey = (themes: NxValueThemes) => `${themes.light}\n${themes.dark}`;

/**
 * Loads a highlighter with the NX grammar and `themes`, once per pair of themes for the page, so
 * every element on it shares one.
 */
export function loadHighlighter(themes: NxValueThemes): Promise<NxValueHighlighter> {
  const key = themeKey(themes);
  let highlighter = loading.get(key);
  if (highlighter === undefined) {
    highlighter = createHighlighter(themes);
    loading.set(key, highlighter);
    highlighter.then(
      (ready) => loaded.set(key, ready),
      // A failed load is not remembered, so a later element can try again.
      () => loading.delete(key)
    );
  }
  return highlighter;
}

/** The highlighter for `themes` if one has finished loading, so an element can color at once. */
export function loadedHighlighter(themes: NxValueThemes): NxValueHighlighter | undefined {
  return loaded.get(themeKey(themes));
}

async function createHighlighter(themes: NxValueThemes): Promise<NxValueHighlighter> {
  // Shiki's core rather than its full bundle, as `@nx-lang/monaco` does: the full bundle's registry
  // references every bundled language, and one grammar is all a value needs.
  const [{ createHighlighterCore }, { createOnigurumaEngine }, { bundledThemes }] = await Promise.all([
    import("shiki/core"),
    import("shiki/engine/oniguruma"),
    import("shiki/themes")
  ]);
  const theme = (name: string) => {
    const bundled = bundledThemes[name as keyof typeof bundledThemes];
    if (bundled === undefined) {
      throw new Error(`'${name}' is not a Shiki bundled theme.`);
    }
    return bundled;
  };
  const highlighter = await createHighlighterCore({
    themes: [theme(themes.light), theme(themes.dark)],
    langs: [nxShikiLanguage()],
    engine: createOnigurumaEngine(import("shiki/wasm"))
  });
  return highlighter as unknown as NxValueHighlighter;
}
