# Design: DrawnUI React fiddle

## Context

See `proposal.md` — Why. This section records only the constraints that shaped the approach, each
verified against the toolchain in this repository rather than assumed.

**The pipeline exists.** `nxlang codegen <file> --target nx-ir` emits `nx-ir-json` schema v2;
`@nx-lang/ir-runtime` (`runtime/typescript`) prepares that IR and evaluates a `root` entrypoint in
plain JavaScript; `@nx-lang/sdk-node` (`bindings/node`) exposes the same code generation in-process
through `NxProgramArtifact.buildSource(...).generateNxIr()` with structured diagnostics carrying
line and column spans. An external component evaluates to `{ "$type": "SkiaLabel", ... }` with
inherited props and defaults already resolved. No new compiler work is required.

**DrawnUI resolves controls by string tag.** `src/react/reconciler.ts` in `~/src/DrawnUi.React`
holds a `Registry` mapping tag name to control constructor, so `$type` maps to
`createElement($type, props, children)` directly. The renderer is a tree walk, not a per-control
switch.

**The demo pages are mostly declarative.** Of the eleven pages in `~/src/DrawnUi.React/samples/demo/pages`,
four (`Images`, `Layouts`, `Shapes`, `Svg`) use no React state, refs, effects, or handlers at all;
three more (`Text`, `Looks`, `Root`) use one to five. The remaining four (`Accessibility`, `Cells`,
`Snapping`, `Transforms`, `UnevenCells`) are interaction- or animation-driven. Every page, including
the fully declarative ones, defines local wrapper components — `Demo`, `Card`, `Tile` — that take a
title and children.

Seven constraints matter, and six are toolchain gaps that had to be found by trying:

1. **Cross-module external components are broken.** NXE12 and NXE13 in
   `docs/drawn-ui-proposal-nx-enhancements.md`: an imported external component loses its prop
   defaults and its inherited props at call sites. A plain workspace-sibling import
   (`import "./skia"`) did not resolve at all in testing. A single compilation unit works perfectly,
   including `extends` and defaults.

2. **NX declaration order does not matter.** A `root()` referencing components declared below it
   compiles and evaluates identically to the reverse order.

3. **NX IR code generation rejects a bare contextual union case as a prop default.**
   `Type: LayoutType = Absolute` evaluates fine under the interpreter but fails IR generation with
   `unresolved contextual name cannot be emitted`. The qualified form `= LayoutType.Absolute` is a
   syntax error in that position. `= {LayoutType.Absolute}` works. A bare case in *value* position —
   `<SkiaLabel HorizontalOptions=Center />` — generates and evaluates correctly; only defaults are
   affected.

4. **The two runtimes serialize union cases differently.** For the same program, the Rust
   interpreter emits `"Type": "Column"` while the TypeScript IR runtime emits
   `"Type": {"$type": "LayoutType.Column"}`. The fiddle renders through the TypeScript runtime, so
   it must handle the second form.

5. **The TypeScript IR runtime rejects a single child of a list-typed content property.**
   `applyContentBinding` binds content of length one to the child itself rather than to a
   one-element list; `normalizeValue` then fails with `Expected SkiaStack props.Children to be an
   array`. The Rust interpreter returns a one-element list for the same program. This is not an edge
   case — `<SkiaScroll><SkiaStack/></SkiaScroll>` opens nearly every DrawnUI page — so it blocks the
   app outright and has to be fixed rather than worked around. See the decision below.

7. **The TypeScript IR runtime rejects a record value it built itself.** `evalRecord` stamps
   `$type` on its result; `normalizeNominalValue` then passes that object to `normalizeFields`,
   which reports `$type` as an unknown field. The union branch three lines below already discards
   its discriminator before normalizing, so this is an inconsistency rather than a decision.
   `Padding={<Thickness Left=4.0 />}` fails, and so does every record-valued default in the catalog
   — `Padding`, `Margin`, `CornerRadius`, `Shadows`. Found while validating the catalog shape, not
   from reading; it blocks the app the same way constraint 5 does.

