## Context

See proposal.md — Why. What this design builds on, in the two repositories:

**The fiddle engine** (`~/src/DrawnUi.FiddleEngine`, a Blazor WebAssembly class library plus a
minimal host) is language-agnostic by rule: a language is an `IFiddleLanguage`, and the page only
branches on its surface. Its React surface is DrawnUi.React on CanvasKit. A React-surface language is
a JS registration `{ id, monaco, compile, configure? }` made through `fiddleReactRegisterLanguage`
after `fiddle-react.js` loads, plus a C# descriptor deriving from `ReactSurfaceLanguage` (abstract
identity and presets, virtual `DiagnosticsAsync`). `compile(code)` returns `{ js, errors, warnings }`
where `js` is an ES module whose default export is a React component or element; the engine
evaluates it from a blob URL with every DrawnUi.React name, React, its hooks, `Core`, `console` and
`Fiddle` in scope, mounts it in a `Canvas`, and stores the same module as the share artifact so the
`/p/` player runs it with no compiler. The engine boots the DrawnUi.React runtime lazily so a C#
visitor never fetches CanvasKit. Monaco comes from BlazorMonaco 3.3.0, which is Monaco 0.52.2 through
the AMD loader as the `monaco` global; the editor uses one shared model at an `editor.tsx` URI and
retags it on language switch after calling each registered language's `configure()`. Color swatches
are a hardcoded per-Monaco-language regex table in `fiddle-intellisense.js`. `npm run runtime`
bundles the pinned `drawnui-react` (0.1.0-preview.9) with esbuild into one IIFE that publishes
`window.React`, `window.DrawnUi` and `window.DrawnUiCore`, plus `canvaskit.wasm`, fonts and a `.d.ts`
harvest, into a host's `wwwroot/react/`; the output is committed.

