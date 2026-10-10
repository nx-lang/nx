# @nx-lang/viewer

An `<nx-viewer>` custom element that shows NX source as a reading view: elements as cards headed by
their kind in words, values without their quotes, logic as sentences and handlers as events, for a
reader who does not write NX. It has no framework dependency and keeps its markup and styles in a
shadow root, so it drops into a plain page, a React app, a docs site or a VS Code webview alike.

What it shows is the source tree the NX language service answers for a document: `sourceTree(uri)`
on a language snapshot of [`@nx-lang/sdk-wasm`](../../bindings/wasm/README.md), or the same query
through any `NxLanguageService`. The reading is lossless: every node of the tree is shown, each
rendering carries its node's key, and every card and value opens to the exact source behind it.

## A plain page

```html
<nx-viewer id="reading"></nx-viewer>

<script type="module">
  import "@nx-lang/viewer";
  import { createNxHost, compileNxModule } from "@nx-lang/sdk-wasm";
  // With a bundler such as Vite; otherwise, wherever the page serves the package's `nx.wasm`.
  import nxModuleUrl from "@nx-lang/sdk-wasm/nx.wasm?url";

  const text = `type User = { id:string name:string? active:bool = true }
<User id="1" name="Ada" />`;
  const host = createNxHost(await compileNxModule(fetch(nxModuleUrl)));
  const snapshot = host.createLanguageSnapshot([{ uri: "nx:///user.nx", source: text }]);
  const viewer = document.getElementById("reading");
  try {
    viewer.text = text;
    viewer.tree = snapshot.sourceTree("nx:///user.nx");
  } finally {
    snapshot.dispose();
  }
</script>
```

The `User` card reads `id 1` and `name Ada`, then `active true` as a muted ghost row marked
"default": the default the tag leaves out, as the declaration spells it.

Importing the package defines the element. Where there is no DOM, as in a server render, importing
it does nothing, and `defineNxViewerElement(registry)` defines it in a registry of your choosing.

## React

React 19 passes properties to custom elements directly, so no wrapper is needed:

```tsx
import { useEffect, useRef } from "react";
import "@nx-lang/viewer";
import { NX_SELECT_EVENT, type NxSelectDetail, type NxViewerElement } from "@nx-lang/viewer";
import type { SourceTree } from "@nx-lang/language-protocol";

export function Reading(props: { tree: SourceTree; text: string; onSelect: (key: string) => void }) {
  const viewer = useRef<NxViewerElement>(null);
  useEffect(() => {
    const element = viewer.current!;
    const listener = (event: Event) => props.onSelect((event as CustomEvent<NxSelectDetail>).detail.key);
    element.addEventListener(NX_SELECT_EVENT, listener);
    return () => element.removeEventListener(NX_SELECT_EVENT, listener);
  }, [props.onSelect]);
  return <nx-viewer ref={viewer} text={props.text} tree={props.tree} />;
}
```

Set `text` before `tree`, as above: each renders, and the tree is read against the text. Declare the
element for JSX once in your app:

```ts
import type { NxViewerElement } from "@nx-lang/viewer";

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      "nx-viewer": React.DetailedHTMLProps<React.HTMLAttributes<NxViewerElement>, NxViewerElement> &
        Partial<Pick<NxViewerElement, "tree" | "text" | "stale" | "ghosts" | "selection">>;
    }
  }
}
```

## Properties and attributes

| Name        | Kind                   | Meaning                                                                      |
| ----------- | ---------------------- | ---------------------------------------------------------------------------- |
| `tree`      | property               | The `SourceTree` to show                                                     |
| `text`      | property               | The document's text, which a node's source is sliced from by byte offsets    |
| `ghosts`    | property               | Whether omitted defaults show as ghost rows; `true` unless set `false`       |
| `stale`     | property and attribute | The reading is muted and badged "Out of date"; selection and hover still work |
| `selection` | property               | The selected node's key; setting it opens what is folded around the node     |

`showSource(key)` shows a node's exact source beside its rendering, `hideSource()` hides it, and
`shownSource` is the key of the node shown.

## Selection

A click selects the smallest node under the pointer and fires `nx-select`, whose `detail` is
`{ key, role, range }`; `range` carries both editor positions and UTF-8 byte offsets, so a host can
put an editor's cursor at the node. Setting `selection` selects without firing the event, and a key
the tree does not hold clears it.

From the keyboard, the reading takes focus and the up and down arrows walk the selection through
every node in reading order; Enter selects a focused card or name, and Escape closes a hover or the
source panel. The selected node carries `aria-current`, and a hover is linked to what it describes
with `aria-describedby`.

`nodeAtSpan(tree, startByte, endByte)` finds the node whose range is exactly that span, preferring an
element. A value origin from the IR runtime carries such a span, so a host can select the source of
a value it shows.

## How it reads

- **Elements** read as cards headed by their kind in sentence case (`SingleChoice` reads "Single
  choice"), with properties in declaration order and the defaults the tag leaves out as ghost rows.
  A kind that names no declaration is shown as written and marked unresolved.
- **Values**: strings lose their quotes, booleans read ✓ and ✗, union cases read as pills, and text
  content whose type is markdown is rendered as markdown.
- **Logic**: `if` reads "Only when … / Otherwise", a `match` as a table of its arms, a `for` as "For
  each … in …", and operators as school math (`= ≠ < > ≤ ≥ + − × ÷`) or words where programmers have
  a convention (`and`, `or`, `not`, "otherwise" for `??`, "is given" for `x?`, "through" for `..=`).
  String concatenation reads as a sentence with its slots marked.
- **Handlers** read "When integer answered →" with one "set … to …" line per state change.
- **Declarations** read as headed sections (Type, Action, One of, Component, Function, Value) with
  their members, and doc comments as prose. Imports fold into a strip at the top. Comments read as
  notes where they stand.
- **Hover** on a property's name shows its type, doc and default, and every case of a union with the
  current one marked; hover on a handler shows the action it answers and what that carries; hover
  on a reference or a kind shows the declaration. All of it comes from the
  tree's declaration table, so it needs no language service.
- **References** to a value declared in the document expand in place to that value.

## Styling

The element follows the page's `color-scheme` through `light-dark()`. These custom properties
restyle it:

| Property                        | Default                   |
| ------------------------------- | ------------------------- |
| `--nx-viewer-font-family`       | the system UI stack       |
| `--nx-viewer-mono-font-family`  | the system monospace stack |
| `--nx-viewer-font-size`         | `14px`                    |
| `--nx-viewer-foreground`        | `CanvasText`              |
| `--nx-viewer-background`        | `transparent`             |
| `--nx-viewer-accent`            | a light or dark blue      |

## Development

```bash
pnpm install
pnpm --filter @nx-lang/viewer test
```

The tests run the element under jsdom with `node --test`, on source trees computed through
`@nx-lang/sdk-wasm` at test time from `test/fixtures/sources.json`. The coverage test renders every
`.nx` file in the repository and fails, naming the file and the key, for any node not rendered
exactly once.