8. **A single value at a list-typed property is a list of one, and the TypeScript IR runtime does
   not know it.** `Shadows={ <SkiaShadow Y=6.0 /> }` and `xs={3.0}` both evaluate to one-element
   lists under the interpreter, which performs the coercion during normalization — the IR records
   the value at its own type. `normalizeValue` fails instead with `Expected ... to be an array`.
   This is the general rule that constraint 5 is one case of: the fix there made content binding
   respect the declared type, and this makes every list-typed field do the same. Found by drawing a
   shape with one shadow, which is how the DrawnUI examples write shadows.

9. **An element with an empty body was a syntax error.** `<SkiaLayer VerticalOptions=Fill>` closed
   immediately by `</SkiaLayer>` failed to parse — anywhere, not only at the top level, and equally
   inside `let root()`. The `element` rule required body content where `mixed_content` is `repeat1`,
   so an open tag followed directly by its close tag had no parse at all. An empty body is the
   natural shape while building a layout up, and it is what a bare trailing element is usually first
   written as, so this is fixed rather than worked around: body content is optional, and an empty
   body now means what a self-closing tag means. It stays distinct from *supplying* content — a
   target that declares no content property still accepts `<Plain></Plain>` and still rejects
   `<Plain><Kid /></Plain>`.

6. **The wrapper-component pattern ports, if its content property is required.** A user-defined NX
   component that takes `content Children: SkiaControl[]` and splices `{Children}` among literal
   siblings works, and the spliced list flattens into siblings correctly. Declaring the same property
   optional (`SkiaControl[]?`) fails with `expected SkiaControl[]?, found list object[]`. Since every
   demo page leans on this pattern, it decides whether the examples port at all — they do.

Two further shape details: a user-defined component used as a child of an external component inlines
into the tree correctly, so helper components cost nothing at the boundary; and NX has no spread, so
`<Tile {...transform} />` becomes explicit props.

## Goals / Non-Goals

**Goals:**

- Demonstrate NX source → NX IR → evaluated value tree → drawn canvas, live, in a browser.
- Present the DrawnUI example set as a gallery, each entry openable in the fiddle as NX.
- Exercise external components against a large real catalog, so gaps surface at scale.
- Keep the compile location behind one seam, so a future WASM build removes the server without
  touching editing or rendering.
- Leave a renderer the eventual portable `nx.ui` catalog can reuse.

**Non-Goals:**

- Fixing NXE12/NXE13, the IR union-default gap, or the union-serialization divergence. Each is
  worked around here and recorded for its own change. The exceptions are the gaps that cannot be
  worked around from the app: the list-content, single-value and record-normalization bugs in the
  TypeScript runtime, the unchecked component body and untyped record field in `nx-types`, and the
  empty element body in the grammar. Those this change fixes.
- Reproducing DrawnUI's interactive and virtualized examples. Where the original animates, responds
  to taps, or recycles cells, the NX port covers the declarative part and says what it omits.
- Reproducing fiddle.drawnui.net's product surface — accounts, sharing, saved fiddles, resource
  panels, MAUI export. The screenshot informs the two-pane layout, not the feature set.
- Fidelity to DrawnUI's exact object model. Editing the vendored copy to suit NX is allowed.
- Sharing, persistence, accounts, or multi-file authoring.
- Server-side rendering. CanvasKit is browser-only; the server compiles and serves, nothing more.

## Decisions

### Vite SPA plus a thin Node server, not Next.js

The client is a Vite React SPA; the server is a small Node HTTP service that serves the built
assets and answers `POST /api/compile`. Deployed as one Node service.

The reason is not framework preference. `~/src/DrawnUi.React` *is* a Vite project:
`src/core/Super.ts` does `import wasmUrl from "canvaskit-wasm/bin/canvaskit.wasm?url"`, and its
fonts come through Vite's `publicDir`. Under Vite the vendored source runs unmodified; under Next
that asset plumbing has to be rewritten and the native addon excluded from bundling. Chosen over
Next.js on that basis; the user confirmed the preference.