**The NX repository** has the whole pipeline the fiddle needs, in the playground:
`@nx-lang/sdk-wasm` (a `wasm32-wasip1` module of 1.9 MB, 680 KB gzipped, JSON in and out over a
hand-written loader with a browser WASI shim; private, unpublished), `@nx-lang/ir-runtime`
(private), `@nx-lang/monaco` (peer `monaco-editor >= 0.56`, takes a namespace), `@nx-lang/language`
(the published editor assets), and in `sites/playground/src`: `compile/catalog.ts` (prelude build
with diagnostic origin classification), `render/values.ts` (union and record coercion driven by a
generated `catalog-meta.json`, constructing DrawnUI value types), `render/DrawnTree.tsx` (a walk that
emits `createElement(type, props)` with string tags the DrawnUi.React reconciler resolves), and
`scripts/generate-catalog.mjs` (the TypeScript compiler API over the vendored DrawnUI sources,
reading the tag list from the reconciler's `Registry` literal). The playground vendors DrawnUi.React
0.1.0-preview.4 with no local edits. IR is pretty-printed by `nx-codegen`, and every program's IR
carries the flattened catalog: the largest example is 1.4 MB. The TypeScript runtime has no action
dispatch, so authored handlers do not run anywhere yet.

**Direction the user has set**: the NX repository keeps no concept of DrawnUI. The DrawnUI catalog,
its generator, the coercion and the renderer are rebuilt in the fiddle repository from the
DrawnUi.React package the fiddle ships. The playground keeps its DrawnUI target for now and drops it
in a later change.

## Goals / Non-Goals

**Goals:**
- One mount path for NX in the fiddle, used by the editor after every compile and by the player from
  the share artifact, so that action dispatch arriving in the IR runtime changes the runtime and
  nothing in the fiddle integration or the share format.
- Nothing DrawnUI-specific enters the NX repository, and what leaves the playground for a package is
  only language infrastructure (the prelude build) that the playground itself keeps using.
- A C# or TSX visitor never fetches the NX module or bundle; an NX visitor fetches each once.
- The fiddle's build is reproducible from pinned npm packages, with the same shape as its
  DrawnUi.React runtime build, and a stale catalog cannot ship silently.
- The two engine extensions are language-agnostic and leave TSX untouched.

**Non-Goals:**
- A Web Worker for the fiddle's compile. Compiles take on the order of 100 ms and a trap is
  recovered by rebuilding the host in under 2 ms; the worker's deadline and isolation are a
  follow-up, with the playground's worker as the model.
- Shipping the catalog's IR with the runtime rather than in each share. The share carries the whole
  program until the catalog can be a library artifact; compact JSON is the mitigation now.
- Exports, Premium publishing, the compiled-artifact cache, and drawfiddle.com's community labels.
- Changing the backend contract. An NX share is an ordinary share with `lang: "nx"` and a `js`
  artifact; the id is the backend's hash of `lang` plus code.
- Reworking the playground's own compile or render paths beyond consuming the moved helper.

## Decisions

### D1. NX is a React-surface language, mounted through one runtime entry point

NX registers `{ id: 'nx', monaco: 'nx', prepare, configure, compile, colors }`. `compile(code)`
returns a module of the form

```js
const IR = /* compact NX IR JSON */;
export default () => NxDrawn.mount(IR);
```

`window.NxDrawn` is set by the NX runtime bundle. `mount(ir)` returns a React element whose
component prepares the program once (memoized on the IR object), evaluates `root()`, and renders the
value tree through the walk. The editor and the player therefore run identical code; the only
difference is where `IR` came from.

*Alternatives.* A .NET-surface language needs a `SkiaControl` built in C#; NX has no .NET IR
evaluator or DrawnUI renderer, and the .NET SDK is a native cdylib for desktop targets, so this is a
from-scratch renderer plus an Emscripten port. Embedding the evaluated value tree in the artifact
instead of the IR would make shares small and the player runtime-free, but it freezes the snippet:
with action dispatch expected soon, it would be replaced immediately, and it needs a flattening
step (expanding authored components) that would then be deleted. The IR is the program.

### D2. Two scripts: a small eager shim and a lazy runtime bundle

`src/DrawnUi.Fiddle/wwwroot/fiddle-nx.js` is hand-written like `fiddle-react.js`, a few dozen lines,
loaded by the host's `index.html` at the marked spot after `fiddle-react.js` on every page load. It
registers the language and defines `prepare()`, which loads `<host>/nx/nx-runtime.js` once (a
script that sets `window.NxDrawn`) and then has the runtime fetch and compile `<host>/nx/nx.wasm`
once. `compile`, `configure`, `diagnostics` and `mount` delegate to the runtime. The bundle carries
`@nx-lang/sdk-wasm` (browser entry and WASI shim), `@nx-lang/ir-runtime`, `@nx-lang/monaco` with
Shiki and the `@nx-lang/language` grammar, the catalog text and metadata, the coercion and the walk.

*Why the split.* The shim must exist before Blazor starts (the player path runs before Blazor and
must be able to prepare NX), but Shiki, the IR runtime and the SDK loader are hundreds of kilobytes
that a C# visitor must not pay for, and the module is 680 KB gzipped. The React runtime uses the
same shape: a small registry in the engine, a large bundle in the host fetched on first use.

### D3. The engine gains a `prepare` hook and registration-supplied color patterns

`fiddleReactRun` awaits `language.prepare?.()` before `compile`; `fiddleReactPlay` reads the share's
language from `X-Fiddle-Lang`, looks up the registration, and awaits its `prepare` before evaluating
the stored module. A rejected `prepare` is a failed run with the rejection's message and is retried
on the next run. `fiddle-intellisense.js` builds its color-swatch provider from
`reactLanguage(id).colors` (a regex over string literals) for registered languages, falling back to
the existing C# and TypeScript patterns. Both are documented in `docs/ADDING-A-LANGUAGE.md`.

*Alternative.* Having the artifact module itself load the runtime (a component that renders empty,
loads, then re-renders) keeps the engine untouched but breaks the player's `ready` message and
thumbnails, which fire after the first drawn frame. `Fiddle.HoldReady()` exists for snippets, not for
languages, and using it would put language plumbing into every artifact. A language-level hook is
the engine's own pattern (`configure` is one already).

### D4. The catalog is generated in the fiddle from the package's declarations

`nx/generate-catalog.mjs` in the fiddle repository ports the playground generator's algorithm and
retargets its inputs: the tag list is the set of React components exported by
`drawnui-react/dist/react/index.d.ts` (the `Registry` literal the playground reads is a runtime value
absent from declarations), and each tag's props are resolved through the TypeScript checker over the
package's `.d.ts` with the same `PropsOf` probe. It writes `nx/catalog/drawnui.nx` and
`nx/catalog/catalog-meta.json` (unions, records with their constructing class, components with
their content property, the node root), plus a `version` field naming the `drawnui-react` version.
`dev/build-nx-runtime.mjs` refuses to build when that version differs from `package.json`'s pin and
names the regeneration command. The coercion (`nx/src/values.ts`) constructs `Thickness`,
`CornerRadius`, `SkiaPoint`, `SkiaShadow` and `SkiaBevel` from `window.DrawnUi`, never from an import,
so there is one engine instance and DrawnUI's `instanceof` checks pass.

*Alternatives.* Reusing the playground's catalog would tie the fiddle to preview.4 while it draws
with preview.9, and would keep DrawnUI in the NX repository. Reading the `.d.ts` harvest the fiddle
already makes (`types.json`) instead of `node_modules` would couple the generator to the React
runtime build's intermediate output for no gain.

### D5. The prelude build moves into the wasm SDK; the walk stays in the fiddle

`buildProgramWithPrelude(host, prelude, source, { fileName })` in `@nx-lang/sdk-wasm` is
`compileWithCatalog` from `sites/playground/src/compile/catalog.ts` with its DrawnUI vocabulary
removed: it returns `{ ir: NxGeneratedNxIr | null, diagnostics }` with `source` / `catalog` /
`program` origins and document-shifted spans, using the prelude arithmetic already in
`@nx-lang/language-core`. The playground's `catalog.ts` becomes a re-export. The value walk and the
coercion are copied into the fiddle (about 150 lines) rather than packaged: they are half DrawnUI
(placeholder controls, value-type constructors, `Children`) and half React, and the playground's
copy is scheduled to leave with the rest of its DrawnUI target.

*Alternative.* A generic `@nx-lang/react-host` package holding the walk, parameterized by
constructors and a placeholder, would remove the duplication but creates an NX package whose only
consumer shape is DrawnUI's, before the general-purpose playground exists to justify it.

### D6. Compact IR from codegen, pretty only in the CLI

`emit_nx_ir` in `nx-codegen` gains a formatting choice; `nx-ffi`, the wasm crate and therefore both
SDKs emit compact JSON, and `nx-cli`'s `nx-ir` target keeps writing pretty-printed files. Content is
unchanged, so the IR runtime, the .NET binding and the playground are unaffected; tests that compare
IR text are updated to compare parsed values.

*Alternative.* Compacting on the JavaScript side (`JSON.stringify(JSON.parse(json))`) works but
moves the largest string in the pipeline through the worker twice and leaves the .NET SDK
pretty-printed. The emitter is the one place.

### D7. Monaco: register against the host's namespace, admit 0.52

`registerNxLanguage(monaco, options)` already takes a namespace object; the change is to make the
package's runtime imports type-only so the bundle never pulls `monaco-editor`, to lower the peer
range to `>= 0.52.0` (every API the package calls exists there), and to state in the README that an
AMD-loaded global is a valid namespace. In the fiddle, `configure()` calls it with `window.monaco`,
a `workspace` that presents the shared model under an `editor.nx` URI, and a language service built
from the SDK's snapshot service over the catalog prelude. The fiddle's theme is not a Shiki theme, so
NX tokens take the integration's first loaded theme's colors under the fiddle's theme, which the
package already handles.

### D8. Diagnostics go through the C# override; hover and completion through Monaco

`NxLanguage.DiagnosticsAsync` calls `fiddleNxDiagnostics(code)`, which asks the language service for
the document's diagnostics over the prelude and returns them in the engine's `FiddleDiagnostic` shape
(message, severity, 1-based start and end). This feeds the engine's own debounce loop, marker
painting and HotReload gate, so NX behaves like C# there. Hover and completion are Monaco providers
registered by `configure()`, as for TSX.

### D9. Presets are ported playground examples that fit the fiddle

At least four presets, ported from playground examples marked complete or static, with the hello
starter first: they must compile against the preview.9 catalog, use fonts the fiddle host serves
(`FontText` is configured by the engine's React boot) and images by absolute URL, and avoid
playground-only assets (Lottie, sprite sheets, shaders under the playground's `public/`). Presets are
compiled by a test in the fiddle's runtime build so a catalog change that breaks one is caught.

### D10. The fiddle consumes published packages; development links a checkout

`package.json` in the fiddle pins `@nx-lang/sdk-wasm`, `@nx-lang/ir-runtime`, `@nx-lang/monaco`,
`@nx-lang/language`, `shiki`, `@shikijs/monaco` and `typescript` (for the generator) as
devDependencies, mirroring how `drawnui-react` is consumed. Until the NX packages are on the
registry, and for iterating on both repositories, `npm run runtime -- --nx ../nx` resolves the
`@nx-lang/*` packages from a sibling NX checkout's workspace, the way the .NET build resolves
`../DrawnUi`. Release builds use the registry.

### D11. Publishing: the workspace packages join the editor-assets track

The release workflows already pack, attach and publish one npm `.tgz` with trusted publishing.
They are extended to pack each publishable workspace package (`pnpm pack` after `pnpm -r build`,
which rewrites `workspace:*` references to the release version), attach every `.tgz`, and publish
them in dependency order (`language-protocol`, `language-core`, `ir-runtime`, `sdk-wasm`, `monaco`)
with `--skip-duplicate` semantics. `@nx-lang/sdk-wasm` and `@nx-lang/ir-runtime` drop `private` and
gain `publishConfig.access: public`; `sdk-wasm`'s `files` already includes `dist`, which holds the
module.

## Risks / Trade-offs

- [Share artifacts are large while the catalog rides in every IR: a few hundred kilobytes compact]
  → Compact JSON in this change; the catalog-as-library follow-up removes the bulk. The private
  backend's artifact limit is unknown: confirm it with the backend owner before the release, and
  document the artifact size in the fiddle's README.
- [Monaco 0.52 differs from what `@nx-lang/monaco` is tested against] → A test in the package
  registers against a 0.52 namespace stub, and the fiddle's dev-host checklist exercises hover,
  completion and markers by hand.
- [Two Shiki-colored themes under the fiddle's own theme may look off] → Acceptable for the first
  release; the fiddle can pass its theme's Shiki equivalent later through the `themes` option.
- [Presets drift from the catalog when `drawnui-react` is bumped] → The stale-catalog check fails
  the build, and the preset compile test runs in the same build.
- [A compiler hang on the main thread freezes the tab] → The compiler is a type checker and code
  generator with no known non-terminating paths; the worker follow-up adds a deadline if one is
  found.
- [The `.d.ts`-based generator may resolve props differently from the source-based one] → Diff the
  fiddle's generated catalog against the playground's for the tags both versions share during
  implementation, and record intended divergences in the fiddle's catalog notes.
- [Duplicated walk and coercion between the playground and the fiddle] → Bounded (about 150 lines)
  and temporary: the playground's copy leaves with its DrawnUI target.
- [Compact IR breaks a text comparison somewhere] → The task list names the known text comparisons;
  CI runs every binding's tests.

## Migration Plan

1. NX repository first: compact IR, the SDK helper, the Monaco range, the publishing track. Cut a
   release so the packages exist on npm.
2. Fiddle repository: generator and catalog, runtime bundle, shim and engine hooks, C# descriptor and
   presets, docs. Develop against the sibling checkout, then pin the released versions.
3. Verify in the dev host with the local backend (the checklist in `docs/ADDING-A-LANGUAGE.md`),
   including a share played through `/p/{id}` and screenshots.
4. Rollback is removing the `NxLanguage` registration and the shim's `load` line; shares in `nx`
   then open as "unknown language" with their source, which the engine already handles.

## Open Questions

- The private drawfiddle.com backend's maximum share artifact size, if any. It does not change the
  design; it decides whether the catalog-as-library follow-up must land before NX shares are opened
  to the public.
