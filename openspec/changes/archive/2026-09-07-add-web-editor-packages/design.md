## Context

See `proposal.md — Why` for motivation. The facts that shape the approach:

- `nx-language-service` is already protocol-independent. `WorkspaceSnapshot::from_documents`
  accepts virtual URIs with no disk access, `hover` and `completions` share one position resolver,
  and every result type derives serde. The snapshot caches its whole-workspace analysis in a
  `OnceLock`, so repeated queries on one snapshot pay only position resolution. It hardcodes
  `ProgramBuildContext::empty()` in two places (`lib.rs:296`, `lib.rs:767`), and `ProgramBuildContext`
  is `Clone`.
- `LineIndex` (`lib.rs:2075`) converts an incoming character offset with `char_indices().nth()` and
  an outgoing one as `offset - line_start`: scalars in, bytes out. `nx-lsp` passes positions through
  untouched, and LSP's default encoding is UTF-16.
- The Node binding (`bindings/node/native`) exposes `NxWorkspace`, `NxLibraryRegistry`,
  `NxProgramBuildContext` and `NxProgramArtifact` as napi classes that hold an `Option<T>` and return
  JSON strings the TypeScript wrapper parses. It depends on `nx-api`, `nx-codegen`, `nx-value` — not
  on `nx-language-service`. Its JSON uses camelCase (`startLine`, `endColumn`).
- `@nx-lang/language` is the already-published editor-assets package (grammar, language
  configuration, snippets), staged by `src/vscode/scripts/package-language.mjs` and verified by a
  pack-and-import smoke test. The release pipeline attaches exactly one npm tarball per release.
- The fiddle (`sample-apps/drawnui-react`) compiles by concatenating the DrawnUI catalog ahead of the
  visitor's source, because an imported external component loses defaults and inherited properties
  (NXE12/NXE13). Its Monaco setup is ~150 lines: a `vscode-textmate` bridge, a custom theme, and a
  relative import of the grammar from `src/vscode`. A full compile against the 535-line catalog
  measures ~25 ms.
- ReachMe's config editor mounts Monaco through `@monaco-editor/react` inside Next.js, highlights
  through `@shikijs/monaco` with `github-light`/`github-dark`, links the grammar as
  `nx-language` from a git submodule, and validates on demand through a Hono API that builds an
  `NxLibraryRegistry` from directories and an `NxWorkspace` from database-backed modules. Its
  configuration is several files. It deploys as multiple instances.
- Neither `bindings/node` nor `runtime/typescript` nor the fiddle is built in CI; only the VS Code
  extension's pnpm project is. The fiddle's Dockerfile builds the three by relative path in order.

## Goals / Non-Goals

**Goals:**

- One Monaco integration, one client, one server handler, one wire contract — each consumable by
  the fiddle today and by ReachMe from the same source tomorrow, with no host-specific code inside
  any of them.
- The language service answers with libraries visible, which is the shape ReachMe's context takes,
  and answers in the units every editor uses.
- Every new package is publishable as-is: correct `exports`, types shipped, `files` bounded, and a
  local pack-and-import verification. The follow-up publication change touches only the pipeline.
- Nothing about hover content, completion content, or the conservative-answer contract changes.

**Non-Goals:**

- Shipping a React component. `@monaco-editor/react` is the React wrapper both hosts want, and the
  host-specific concerns (ReachMe's reveal requests and theme hook, the fiddle's gallery reload)
  stay in the hosts.
- Moving diagnostics off the compile path in the fiddle, or off the validate-draft path in ReachMe.
  The handler answers `diagnostics` so a host *can* mark live; when to ask stays a host policy.
- A worker or WASM implementation of `NxLanguageService`. The interface is the seam; nothing in this
  change presumes HTTP beyond the one implementation.

## Decisions

### D1 — The build context is a property of the snapshot, not of a query

`WorkspaceSnapshot` gains a `build_context: ProgramBuildContext` field, set by a builder method
(`with_build_context`) on the result of `from_documents`, defaulting to `empty()`. `diagnostic_report`
and `build_workspace_declarations` pass `&self.build_context`.

