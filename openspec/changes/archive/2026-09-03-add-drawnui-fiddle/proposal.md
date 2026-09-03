# Add DrawnUI React fiddle sample app

## Why

NX has a component model, an IR format, and a JavaScript IR runtime, but nothing in the repository
demonstrates the three composed into a live, graphical, end-to-end experience. Everything visual
today is a proposal document: `docs/drawnui-proposal/` models a portable `nx.ui`/`nx.graphics`
catalog on paper, and `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` reasons about a renderer that
does not exist.

A fiddle — NX source in one pane, drawn output in the other — turns that into something runnable. It
proves the compile-to-IR-to-render pipeline works, exercises external components against a large
real-world catalog rather than a toy one, and surfaces concrete toolchain gaps that only appear at
scale. It is deliberately a proof of concept against the *existing* DrawnUI object model; the
portable `nx.ui`/`nx.graphics` catalog remains the long-term target, and this app is the renderer
groundwork it will later reuse.

## What Changes

- **New sample app** at `sample-apps/drawnui-react/`: a Vite React single-page app with a gallery
  of examples and a fiddle view — a source pane (Monaco, using the repository's existing NX TextMate
  grammar) beside a live canvas pane.
- **Example gallery** mirroring the DrawnUI.React demo site's top-level structure, where each entry
  draws its example and offers an affordance that opens the fiddle loaded with that example's NX.
- **NX ports of the demo pages**, each declaring its coverage as complete, static (drawn correctly
  but without the original's motion or interaction), or reduced (scaled down because NX cannot
  express the mechanism the original demonstrates). Every non-complete example attributes its gap to
  a named capability from a shared vocabulary, so the gallery doubles as a coverage report on NX
  itself rather than a list of disclaimers.
- **Vendored DrawnUI runtime**: `~/src/DrawnUi.React`'s `src/` tree is copied into the app, with a
  sync script that records the upstream commit. Local edits to the vendored copy are permitted where
  they improve NX compatibility.
- **Generated NX catalog** (`skia.nx`): NX external component declarations covering the full
  DrawnUI control set, generated from the vendored TypeScript sources via the TypeScript compiler
  API so that inherited and accessor-defined props are captured, and regenerable after a sync.
- **Compile service**: a small Node HTTP server that serves the built SPA and exposes one endpoint
  turning NX source into NX IR JSON plus diagnostics, backed by the existing `@nx-lang/sdk-node`
  native binding.
- **IR-to-DrawnUI renderer**: a client-side adapter that walks the canonical value tree produced by
  `@nx-lang/ir-runtime` and instantiates DrawnUI controls through its React reconciler.
- **Deployment**: a Dockerfile building the Rust native addon and the SPA into one Node service,
  suitable for Railway.

Not in scope: event handlers and interactive state. The TypeScript IR runtime has no action-dispatch
support (the Rust side does, via `dispatch_component_actions_*`), so authored NX renders statically.
DrawnUI's own built-in gestures — scrolling, ripple — still work.

## Capabilities

### New Capabilities

- `drawnui-nx-catalog`: How the NX external component catalog mirroring the DrawnUI control set is
  derived, what it includes and excludes, and how DrawnUI TypeScript types map onto NX types.
- `drawnui-fiddle`: The fiddle application itself — its compile pipeline, editing and diagnostic
  behavior, and the rules for translating an evaluated NX value tree into drawn output.

### Modified Capabilities

- `typescript-ir-runtime`: Three places where the TypeScript runtime rejects values the language
  produces, all blocking and none workaroundable from the app.

  Content bound to a list-typed content property must stay a list when a component has exactly one
  child. Today the runtime collapses single content to the child itself and then rejects it, so
  `<SkiaScroll><SkiaStack/></SkiaScroll>` — the opening shape of nearly every DrawnUI example —
  fails.

  A single value at a list-typed property must normalize to a list of one, which is what NX means
  by `Shadows={ <SkiaShadow Y=6.0 /> }` — the interpreter coerces it and the IR leaves the coercion
  to normalization. This is the general rule the content case above is one instance of.

  A record value must normalize into a record-typed field. Record construction stamps a `$type`
  discriminator on its result, and normalization then reports that discriminator as an unknown
  field, so `Padding={<Thickness Left=4.0 />}` fails — as does every record-valued default the
  catalog declares.

  The Rust interpreter is right in both cases; this aligns the two.

- `component-syntax`: A component body is type checked. Type checking walked functions and skipped
  components, so a component body was never inferred — which made a contextual literal there
  unresolvable at code generation (`unresolved contextual name cannot be emitted`), left a property
  type mismatch inside a body unreported until runtime, and left prop and state defaults unchecked.
  The unbraced-literal contract already requires a contextual literal to work "in every unbraced
  position where the expected type is already declared" and to be indistinguishable from the
  qualified form downstream, so this is a defect against a shipped requirement rather than a new
  one. Component bodies are now inferred, with props and state bound by name the way a function's
  parameters are.

- `record-type-inheritance`: A record field read has the field's declared type. Inference had an arm
  for a union and for a union case and none for a record, so `u.name` reported
  `Member access not yet implemented: .name` — in a function body as much as in a component body,
  though nothing hit it until component bodies started being checked. The requirement that
  inherited fields "participate in typed construction, field access, and default application"
  already covered this. Fields now resolve through the record's effective shape, each field's type
  resolved in the module that declared that field.