### Server-side compilation via the native Node SDK

`POST /api/compile` takes NX source and returns IR plus diagnostics, using `@nx-lang/sdk-node`
in-process.

Chosen over **client-side WASM**, which would remove the server entirely and allow a static
deployment — but no WASM target exists for the compiler today, and building one is a change of its
own. Chosen over **spawning the `nxlang` CLI**, which needs temp files and gives diagnostics as
parsed text rather than as data.

The client reaches compilation through one interface — source in, `{ ir, diagnostics }` out. Adding
a WASM build later replaces that one implementation; the editor and renderer do not change. This is
the seam the spec requires, and it is the main reason to name it explicitly rather than to `fetch`
from wherever a result is needed.

### One compilation unit: catalog first, visitor source last

The server concatenates the catalog with the visitor's source and compiles the result as a single
module. Concatenation rather than an import is forced by constraint 1 — an imported catalog loses
defaults and inherited props — and constraint 2 makes either order legal.

**The catalog goes before the visitor's source.** This reverses the original decision, which
appended it. Appending was chosen because it needs no position arithmetic: the visitor's text
occupies lines 1..N unchanged, so diagnostic lines and columns are already correct. That reasoning
was sound but incomplete — it missed constraint 9. A source file may end in a single bare element
instead of declaring `root`, and the grammar allows that element only as the file's **last** item.
Appending the catalog puts declarations after it, so the most concise NX a visitor can write —

```nx
<SkiaLayer VerticalOptions=Fill>
</SkiaLayer>
```

— stayed a syntax error in the fiddle even once constraint 9 was fixed and that file compiled on
disk. Leading with the catalog is the only arrangement under which both that form and a
concatenated catalog work.

The cost is the arithmetic the original decision was avoiding, and it is paid in one place. The
catalog contributes a known number of leading lines and bytes, and `classify` subtracts them when it
attributes a diagnostic to the visitor. Only lines and bytes shift; a column is relative to its own
line's start and the catalog contributes whole lines, so columns carry over untouched. Every
diagnostic reaches the client through that one function, so there is no second path to keep in step.

A diagnostic positioned before the visitor's offset falls inside the catalog. That is an application
fault, not an authoring error: it is surfaced as such and never marked in the editor.

A second cost is that catalog names are reserved — declaring `SkiaLabel` yields a
duplicate-declaration error. Acceptable for a fiddle, and it disappears when NXE12/NXE13 are fixed and the catalog can
become a real imported library.

### Catalog generated from DrawnUI's TypeScript, not hand-written

A generator script uses the TypeScript compiler API to resolve each registered tag's props and emits
`skia.nx` plus a companion metadata file. Both are committed.

Hand-writing is not viable at the requested breadth: roughly twenty controls over a base carrying
about fifty properties. It is also unreliable — `SkiaLabel` declares `Text`, `FontSize`, `TextColor`
and the rest as getter/setter pairs over private fields, so anything scraping field declarations
misses them entirely. The type checker sees through accessors and inheritance both.

`src/react/index.tsx` already defines `PropsOf<T>`: DrawnUI's own curated judgment about which
members are author-settable, excluding function-typed members and naming engine state explicitly.
The generator resolves that same type, so the catalog inherits a decision DrawnUI already made
rather than re-deriving it.

**Type mapping:**

| DrawnUI | NX | Renderer |
|---|---|---|
| `string`, `Color` | `string` | as-is |
| `number` | `float64` | as-is |
| `boolean` | `boolean` | as-is |
| string-literal union | NX union, cases spelled as the literals | unwrap to case name |
| `Thickness`, `CornerRadius`, `SkiaShadow`, `SkiaPoint` | record | construct the class |
| `SkiaGradient` | record | plain object |
| `GridLength` | `string` | as-is |
| function-typed | omitted | — |