*Why:* the context determines what is visible, and visibility is a property of the whole analysis
the snapshot caches once. A per-query context would either invalidate the cache or silently answer
from the wrong one.

*Alternative considered:* a second constructor. Rejected — `from_documents` has two callers and a
builder keeps both unchanged.

*Verification concern, resolved first:* `analyze_workspace_modules` returns
`analyze_logical_module_graph(...).modules`. Whether library modules from the context appear in that
list, or only affect resolution of the workspace modules, is not evident from the call site. Task 1
measures it with a real library on disk before any binding code is written. If library artifacts
are absent, `build_workspace_declarations` also walks the context's library modules — the scope
lookup already keys declarations by module identity, so adding modules is additive.

### D2 — UTF-16 is converted in `LineIndex`, once, for everyone

`LineIndex::position_to_byte_offset` walks the line's `char_indices()` accumulating
`ch.len_utf16()` until the requested offset is reached; `byte_offset_to_position` computes the
UTF-16 length of `text[line_start..offset]`. `LineIndex::new` therefore keeps a reference to the
text, or the two methods take it — the latter matches the existing `position_to_byte_offset(text, …)`
signature. Serde types gain `#[serde(rename_all = "camelCase")]`.

*Why here:* every consumer — `nx-lsp`, the binding, and through it the HTTP handler and Monaco —
inherits the fix from one function. Converting in each client would leave `nx-lsp` wrong.

*Alternative considered:* negotiate `positionEncoding` in `nx-lsp` and offer UTF-8. Rejected — VS
Code negotiates UTF-16 anyway, Monaco is UTF-16 only, and a second encoding is a second thing to
test for no consumer.

*Consequence:* an offset landing inside a surrogate pair (between the two units of one emoji) is
clamped to the character's start. The spec's clamp-at-end rule already establishes clamping as the
policy for out-of-range input.

### D3 — The binding returns JSON strings and the wrapper types them; the wire shape is the serde shape

`NativeNxLanguageSnapshot` holds `Option<WorkspaceSnapshot>` and exposes `hover(uri, line,
character) -> Option<String>`, `completions(...) -> String`, `diagnostics() -> String`,
`documentSymbols(uri) -> String`, `dispose()`. Its constructor takes the document array and an
optional `&NativeNxProgramBuildContext`, cloning the context out of it (D1 makes that a cheap
`Arc`-style clone of the registry handle, which `ProgramBuildContext` already supports by deriving
`Clone`). The TypeScript `NxLanguageSnapshot` parses the JSON into types imported from
`@nx-lang/language-protocol`.

*Why JSON strings:* it is the binding's established pattern (`validate` returns a JSON string) and
avoids napi object marshalling for nested result types.

*Why the serde shape is the protocol:* one definition. The protocol package's answer types are
written to match what `Hover`, `CompletionList`, `DocumentDiagnostics` and `DocumentSymbol`
serialize to, camelCased. A parity test in the binding (task 3) serializes each result kind from a
real snapshot and asserts the exact key set the protocol types declare, so drift fails a test rather
than a consumer.

*Alternative considered:* generate the TypeScript from Rust with `ts-rs`. Better guarantee, but a
new proc-macro dependency on the service crate, a generation step in the build, and generated files
to commit or ignore. The parity test buys most of the guarantee for a handful of assertions; revisit
if the shape grows past four result kinds.

### D4 — Every query carries its documents; the server is stateless with a content-keyed cache

The request body is `{ documents: [{ uri, source, version? }], uri, position? }` for every query.
The handler hashes the concatenation of `(uri, source)` pairs plus the prelude identity, looks the
hash up in a small LRU (default 8) of `NxLanguageSnapshot`s, and constructs one on a miss.

*Why stateless:* ReachMe runs several API instances behind a load balancer; a session model would
need affinity or shared state for what is, at ~25 ms per analysis, a cheap recomputation. It also
matches the fiddle's existing compile seam, so a WASM implementation later is a drop-in.

*Why the cache anyway:* a hover storm sends the same text many times per second. The snapshot's
`OnceLock` makes each subsequent query on a cached snapshot sub-millisecond; without the cache the
single-threaded Node server would analyze on every mouse move.

