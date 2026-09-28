## Why

The playground is a DrawnUI gallery. Every example assumes the DrawnUI catalog, the output is a
CanvasKit canvas, and the site carries its own copy of a renderer, coercion layer and catalog
generator that the DrawnUI fiddle (`https://fiddle.drawnui.net`) now owns. `specs/future.md`
records that duplication as the reason to drop DrawnUI from the playground. NX drawing interfaces is
the fiddle's job, and `launch-nxlang-website` links to it from the landing page.

What the site lacks is a place to try the language itself: to paste a snippet from the docs, see
what it evaluates to, change it, and share the result. That is also what makes "Open in playground"
on a docs code block possible, which matters more than any single example.

NX has no `print`. What a program shows is what its `root` function returns, so the output pane is
where the playground either helps or doesn't. A flat dump of text makes records and sequences of
records hard to read. The value deserves a viewer that folds, highlights and explains itself on
hover, and one that other hosts can reuse.

## What Changes

- **BREAKING: DrawnUI leaves the playground.** Removed:
  - the gallery, the DrawnUI examples, the catalog (`catalog/skia.nx`) and its generator;
  - the renderer, `drawnui-react`, CanvasKit and `react-reconciler`;
  - the vendored fonts, images, animations and shaders, and the reference demo pages;
  - `sync-drawnui` and the catalog, coverage and capability tooling.

  The `drawnui-nx-catalog` spec is retired, because the fiddle keeps its own catalog, specified by
  `fiddle-nx-language`.
- **BREAKING: the playground moves to `/play`.** Language sites mostly use `play` (TypeScript, Go,
  Rust, Kotlin, CUE), and share links get shorter. The label stays "Playground". Nothing redirects
  from `/playground`, which went public a day before this change was written.
- **A language playground.** The playground becomes one editor view: NX source on one side, and the
  value its `root` returns on the other. Diagnostics, runtime errors and a missing `root` are
  reported in the output pane in place of a value. Hover in the source pane stays as it is.
- **A reusable NX value viewer.** A new package, `@nx-lang/value-view`, provides an `<nx-value>`
  custom element with no framework dependency. It shows a value in NX syntax, exactly the text
  `nxlang run` prints, highlighted with the grammar the editors use. Records and sequences that span
  several lines fold. Hovering a type name, property or case shows what it is, in content the host
  supplies. The playground's output pane is its first user; the docs, the fiddle and a VS Code
  webview can use it later.
- **Examples of the language, kept out of the way.** 10 to 20 short examples, one or two per topic,
  in a compact drop-down in the toolbar. They exist so a visitor never faces a blank page and can
  see what to write, not to be browsed as a gallery. Topics:
  - basics
  - records and defaults
  - unions
  - occurrences `?`/`+`/`*`
  - sequences
  - `if` and `for` with ranges
  - functions and function values
  - components
  - text and interpolation

  Each example's expected output is committed and checked, so the examples double as tests.
- **Source in the address.** The source is encoded in the URL fragment (`/play#code=…`,
  deflate-compressed and base64url-encoded). A Share button copies that address. Edits keep the
  fragment current, so a reload keeps the visitor's work. `/play/<id>` opens an example.
- **"Open in playground" on the docs.** Every checked `nx` code block on the website gets a button
  that opens exactly that block in the playground. A new `nx output` fence shows a block's evaluated
  value, and the docs check verifies it. That includes the landing page's hero, whose value
  `launch-nxlang-website` wrote by hand.
- **The wasm SDK evaluates, with annotations.** It gains `NxProgramArtifact.evaluateNx()`, which
  runs `root` and returns its NX text together with annotations. The annotations mark every value,
  property and type name in that text with its role, its type, and where it is declared. The formatter
  moves from `nx-cli` into `nx-api`, so the CLI, the playground and the docs check share one
  implementation. The module's ABI version is bumped.
- **Matching chrome.** The playground gets the website's header (NX, Docs, Playground, GitHub) and a
  layout that stacks its panes on a phone.

Evaluating a function on hover in the language service, when its arguments are known from the
source, is a separate follow-up change. It will reuse the formatter this change moves into
`nx-api`.

## Capabilities

### New Capabilities
- `value-view`: the `<nx-value>` element that shows an annotated NX value, with highlighting,
  folding, hover and copying.

### Modified Capabilities
- `playground`: the DrawnUI requirements are removed; new requirements cover the `/play` address
  scheme with shared source, the output pane built on `value-view`, language examples with checked
  output in a drop-down, the shared header and a narrow layout. The editor, diagnostics and
  in-browser compile requirements are carried forward without the catalog.
- `sdk-wasm`: adds evaluating a program artifact's `root` to annotated NX text.
- `website`: the playground's prefix becomes `/play`; adds "Open in playground" on code blocks and
  checked `nx output` blocks.
- `drawnui-nx-catalog`: every requirement is removed, and the spec directory is deleted at archive.

## Impact

- **Depends on** `launch-nxlang-website`, which creates the `website` capability, the playground's
  static hosting and its Worker routing. Archive that change first.
- **Rust**: `crates/nx-cli/src/format.rs` moves to `nx-api` and learns to emit annotations beside
  the text. The CLI calls it there with no change in output. `nx-api` gains a root-to-annotated-text
  entry point. `bindings/wasm/native` gains an evaluate export, and `ABI_VERSION` goes from 2 to 3.
- **TypeScript**:
  - `@nx-lang/sdk-wasm` gains `evaluateNx()`, with parity tests against the CLI's output.
  - The new `packages/value-view` holds `@nx-lang/value-view`. It highlights through Shiki and the
    grammar `@nx-lang/language` publishes, as `@nx-lang/monaco` does.
  - The playground drops `@nx-lang/ir-runtime`, `drawnui-react`, `canvaskit-wasm` and
    `react-reconciler`.
- **Addresses**: the playground Worker's routes, the Vite base, `base.mjs`, the router, `_headers`,
  the website's header and landing links, the Starlight link-validator exclusion, the deploy
  workflow's `SITE_URL`, and `docs/deployment.md` and `docs/deployment-setup.md` all move from
  `/playground` to `/play`. The Worker keeps its name, `nxlang-playground`, and the site stays in
  `sites/playground`.
- **Deleted**:
  - `sites/playground/{catalog,reference,docs,public/{shaders,anims,lottie,fonts,images}}`;
  - `src/{render,gallery,compile/catalog.ts,drawnui-runtime.ts}`;
  - the DrawnUI scripts and the `virtual:nx-catalog-artifact` Vite plugin.
- **Website**: an Expressive Code plugin for the button, the shared fragment encoder, and
  `nx output` support in the code-block check.
- **Docs**: the playground README; `specs/future.md` loses "Removing DrawnUI from the playground",
  "Shareable edited source", "NX IR size" and the playground framing of "The catalog as a library
  artifact", which moves to the fiddle's section.
- **Release**: the next package release ships the new `@nx-lang/sdk-wasm` API and the first
  `@nx-lang/value-view`. The fiddle is unaffected until it opts in, because the ABI bump travels
  inside the package.