Union cases keep DrawnUI's PascalCase spelling (`Start`, `Center`, `Fill`), verified to be legal NX
and to serialize to exactly the string DrawnUI expects. `CornerRadius | number` and
`string | GridLength[]` collapse to their single richer form; NX has no untagged unions and this is
a proof of concept. `GridLength` should eventually be a discriminated union; a string is enough now,
and the catalog says so.

**Every property is optional and the catalog declares no defaults.** DrawnUI's constructors already
establish them and the renderer drops nulls, so an unset property is left to the control.

Mirroring each default into the catalog was the first plan and is the worse one. Most DrawnUI
properties are accessor pairs over private fields, so a generator reading initializers recovers some
defaults and misses others silently — and a wrong mirrored default changes what is drawn with
nothing to catch it. It is also a second copy of a value that can drift from the vendored code
between syncs. Since external components are opaque to NX, nothing can read a default back, so
mirroring buys nothing an author can observe. It costs one thing: constraint 3 no longer applies to
the catalog at all, because there are no defaults to brace.

**Bases are split where a control is both a tag and a base.** Only abstract components may be
extended, so `SkiaLayout` is emitted twice: an abstract `SkiaLayoutBase` carrying the properties and
a concrete `SkiaLayout` extending it. This affects `SkiaLayout`, `SkiaLabel` and `SkiaShape`.
Content is typed against `DrawnNode`, a root invented for the catalog, because `TextSpan` is a legal
child of `SkiaLabel` without being a `SkiaControl` and no upstream type covers both.

### Renderer keyed by generated metadata

Alongside `skia.nx` the generator emits metadata naming which types are unions, which are records,
and which records need class construction. The renderer consults it rather than inferring from value
shape.

Inference is tempting: a union case's `$type` contains a dot (`"LayoutType.Column"`) and a record's
does not (`"Thickness"`), so the dot could discriminate. But the renderer needs the record-to-
constructor mapping regardless — `Thickness` becomes `new Thickness(...)` while `SkiaGradient` stays
a plain object — so the metadata has to exist anyway, and once it does, using it for both is one
mechanism instead of two. It also keeps the renderer honest when the catalog is regenerated.

The walk itself: read `$type`, look up the tag, coerce each property, normalize the content property
to an array (constraint on single children), drop nulls so DrawnUI's own defaults survive, recurse.
An unrecognized `$type` renders as a placeholder and is reported, rather than throwing away the
whole tree.

### Fix the two blocking bugs in the TypeScript IR runtime

`applyContentBinding` gains the content field's declared type and binds a list whenever that type is
a list, unwrapping `nullable` first. Both callers — `constructComponentDescriptor` and
`evalComponentDescriptor` — already have the field in hand, so the change is local.

Normalization treats a single value at a list-typed field as a list of one, which is what the
language means by it. That subsumes the content fix as a special case, and the two agree; the
content fix stays because binding is where the declared type is already in hand, and it is the path
`constructComponentDescriptor` takes when a host supplies content directly.

Record normalization drops the value's own `$type` before matching fields, exactly as the union
branch beside it already does. For a record the discriminator selects nothing — the declared type
supplies the field list — so ignoring it is not leniency, it is the only reading under which
construction and normalization agree.

These are the gaps the app cannot route around. Normalizing in the renderer was considered and
rejected for both: the runtime throws during evaluation, before any value reaches the renderer.
Reshaping the catalog was considered and rejected too — a container that takes exactly one child is
the ordinary case, not a catalog quirk, and `Padding` is on every control DrawnUI has. And the Rust
interpreter already does the right thing in both cases, so this is aligning the two runtimes rather
than inventing behavior. It does mean this change touches a shipped runtime, which is why
`typescript-ir-runtime` appears as a modified capability.

### Gallery of examples, each an NX file

