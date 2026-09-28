# @nx-lang/value-view

An `<nx-value>` custom element that shows an NX value the way NX writes it: the text `nxlang run`
prints, highlighted with the grammar the NX editors use, folding where it spans several lines, and
explaining each part on hover. It has no framework dependency and keeps its markup and styles in a
shadow root, so it drops into a plain page, a React app, a docs site or a VS Code webview alike.

The value it shows is the annotated text `NxProgramArtifact.evaluateNx()` returns in
[`@nx-lang/sdk-wasm`](../../bindings/wasm/README.md): `{ text, nodes }`, where each node says what one
part of the text is (a record, a property, a sequence, a case, a scalar) with its type and, when
the program declares it, where.

## A plain page

```html
<nx-value id="output"></nx-value>

<script type="module">
  import "@nx-lang/value-view";
  import { createNxHost, compileNxModule } from "@nx-lang/sdk-wasm";
  // With a bundler such as Vite; otherwise, wherever the page serves the package's `nx.wasm`.
  import nxModuleUrl from "@nx-lang/sdk-wasm/nx.wasm?url";

  const host = createNxHost(await compileNxModule(fetch(nxModuleUrl)));
  const artifact = host.buildProgramArtifact(`type User = { id:string name:string }
<User id="1" name="Ada" />`);
  try {
    document.getElementById("output").value = artifact.evaluateNx();
  } finally {
    artifact.dispose();
  }
</script>
```

Importing the package defines the element. Where there is no DOM, as in a server render, importing
it does nothing, and `defineNxValueElement(registry)` defines it in a registry of your choosing.

## React

React 19 passes properties to custom elements directly, so no wrapper is needed:

```tsx
import "@nx-lang/value-view";
import type { NxValueText } from "@nx-lang/value-view";

export function Output({ value, stale }: { value: NxValueText; stale: boolean }) {
  return <nx-value value={value} stale={stale} />;
}
```

Declare the element for JSX once in your app:

```ts
import type { NxValueElement } from "@nx-lang/value-view";

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      "nx-value": React.DetailedHTMLProps<React.HTMLAttributes<NxValueElement>, NxValueElement> &
        Partial<Pick<NxValueElement, "value" | "stale" | "truncated" | "describe" | "highlighter">>;
    }
  }
}
```

## Properties and attributes

| Name          | Kind                    | Meaning                                                                    |
| ------------- | ----------------------- | -------------------------------------------------------------------------- |
| `value`       | property                | The `{ text, nodes }` to show                                              |
| `truncated`   | property and attribute  | A notice after the text says the rest was cut                              |
| `stale`       | property and attribute  | The text is muted and badged "Out of date"; hover and folding still work   |
| `describe`    | property                | The host's hover for a node; see below                                     |
| `highlighter` | property                | A Shiki highlighter to share, with the NX grammar and both themes loaded   |
| `themes`      | property                | `{ light, dark }` Shiki theme names; default `github-light`, `github-dark` |

Without a `highlighter`, the element loads Shiki's core, the oniguruma engine and the two themes on
first use, once for the whole page, and shows the value uncolored until they arrive. A page that
already colors with Shiki can pass its highlighter instead; `nxShikiLanguage()` returns the grammar
in the form Shiki takes, for building one.

A value longer than `MAX_COLORED_CHARACTERS` (20,000) is shown uncolored: coloring runs on the main
thread over the whole text, and at that size it would hold up the page on every change. It still
folds, hovers and copies.

Do not pass the highlighter behind a Monaco editor. The element colors with a light and a dark
theme, which switches the highlighter's current theme, and `@shikijs/monaco` tokenizes with
whatever theme that is, so the editor would recolor from the wrong palette. The NX playground lets
the element load its own for this reason.

`setFolded(index, folded)` and `isFolded(index)` fold and read the node at `index` of `value.nodes`.

## Folding

Every record, property and sequence whose text spans more than one line gets a toggle to its left.
A folded node shows its first line and an ellipsis; either the toggle or the ellipsis unfolds it.
Everything starts unfolded, and copying always copies the whole text.

## Hover

Pointing at a part of the value, or moving focus to it with the arrow keys, shows a description of
the innermost node there. By default the description comes from the node alone, spelled as the NX
language service spells hovers:

```nx
(property) done: boolean
```

A host that knows more sets `describe`. It receives the node, its index and the value, and returns
markdown, or `undefined` for the default, synchronously or as a promise. The playground answers
with the language service's hover at the node's declaration, so hovering `User` in the output shows
what hovering its declaration in the source shows:

```ts
element.describe = async (node) =>
  node.declaration === undefined ? undefined : await hoverMarkdownAt(node.declaration);
```

The element renders the markdown hovers use: fenced `nx` blocks, highlighted, and paragraphs with
inline code.

## Going to a declaration

Clicking a node with a `declaration`, or pressing Enter on it, fires `nx-value-navigate`. Its
`detail` is `{ node, index, declaration }`, where `declaration` is the span in the program's source,
with 1-based lines and columns:

```ts
element.addEventListener("nx-value-navigate", (event) => {
  const { declaration } = event.detail;
  editor.revealLine(declaration.startLine);
});
```

## Styling

The element follows the page's `color-scheme`: token colors use `light-dark()`, so a page that sets
`color-scheme: dark` gets the dark theme. These custom properties restyle it:

| Property                             | Default                     |
| ------------------------------------ | --------------------------- |
| `--nx-value-font-family`             | the system monospace stack  |
| `--nx-value-font-size`               | `13px`                      |
| `--nx-value-line-height`             | `1.5`                       |
| `--nx-value-foreground`              | `CanvasText`                |
| `--nx-value-background`              | `transparent`               |
| `--nx-value-padding`                 | `8px 12px 8px 24px`         |
| `--nx-value-border`                  | a light or dark gray        |
| `--nx-value-hover-background`        | a light or dark blue tint   |
| `--nx-value-hover-popup-background`  | white, or a dark gray       |

## Development

```bash
pnpm install
pnpm --filter @nx-lang/value-view test
```

The tests run the element under jsdom with `node --test`.
