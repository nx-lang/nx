## Why

The DrawnUI fiddle engine (`DrawnUi.FiddleEngine`, the engine behind drawfiddle.com) is a pluggable
in-browser playground: a language compiles or interprets in the visitor's tab and draws with DrawnUI,
and the engine supplies the editor, shares, the `/p/` player and screenshots. NX already has every
piece such a language needs: a WebAssembly compiler and language service (`@nx-lang/sdk-wasm`), a
TypeScript IR runtime, a Monaco integration, and a proven pipeline from NX source to DrawnUi.React
controls on CanvasKit, which is exactly the fiddle's React surface. Today that pipeline lives only in
the NX playground at nxlang.org, where DrawnUI is a stand-in target rather than the language's
concern. Putting NX into the fiddle gives NX a public DrawnUI playground with sharing and embedding
that the NX repository does not have to build or host, and lets the NX playground become a
general-purpose site later, since nothing DrawnUI-specific needs to stay in the NX repository for
the fiddle to work.

## What Changes

**In the fiddle repository** (`DrawnUi.FiddleEngine`):

- Add NX as a built-in third language of the engine, on the React surface, alongside C# and TSX:
  an `nx` language descriptor with NX presets, registered by the engine itself.
- Add an NX runtime bundle for fiddle hosts: the NX wasm compiler, the IR runtime, the Monaco
  integration with the published NX grammar, the DrawnUI catalog and the renderer that turns an
  evaluated NX value tree into DrawnUi.React controls, built by the existing `npm run runtime` step
  next to the DrawnUi.React runtime and loaded only when a visitor first uses NX.
- Generate the NX catalog of DrawnUI controls in the fiddle repository, from the `drawnui-react`
  package the fiddle pins, so the catalog always matches the engine the fiddle draws with. The
  catalog is committed and regenerated when the pin moves.
- Compile in the browser: NX source plus the catalog goes through the wasm SDK to NX IR, the IR
  runtime evaluates `root()`, and the renderer mounts the result through the engine's React surface.
  Errors show under the editor in the engine's `L{line}: message` form; squiggles, hover and
  completion come from the NX language service over the same catalog.
- Share and play: an NX share stores its NX IR as the share artifact, so the `/p/` player draws it
  with the IR runtime and the renderer and never loads the compiler. The artifact carries the program,
  not a frozen picture, so authored interaction can run in the player once the IR runtime dispatches
  actions, with no change to the share format.
- Two small language-agnostic extensions to the engine's React surface: a `prepare` hook a language
  can use to load its runtime before its first compile and before the player runs one of its shares,
  and per-language color-swatch patterns supplied by the registration rather than a hardcoded map.
- Documentation: `docs/ADDING-A-LANGUAGE.md` and `AGENTS.md` describe the new hooks and the NX
  language; the README lists NX among the languages.

**In the NX repository**:

- Publish the packages the fiddle consumes to npm: `@nx-lang/sdk-wasm` and `@nx-lang/ir-runtime`
  stop being private, and they ship with `@nx-lang/language-core`, `@nx-lang/language-protocol`,
  `@nx-lang/language-client` and `@nx-lang/monaco` through the existing package release track, next
  to the `@nx-lang/language` editor assets. The wasm module travels inside the `@nx-lang/sdk-wasm`
  package. The track publishes every workspace member that is not `private`, which withdraws
  `@nx-lang/language-http`: it depends on `@nx-lang/sdk-node`, which stays private, so a consumer
  could not install it. It becomes `private` until the Node SDK is published.
- Move the prelude-aware build helper out of the playground into `@nx-lang/sdk-wasm`: build a
  program from a prelude plus a document, emit NX IR, and classify each diagnostic as belonging to
  the document, the prelude or the program with spans shifted into the document's coordinates. The
  playground becomes its first consumer; the fiddle its second.
- Emit compact NX IR JSON from the wasm SDK and the FFI instead of pretty-printed JSON. The CLI's
  file output stays readable. This roughly halves what every fiddle share stores and what every
  playground compile moves between threads.
- Let `@nx-lang/monaco` register against a host-supplied Monaco namespace from 0.52 onward, including
  a Monaco loaded through the AMD loader as BlazorMonaco does, rather than requiring 0.56.

**Out of scope, recorded as follow-ups**: removing DrawnUI from the NX playground once the fiddle is
live, which is a separate change with its own question of what a general-purpose playground draws;
shipping the catalog's IR once with the runtime instead of inside every share; running the fiddle's
compile in a Web Worker as the playground does; MAUI or web exports and Premium publishing for NX;
and teaching drawfiddle.com's private site chrome to label NX shares, which is not in either
repository.

## Capabilities

### New Capabilities

- `fiddle-nx-language`: NX as a language of the DrawnUI fiddle engine: how it is selected and
  presented, how a snippet compiles and draws in the browser, how the DrawnUI catalog is derived from
  the package the fiddle ships, what the editor reports, how a share is stored and played, and the
  engine hooks a runtime-loading language needs.

### Modified Capabilities

- `sdk-wasm`: the package is published to npm with the module inside it; it exposes a prelude-aware
  build that classifies diagnostics by origin; the NX IR it emits is compact JSON.
- `typescript-ir-runtime`: the runtime is published to npm as `@nx-lang/ir-runtime`.
- `monaco-language-integration`: registration accepts a host-supplied Monaco namespace from 0.52
  onward, including an AMD-loaded Monaco.
- `package-release-automation`: the workspace npm packages the fiddle depends on are built,
  verified, attached and published on the same track as the editor-assets package.

## Impact

- **Fiddle repository, new**: `src/DrawnUi.Fiddle/Languages/NxLanguage.cs`, `FiddlePresetsNx.cs`,
  `wwwroot/fiddle-nx.js` (the registration shim); an `nx/` area with the catalog generator, the
  committed catalog and metadata, and the TypeScript runtime sources; `dev/build-nx-runtime.mjs`
  producing `wwwroot/nx/nx-runtime.js` and `nx.wasm` in the host.
- **Fiddle repository, changed**: `fiddle-react.js` (the `prepare` hook in run and play),
  `fiddle-intellisense.js` (color patterns from registrations), `ServiceCollectionExtensions.cs`
  (registers `NxLanguage`), `Fiddle.DevHost/wwwroot/index.html` (loads the shim), `package.json`
  (pins the `@nx-lang/*` packages, `shiki`, `@shikijs/monaco`, `typescript`), the three docs.
- **NX repository, changed**: `bindings/wasm` (public, `buildProgramWithPrelude`, compact IR),
  `runtime/typescript` (public), `packages/monaco` (peer range, namespace input),
  `crates/nx-codegen` (IR formatting option) and its callers in `nx-ffi`, `nx-cli` and the wasm
  crate, `sites/playground/src/compile/catalog.ts` (thin wrapper over the SDK), the release
  workflows and `docs/deployment*.md`, package READMEs.
- **Dependencies**: the fiddle gains the NX packages and Shiki; the NX repository gains no
  dependency. The backend contract shared with drawfiddle.com's private backend is unchanged: an NX
  share is an ordinary share with `lang: "nx"` and a `js` artifact.
- **Compatibility**: compact IR is byte-different from today's output; consumers parse JSON and are
  unaffected, but any test that compares emitted IR text must be updated. C# and TSX in the fiddle
  keep running, sharing and playing unchanged.