The app opens on a gallery mirroring the DrawnUI demo site's top-level page list. Each entry draws
its example and carries an affordance opening the fiddle at that entry's own address
(`/fiddle/<example>`), so an example can be linked to directly.

Every entry is **NX source compiled through the app's own pipeline** — not the vendored TSX page
rendered natively. Rendering the original TSX beside NX ports would give a gallery that always looks
right while proving nothing, and would mean two rendering paths to keep working. Compiling the
examples means a gap in the catalog or the renderer shows up as a broken example, which is the
feedback this app exists to produce.

The examples are ports, and they are honest about it. Four pages (`Images`, `Layouts`, `Shapes`,
`Svg`) port fully. Three (`Text`, `Looks`, `Root`) port with minor interactive trimming. The
interaction- and animation-driven pages (`Accessibility`, `Snapping`, `Transforms`) port their
declarative substance — the transform tiles, the shape grids — and drop the animation and the
handlers. `Cells` and `UnevenCells` are built entirely on `ItemsSource`/`ItemTemplate` virtualization
over 100 000 rows, which has no NX expression at all; they port as a short fixed list.

### Coverage is three states, and the reason is a shared vocabulary

Most of these ports *work*. They draw correctly and completely; what is absent is motion and
interaction. Marking them all "partial" would present a correct port as a faulty one, and with seven
of eleven entries flagged the gallery would read as broken software. So coverage is three states, not
two: **complete** (no note), **static** (the drawing is right, the motion is gone), and **reduced**
(scaled down because NX cannot express the mechanism the original demonstrates).

The distinction earns its keep at the two ends. `Transforms` is pixel-correct and one logo does not
spin. `Cells` exists to virtualize 100 000 rows and does not. Collapsing both into "partial" discards
the more useful half of what the gallery knows. Visual weight follows: static gets a quiet neutral
chip, reduced a stronger one, and neither is styled as an error.

Every non-complete example attributes its gap to one or more capabilities from a **fixed shared
vocabulary** — `event-handlers`, `animation`, `component-state`, `list-virtualization` — and the
displayed wording derives from those rather than being written per example.

Chosen over per-example prose, which costs the same to write and yields less: prose cannot be
counted, drifts in wording across eleven cards, and cannot be searched. Tags make the gallery a
coverage report on NX itself — "four examples need event handlers" is a roadmap signal — and when a
capability lands, one grep names every example ready to be upgraded.

The gap is also recorded **in the NX source**, at the point the dropped behavior would have been
written:

```nx
// The original spins this logo on tap — NX has no animation or event handlers yet.
<SkiaSvg Source="drawnui.svg" WidthRequest=64.0 />
```

Someone who opens the fiddle wondering why tapping does nothing reads the code, not the badge above
it.

Two invariants fall out. **Every gallery entry is backed by working NX** — reduced rather than
placeholder, so no card lies and no fiddle affordance opens nothing; a page that cannot be reduced to
anything worth drawing is documented as omitted instead of shown as a dead tile. And **a static
example must rest in a sensible state**: `Transforms` frozen looks fine, but a carousel frozen
mid-scroll looks like a bug, so those ports are authored to sit at a deliberate resting position.

### Monaco with the repository's own grammar

The editor is Monaco with TextMate highlighting via `vscode-textmate` and `vscode-oniguruma`,
loading `src/vscode/syntaxes/nx.tmLanguage.json` directly from the repository. Vite resolves the
relative import, so no packaging step is needed and the grammar cannot drift from the extension's.

Diagnostics become Monaco markers. A full language service is out of scope — `nx-lsp` is a Rust
binary, and wiring it over a WebSocket is a change of its own.

### Vendoring DrawnUI with a sync script

`~/src/DrawnUi.React/src/` is copied to `sample-apps/drawnui-react/src/drawnui/`. A script re-copies
it and records the upstream commit. Upstream is `private: true` with no build output, so there is
nothing to depend on; and since local edits for NX compatibility are expected, the recorded commit
plus a documented list of divergences is what keeps the copy legible.

