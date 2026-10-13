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

The `User` card reads `id 1` and `name Ada`. Set `ghosts` and it also reads `active true` as a
muted ghost row marked "default": the default the tag leaves out, as the declaration spells it.

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
        Partial<Pick<NxViewerElement, "tree" | "text" | "stale" | "ghosts" | "images" | "selection">>;
    }
  }
}
```

## Properties and attributes

| Name        | Kind                   | Meaning                                                                      |
| ----------- | ---------------------- | ---------------------------------------------------------------------------- |
| `tree`      | property               | The `SourceTree` to show                                                     |
| `text`      | property               | The document's text, which a node's source is sliced from by byte offsets    |
| `ghosts`    | property               | Whether every omitted default shows as a ghost row; `false` unless set `true` |
| `images`    | property               | Whether a value formatted `image` loads its picture; `true` unless set `false` |
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
source panel. The selected node carries `aria-current`, a polite live region says what was selected,
and a hover is linked to what it describes with `aria-describedby`.

`nodeAtSpan(tree, startByte, endByte)` finds the node whose range is exactly that span, preferring an
element. A value origin from the IR runtime carries such a span, so a host can select the source of
a value it shows.

## How it reads

- **Elements** read as cards headed by their kind as it is written (`SingleChoice`), with
  properties in declaration order. A default the tag leaves out reads as a ghost row when the
  kind's viewer gives its property a label, a phrase or a place; the rest stay out unless `ghosts`
  is set.
  A kind that names no declaration is marked unresolved.
- **Properties**: an element's properties read beside its heading as `name=value` when there are
  at most four and they are short, as a tag's attributes do in the source, and as a row each
  otherwise.
- **Values**: strings lose their quotes, booleans read ✓ and ✗, union cases read as pills, a list
  on one line reads with commas, and text content whose type is markdown is rendered as markdown.
- **Logic**: `if` reads "Only when … / Otherwise", a `match` as a table of its arms, a `for` as "For
  each … in …", and operators as school math (`= ≠ < > ≤ ≥ + − × ÷`) or words where programmers have
  a convention (`and`, `or`, `not`, "otherwise" for `??`, "is given" for `x?`, "through" for `..=`).
  String concatenation reads as a sentence with its slots marked.
- **Handlers** read "When integer answered →" with one "set … to …" line per state change.
- **Declarations** read as cards (Type, Action, Component, Function, Value) with their members,
  and doc comments as prose at the top of the card they are written over. A union case with fields
  reads after a bar, with one or two fields beside it in braces and more on rows under it. What fits beside the name reads
  in the header, as in the source: an alias's type (`Type Swatch = string`), a union whose cases
  carry no fields (`Type Size = small | medium | large`), a function's return type after an arrow
  and a value's type after a colon. A member's default reads as it is written
  (`required boolean = true`). Groups are headed ("Takes", "Returns", "State", "Emits") only when
  a declaration has more than one. Imports fold into a strip at the top. Comments read as
  notes where they stand, and line comments standing alone on consecutive lines read as one note.
- **Hover** on a property's name shows its type, doc and default, and every case of a union with the
  current one marked; hover on a handler shows the action it answers and what that carries; hover
  on a reference or a kind shows the declaration. All of it comes from the
  tree's declaration table, so it needs no language service.
- **References** to a value declared in the document expand in place to that value.
- **Viewer declarations**, `viewer Choice = <ElementViewer … />`, read as a card headed "viewer
  Choice" over one line per element: its name and its attributes, with a `PropertyViewer` or
  `CaseViewer` led by what it is `of`.

## Viewer declarations

Ask the language service for the tree with `viewers`, and the declaration table carries each
declaration's resolved viewer, from the `viewer` declarations of the document's program and of any
host modules named:

```ts
viewer.tree = snapshot.sourceTree("nx:///survey/form.nx", { viewers: { modules: ["tenant/viewers.nx"] } });
```

With them, the reading follows what the viewer declarations say:

- A card is headed by the viewer's `name` for its kind. The properties bound to `title`,
  `subtitle`, `badge` and `image` show in its header and not again as rows.
- Rows read the `label` of their property, in the order the viewer names them, then the rest in
  declaration order. A boolean with `whenTrue` or `whenFalse` reads as that phrase in place of its
  row, and a property placed in `details` folds into a strip at the foot of the card.
- A `row` layout shows a card as one line, and an `inline` layout shows an element within the line
  that holds it, each with everything but the header folded into the details strip.
- A union's case reads as its `label`, with its name and `example` in the hover.
- A string value is shown by its format: `url`, `email` and `phone` as links, `image` as a picture
  with its URL, `color` with a swatch, `markdown` as prose. A `url` or `image` value is linked or
  loaded only when it is an `http:` or `https:` URL, and setting `images` to `false` shows a
  picture's URL as a link without loading it.

Nothing is hidden: a folded row is still in the reading, with its key, one click away.

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