*Cost accepted:* the full source travels with each query. The fiddle's sources are a few hundred
lines; ReachMe's configurations likewise. The body limit (default 1 MiB) bounds it.

*Alternative considered:* `monaco-languageclient` over a WebSocket to `nx-lsp`. Rejected — stateful
per connection, brittle Monaco version coupling, a large VS Code API shim in the bundle, and the
prelude would still need a home.

### D5 — The handler is Fetch-shaped, and the prelude lives inside it

`createNxLanguageHandler(options)` returns `(request: Request) => Promise<Response>`. Options:
`buildContext?: NxProgramBuildContext`, `prelude?: { source: string }`, `maxBodyBytes?`,
`cacheSize?`, `onError?`. `toNodeListener(handler)` adapts to `(req, res)` by reading the body into
a `Request` and streaming the `Response` back.

*Why Fetch-shaped:* Hono's `app.all('/api/language/*', c => handler(c.req.raw))` is one line, Node
22 has the types natively, and the fiddle's plain server uses the adapter.

*Why the prelude is a handler option:* the shift arithmetic already exists in
`server/compile.mjs` and has three subtle rules (a trailing newline is added if missing, columns
never move, an empty span at the very first byte is the program sentinel). Putting it in the handler
means one implementation with tests, shared by `compile.mjs` through an exported helper, and one
place to delete when NXE12/NXE13 are fixed. The prelude applies to the document named by the
query's `uri`; other documents in the set are passed through untouched.

*Prelude and the cache:* the prelude source is part of the cache key implicitly — it is fixed per
handler instance, and the cache is per instance.

### D6 — One Monaco package, Shiki for highlighting, no React

`registerNxLanguage(monaco, { service?, workspace?, themes?, onError? })` returns an
`IDisposable`. It registers the language and configuration from `@nx-lang/language`, builds a Shiki
highlighter with the grammar and the requested themes (default `github-light`, `github-dark`),
applies `shikiToMonaco`, and — when `service` is given — registers hover and completion providers.
A module-level `WeakMap<MonacoNamespace, Registration>` makes a second call return the existing
registration, which is what `@monaco-editor/react`'s `beforeMount`/`onMount` double call needs.
`monaco-editor`, `shiki` and `@shikijs/monaco` are peer dependencies so the host controls versions
and there is one Monaco on the page.

*Amended during implementation — `@nx-lang/language` is a peer dependency too.* The release
pipeline that publishes `@nx-lang/language` exists, but no release has run: the package is not on
the npm registry (`npm view @nx-lang/language` → 404), so the Monaco package cannot depend on it by
version today. It is therefore declared as a peer dependency alongside Monaco and Shiki, and each
host supplies it: from the registry once a release has published it, or until then by linking the
extension directory, which already carries the same `exports` the staged package does. This
repository's workspace does the latter — `"@nx-lang/language": "file:../../src/vscode"` in the Monaco
package's dev dependencies and the fiddle's dependencies (pnpm accepts the alias; the link is a
snapshot copy, so a grammar edit needs `pnpm install` to reach the Monaco package). ReachMe's
existing `nx-language` link becomes the same alias when it migrates. The fold of `src/vscode` into
the workspace (D7's follow-up) replaces the link with `workspace:*`.

*Why Shiki over the fiddle's `vscode-textmate` bridge:* ReachMe already pays for it, the Monaco
bridge is maintained by Shiki's authors rather than by us, themes come with it, and the same
highlighter renders hover code blocks. The fiddle's custom `nx-dark` theme is dropped for
`github-dark`.

*Why no React component:* both hosts already compose Monaco through React in host-specific ways;
a shared component would either be a thin pass-through or a third opinion about theme and layout.

*Position conversion:* Monaco's `column` is one-based UTF-16; the package subtracts one and passes
it through, and adds one on the way back. Model text is read with `model.getValue()` at request
time. The default workspace callback returns `[{ uri: model.uri.toString(), source, version:
model.getVersionId() }]`; Monaco's `inmemory://` URIs parse as URLs, so the service accepts them as
logical identities.

*Trigger characters:* `<` and `:`, matching `nx-lsp`'s advertisement. Adding `=` for property
values would help both hosts but belongs to a change that adds it to the LSP as well.

### D7 — A root pnpm workspace, with `src/vscode` left as a nested workspace for now

A root `pnpm-workspace.yaml` lists `packages/*`, `runtime/typescript`, `bindings/node`, and
`sample-apps/drawnui-react`; a root `package.json` pins `packageManager` to the pnpm version
`src/vscode` already uses; and a root `pnpm-lock.yaml` replaces the three per-directory
`package-lock.json` files. Packages depend on each other with the `workspace:*` protocol, which pnpm
links locally and rewrites to the real version at publish time. `src/vscode` keeps its own
`pnpm-workspace.yaml`; pnpm resolves the nearest workspace file, so commands run there behave exactly
as today and the root workspace does not see it. The Monaco package depends on the published
`@nx-lang/language` by version; this change needs nothing unreleased in the grammar.

*Why a workspace at all:* today's `file:` links are symlinks that neither install nor build their
target, so contributors build the IR runtime, the binding, and the fiddle by hand in order. This
change would make that seven steps. One install links every member, and `pnpm -r build` orders the
builds by the dependency graph.

*Why pnpm rather than npm:* the repository's existing three JavaScript packages happen to use npm,
but the stronger precedents are pnpm — the extension, all four CI workflows, and ReachMe, which is a
pnpm workspace that already consumes this repository's packages as workspace entries. Beyond
consistency, pnpm has three properties that matter for packages meant to be published:
`workspace:*` makes intra-workspace dependencies explicit and publish-safe, where npm links by
version-range coincidence and has no publish-time rewrite; strict `node_modules` fails an undeclared
dependency locally instead of at the first registry consumer; and recursive commands are
topological. pnpm 10+ blocks dependency build scripts by default, which is fine because the
binding's native build is an explicit script, not a postinstall.

*Alternative considered:* fold `src/vscode` in now. Deferred rather than rejected — the right end
state is one workspace where `@nx-lang/language` is a real package directory under `packages/`
holding the grammar, language configuration and snippets, the extension is a member that depends on
it and copies the assets into its tree at VSIX packaging time (grammar contributions must be file
paths inside the VSIX), the `package-language.mjs` staging script is deleted, and the Monaco package
depends on it as `workspace:*`. That fold moves the lockfile and working directory that four CI
workflows reference and must verify `vsce` packaging under a root workspace, which has a history of
friction with pnpm's symlinked dependencies (the extension already bundles, which is the standard
remedy). It is recorded as a follow-up so it lands as a join, not a conversion.

*Interim ReachMe consumption:* ReachMe adds the four new directories to its `pnpm-workspace.yaml`
alongside the existing two. Because the packages already use `workspace:*` between themselves, no
overrides are needed. That is a ReachMe change and is recorded, not performed, here.

### D8 — Release-ready packages, no release

Each new package: `"private"` absent, `publishConfig.access: public`, `exports` with `types`
conditions, `files: ["dist", "README.md"]`, `prepack: pnpm run build`, and a `verify:package` script that
runs the repository's one `scripts/verify-package.mjs` against the package: it packs to a temp dir
with `pnpm pack` (which rewrites `workspace:*` to concrete versions), installs the tarball into a
scratch project, and imports each export — the same proof `@nx-lang/language` already requires. Versions stay at `0.1.0` in source;
the pipeline stamps them at release, as it does for `@nx-lang/language`.

*Why stop here:* production publication needs the release manifest, the one-tarball assertion in
`package-publish.yml`, trusted-publishing registration for each new name, and — for `sdk-node` — a
napi prebuild matrix. That is a pipeline change with its own review; nothing in it changes how the
packages are shaped.

### D9 — Added during implementation: the language service flattens `extends`

Task 7.3's route test was the first query against the real catalog, and it showed that hover on
`<SkiaLabel>` reported one property and completions inside its tag offered one. Every DrawnUI
control is declared as `external component <X extends Base ... />`, and the language service built
a declaration's property list from the item's own props alone: the compiler resolves the base chain
when it checks a tag, but nothing that hover or completion read did. The `drawnui-fiddle` scenarios
("its properties and their types", "properties not yet supplied") are unmeetable without this, and
ReachMe's built-ins are declared the same way.

`WorkspaceDeclarations` now runs a pass after indexing every module — workspace and library — that
gives each component or record declaration with a base the base's properties after its own,
nearest base first, own names shadowing inherited ones, cycles ending the chain. The base name is
resolved in the declaring module's own bindings, and each property remembers the module that
declared it so a member completion on an inherited property resolves the property's type where it
was written rather than in the module of the tag. Hover on such a component spells the declaration
as written and then lists what it inherits, with the bases it comes from.

*Alternative considered:* calling `nx-hir`'s `effective_component_contract_at`. It needs a
`PreparedModule`, which the analysis artifacts the service keeps do not retain; threading one
through would change what every snapshot holds for a lookup the declaration index can answer from
what it already has.

## Risks / Trade-offs

- [Library modules may not appear in `analyze_workspace_modules` output] → Task 1.1 measures it
  before anything depends on it; the fallback (walk the context's libraries in
  `build_workspace_declarations`) is additive and confined to the service crate.
- [UTF-16 changes existing behavior for non-ASCII documents] → It changes it from wrong to right,
  and `nx-lsp`'s existing tests are ASCII, so they pass unchanged; a non-BMP test is added in both
  crates.
- [Monaco's `inmemory://` model URI as a logical identity] → `identity_from_uri` derives an identity
  from host and path segments; `inmemory://model/1` yields `model/1`, which is valid. The fiddle sets
  an explicit `nx://fiddle/fiddle.nx` URI anyway so the prelude has a stable name.
- [Peer dependency version drift between hosts] → The package declares ranges covering Monaco 0.54
  and 0.55 and Shiki 3 and 4; the fiddle pins one end and ReachMe the other, so both are exercised.
  *Amended during implementation:* the ranges are `monaco-editor >=0.56.0`, `shiki ^4.0.0` and
  `@shikijs/monaco ^4.0.0`, the versions the package's tests and the fiddle run against. The fiddle
  moved from Monaco 0.54 to 0.56 for its worker import: 0.56's exports map resolves
  `monaco-editor/editor/editor.worker.js`. The package itself reaches Monaco through types only, so
  the floor records what was exercised rather than a feature it needs, and only one end is exercised
  today. ReachMe's versions are not confirmed; its migration (`specs/future.md`) starts by checking
  them, and widens the range after testing there if it is on an older Monaco.
- [Single-threaded server blocked by a slow analysis] → The cache makes steady-state queries
  sub-millisecond; the body limit bounds input; the fiddle README's existing note about the compile
  route applies to the language route and is extended to say so.
- [Root workspace changes the Dockerfile and the contributor bootstrap] → The Dockerfile copies the
  root manifests, enables pnpm through corepack, and runs one `pnpm install --frozen-lockfile`;
  the fiddle and binding READMEs are updated; a build of the image is a task.
- [pnpm's strict `node_modules` may expose undeclared dependencies in the existing three packages]
  → That is the property being bought; each such failure is fixed by declaring the dependency, and
  the conversion task verifies every package builds and tests under the root workspace.
- [Prelude concatenation is a workaround that now has an API surface] → It is documented as such,
  scoped to one option, and its tests double as the acceptance tests for deleting it after
  NXE12/NXE13.

## Migration Plan

1. Rust first: build context and UTF-16 land with tests; `nx-lsp` picks them up with no change.
2. Binding class and protocol package; binding tests prove parity.
3. Handler and client packages with their tests.
4. Monaco package; the fiddle switches to it and to the handler in one step, deleting its bridge.
5. Root workspace, Dockerfile, READMEs; image build verified.
6. Recorded follow-ups: `publish-web-editor-packages` (pipeline, prebuilds), folding `src/vscode`
   into the root workspace with `@nx-lang/language` extracted to `packages/language`, ReachMe
   migration, NXE12/NXE13 fix removing the prelude.

Rollback within the repo is a revert; nothing is published and no external consumer moves until the
follow-ups.

## Open Questions

- Which Shiki themes the fiddle should default to beyond `github-dark` is cosmetic and can be
  changed after the switch without affecting any spec.