## Risks / Trade-offs

**Native addon must be built from source at deploy time** → `bindings/node/native/*.node` is
gitignored, so the Docker image builds it, which means a Rust toolchain in the build stage and a
slower first build. Layer caching makes rebuilds cheap. The alternative — committing a binary — is
worse.

**Catalog generation depends on DrawnUI's internal type shapes** → `PropsOf<T>` and its exclusion
list are DrawnUI's, and a refactor upstream could change what the generator resolves. Mitigated by
committing the generated output, so a sync that changes the catalog shows up as a reviewable diff
rather than as a silent behavior change.

**A large catalog may hit unknown NX limits** → roughly twenty components over a fifty-property base
is far more than any existing test exercises, and compile time or IR size could disappoint. This is
partly the point: finding those limits is a goal. Mitigated by generating the catalog early, before
the app is built around it, so a limit surfaces while the response is still cheap.

**Static-only rendering may read as a broken fiddle** → a visitor will try to make a button do
something. Mitigated by saying so in the app itself, not only in documentation, and by the in-source
notes at the point behavior was dropped. The gallery raises the stakes: a ported example is a second
place the same disappointment can land, which is why coverage is labelled per example and why the
labelling is proportionate — over-flagging correct ports would make working software look broken,
which is its own failure mode.

**Porting the examples is the largest and least predictable task** → eleven pages of dense JSX, and
each may hit an NX gap of its own beyond the ones already found. Mitigated by ordering the ports
from fully declarative to least, so the cheap ones land first and the catalog and renderer are
proven before the hard pages start; and by treating a page that cannot be ported as a finding to
record rather than a task to force.

**Changing `applyContentBinding` affects existing runtime consumers** → the fix alters a shipped
behavior, and something may depend on single content collapsing. Only for non-list content
properties, where behavior is unchanged; the list case currently throws, so nothing can depend on
it. The same argument covers the record fix: the path being changed throws today. The existing
runtime test suite is the check.

**Diagnostic positions now depend on offset arithmetic** → leading with the catalog means every
visitor-facing span is shifted before it reaches the editor, and a drift there misplaces markers
silently, which is what the original append ordering was chosen to avoid. Mitigated by keeping the
shift in the single function every diagnostic already passes through, and by a test that pins an
error to the exact line and column the visitor sees rather than merely asserting one is reported.

**Catalog names are reserved in visitor source** → a visitor declaring `SkiaLabel` gets a confusing
duplicate-declaration error pointing at a catalog they cannot see. Low likelihood; the diagnostic is
at least positioned on their own line.

**Compile requests execute visitor-supplied source** → NX evaluation on the server is a code path
driven by untrusted input. NX has no I/O and the server only generates IR rather than evaluating it,
so the exposure is resource consumption: a pathological program could be slow or large. Mitigated by
bounding request body size and rejecting oversized source; this is a demo, not a hardened service.

## Migration Plan

Almost entirely additive: a new tree under `sample-apps/`, plus contained fixes in
`runtime/typescript/src/index.ts`, `crates/nx-types`, and the `element` rule in
`crates/nx-syntax/grammar.js`. The runtime fixes land first and independently, since the app cannot
draw anything without them and their own capability spec covers them. The grammar fix carries a
regenerated `parser.c` and `grammar.json`, which are large but generated — they are reproduced by
`tree-sitter generate` at the pinned CLI version and reviewed through `grammar.js`, which changes by
one word. Nothing else to roll back beyond not deploying; the Railway service is new and independent
of anything already running.

## Open Questions

- Whether the sample app should be wired into CI. It depends on a Rust build and on a source tree
  outside the repository, which argues for leaving it out of the default build initially. Deferrable:
  it changes no spec, and the answer is clearer once build times are known.
- Whether the shared capability vocabulary needs a fifth tag once all eleven ports are done. The four
  named cover every gap found so far, and adding one later changes no spec — the vocabulary being
  fixed matters more than its exact size.