- `content-properties`: An element body may be empty. The `element` grammar rule required body
  content, so an open tag closed immediately by its own close tag — `<SkiaLayer></SkiaLayer>` — had
  no parse, at the top level and inside `let root()` alike. An empty body is the ordinary shape while
  a layout is being built up, and it is how a bare trailing element is usually first written, so it
  blocked the most concise NX the fiddle can accept. Body content is now optional, and an empty body
  means what a self-closing tag means. Supplying content is unchanged: a target that declares no
  content property still accepts an empty body and still rejects a populated one.

The change otherwise consumes `sdk-node`, `nx-ir-format`, `external-components`, and `editor-assets`
through their existing contracts without altering their requirements.

## Impact

- **New tree**: `sample-apps/drawnui-react/` — the first entry under `sample-apps/`, and a
  self-contained npm project, since the repository has no JavaScript workspace root.
- **Modified**: `runtime/typescript/src/index.ts` — content binding becomes aware of the content
  field's declared type, so a single child of a list-typed content property stays a list; record
  normalization checks the value's own `$type` against the declared type and then drops it, instead
  of reporting it as an unknown field; and a value whose `$type` names a record extending the
  expected one is normalized against its own schema and keeps its own discriminator, rather than
  being rejected as a foreign type; and a value offered as an abstract record itself — with no
  discriminator, or with one naming an abstract type — is rejected rather than stamped with a type
  name NX refuses to construct. Both are small and self-contained, but they are behavior
  changes to a shipped runtime, not sample-app code.
- **Consumed, unmodified**: `@nx-lang/sdk-node` (`bindings/node`) for compilation and
  `src/vscode/syntaxes/nx.tmLanguage.json` for editor highlighting. Both packages are `private: true`
  and consumed by relative path, and the native addon is gitignored, so the app's build depends on a
  local Rust toolchain.
- **Vendored demo content**: the DrawnUI demo pages are the source material for the gallery's
  examples, so the sync script copies them alongside the runtime.
- **External source**: `~/src/DrawnUi.React`, vendored rather than depended upon — it is a private,
  unbuilt package with no published artifact.
- **Modified**: `crates/nx-types/src/check.rs` and `crates/nx-types/src/infer.rs` — component
  bodies are inferred rather than skipped, the props bound while inferring one are the component's
  effective props so an inherited prop is checked like a declared one, and record field access has a
  type. All are compiler behavior changes rather than sample-app code, and all turn silent
  acceptance into diagnostics, so a program that relied on an unchecked component body may now
  report errors it always had.
- **Modified**: `crates/nx-hir/src/scope.rs` — a prop or state default is now visited by
  undefined-name checking, in the scope it will actually have: the fields materialized before it.
  A default naming a later field, or itself, was accepted by every static pass and then failed at
  run time with `Undefined variable`; it is a diagnostic now. A default naming an unknown name was
  not checked at all.
- **Modified**: `crates/nx-codegen/src/builder.rs` — generated IR carries the effective fields of a
  record, a record construction and a union case rather than only the declared ones, so a value
  built from IR carries what the interpreter's carries; a type reference resolves through type
  aliases, so a list spelled through one is still a list where the IR is read; and a name reaching
  neither a binding nor a declaration is reported instead of emitted as a slot no runtime can bind.
  All three are divergences between the interpreter and the IR runtimes, found by review rather than
  by the app.
- **Modified**: `crates/nx-codegen/src/model.rs`, `crates/nx-codegen/src/ir.rs` and
  `docs/nx-ir-format.md` — a record declaration and a union declaration carry `bases`, the abstract
  records they extend, nearest first, as declaration references, and a record declaration also
  carries `isAbstract`. Fields are already flattened into the IR, so these answer only what
  flattening cannot: whether a value stamped with one type's name is acceptable where another was
  asked for, and whether the type asked for is one that has values at all. Both are additions to the
  IR schema; nothing that was emitted before changes.
- **Modified**: `crates/nx-syntax/grammar.js` — an element's body content becomes optional, with
  `src/parser.c` and `src/grammar.json` regenerated by `tree-sitter generate` at the pinned CLI
  version. The generated diff is large and the authored one is a single word. This widens what
  parses and rejects nothing that parsed before, so no existing program changes meaning.
  `src/node-types.json` is unchanged, because a self-closing tag had already made `content` an
  optional field there.
- **Modified**: `crates/nx-syntax/src/scanner.c` — the external scanner decided two lookaheads by
  copying `TSLexer` and restoring it, which rewinds the lookahead character but not the position it
  came from. At end of input, where nothing is left to consume, the restored character came back
  forever and the parse never returned; `@` on its own was enough to hang the compiler, and with it
  the fiddle's single-threaded compile service. Both lookaheads now decide by consuming, and the
  parse trees they produce are unchanged.
- **Toolchain findings surfaced, not fixed**: imported external components lose defaults and
  inherited props at call sites (NXE12/NXE13), and the Rust interpreter and TypeScript IR runtime
  disagree on how a union case is serialized. The app works around both; each is a candidate for
  its own later change.
