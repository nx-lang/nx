## Why

NX now has two browser editors and neither has language intelligence. The DrawnUI fiddle
(`sample-apps/drawnui-react/src/editor/`) and ReachMe's config editor both mount Monaco over the
repository's TextMate grammar, and both stop at highlighting: no hover, no completion, no live
diagnostics. The Rust language service that drives VS Code already answers all three, and the
`resolve-editor-positions` and `improve-hover-content` changes made those answers worth showing. What
is missing is a way for a browser to ask.

Each host has, separately, built the same pieces in a different way — the fiddle bridges the grammar
through `vscode-textmate` with a hand-written theme, ReachMe through Shiki with GitHub themes; the
fiddle reaches the grammar by a relative path into `src/vscode`, ReachMe by a `file:` link into a git
submodule. Adding hover and completion to each host independently would double that divergence.
The right moment to share is before the providers exist, not after.

Two facts about the language service block ReachMe specifically, and were discovered by reading its
integration:

- **Libraries loaded through a build context are invisible.** `WorkspaceSnapshot` hardcodes
  `ProgramBuildContext::empty()` for both diagnostics (`crates/nx-language-service/src/lib.rs:296`)
  and declarations (`lib.rs:767`). ReachMe supplies its built-in NX modules through
  `NxLibraryRegistry.loadFromDirectory`, so today hover and completion would report nothing for any
  name those libraries declare. The fiddle sidesteps the same gap by concatenating its catalog into
  the visitor's source and shifting every position back.
- **Positions are not in any editor's units.** `LineIndex` reads an incoming `character` as a count
  of Unicode scalars and writes an outgoing one as a count of bytes from the line start. LSP's
  default and Monaco both count UTF-16 code units. A string literal containing an emoji earlier on
  the line shifts every later query on that line, in VS Code today.

The longer-term goal is that ReachMe consumes NX as published npm packages rather than a submodule.
`@nx-lang/language` (the grammar and language configuration) has a release pipeline but, as of this
change's implementation, no release has published it to the registry; nothing that would carry
language intelligence has a pipeline at all. This change shapes the new packages so that publishing them
is a pipeline change rather than a restructuring.

## What Changes

- **The language service accepts a build context.** A `WorkspaceSnapshot` can be built against a
  `ProgramBuildContext`, and diagnostics, hover, completion and symbols see every library module
  that context makes visible, with the same visibility rules the compiler applies. A snapshot built
  without one behaves as today.
- **Editor positions are UTF-16 code units, in both directions.** `LineIndex` converts incoming
  `character` values and outgoing ranges as UTF-16 offsets from the line start. This is the LSP
  default encoding, so `nx-lsp` needs no protocol change. **This changes the meaning of
  `TextPosition.character` for any caller sending non-ASCII text.**
- **`@nx-lang/sdk-node` exposes the language service.** A new `NxLanguageSnapshot` class is built
  from in-memory documents with logical URIs and an optional `NxProgramBuildContext`, and answers
  hover, completions, diagnostics and document symbols as typed results. The native crate gains a
  dependency on `nx-language-service`.
- **New package `@nx-lang/language-protocol`**: the transport-independent request and response
  shapes for language queries — a set of documents plus an active URI, UTF-16 positions, hover,
  completion, diagnostic and symbol results — and the `NxLanguageService` interface every client
  implementation satisfies. Types only.
- **New package `@nx-lang/language-http`**: a stateless, mountable Node handler in the Fetch API
  shape (`Request` → `Response`) that answers protocol requests through `@nx-lang/sdk-node`, with a
  content-keyed snapshot cache, a body limit, an optional build context, and an optional *prelude*
  document that is concatenated ahead of the active document with all positions shifted back — the
  fiddle's catalog workaround, moved into one tested place until the compiler can import external
  components without loss (NXE12/NXE13). Includes an adapter for Node's `http` listener signature.
- **New package `@nx-lang/language-client`**: the browser side of the protocol — an HTTP
  implementation of `NxLanguageService` taking a base URL and a `fetch` or header hook, owning
  request cancellation and timeouts.
- **New package `@nx-lang/monaco`**: one framework-free Monaco integration — language registration
  and configuration from `@nx-lang/language`, Shiki highlighting, hover and completion providers over
  an `NxLanguageService`, a workspace callback so a host can supply sibling documents, and helpers
  that map protocol diagnostics to Monaco markers. Idempotent under repeated registration, so it
  works beneath `@monaco-editor/react` and beneath a direct `monaco.editor.create` alike.
- **The fiddle consumes the packages.** Its hand-written TextMate bridge, custom theme, and
  relative grammar import are deleted in favor of `@nx-lang/monaco`; its server mounts the HTTP
  handler at `/api/language` with the DrawnUI catalog as the prelude; the source pane gains hover
  and completion. Compile-driven diagnostics are unchanged.
- **A root pnpm workspace** ties `packages/*`, `runtime/typescript`, `bindings/node` and
  `sample-apps/drawnui-react` together with one lockfile and `workspace:*` dependencies, replacing
  chained `file:` links and the three npm lockfiles. pnpm is what the extension, CI, and ReachMe
  already use. Each new package carries release-ready metadata (`exports` with types, `files`,
  `publishConfig`, a `prepack` build) and a pack-and-import verification script in the style of the
  existing `@nx-lang/language` verification.

Explicitly out of scope, and recorded as follow-ups:

- **Registry publication of the new packages, `@nx-lang/ir-runtime`, and `@nx-lang/sdk-node`.**
  The release pipeline (`package-release-automation`) currently attaches exactly one npm tarball
  per release, and `sdk-node` needs per-platform native prebuilds before it can be installed from a
  registry. Extending the pipeline is its own change; this one makes every new package pass
  `pnpm pack` verification so that change is mechanical.
- **ReachMe's migration.** Replacing its `monacoNxLanguage.ts` with `@nx-lang/monaco`, mounting
  the handler in its Hono API, and moving from the submodule to published packages happen in the
  ReachMe repository once packages are published.
- **Folding `src/vscode` into the root workspace.** The end state is `@nx-lang/language` as a real
  package directory under `packages/` that the extension, the Monaco package, and every other
  consumer depend on directly, with the staging script that fabricates the package today deleted.
  That fold moves paths four CI workflows reference and must verify VSIX packaging under a root
  workspace, so it is its own change.
- **Fixing NXE12/NXE13** so context modules can be separate documents everywhere and the prelude
  option can be deleted.
- **A WASM build of the service.** Both hosts have Node backends with the native binding loaded;
  in-browser analysis stays behind the same `NxLanguageService` interface for later.
- **New language features** — go-to-definition, rename, signature help, inlay hints, semantic
  tokens. The protocol reserves method names for them; none is implemented.

## Capabilities

### New Capabilities
- `language-protocol`: the transport-independent contract between a browser editor and the NX
  language service — request and response shapes, UTF-16 position encoding, the
  `NxLanguageService` interface, and the behavior of its HTTP client implementation.
- `language-http-service`: the mountable HTTP handler that answers `language-protocol` requests
  from a Node host — statelessness, caching, body limits, build-context and prelude support, and
  the Node listener adapter.
- `monaco-language-integration`: the shared Monaco integration — highlighting from the published
  grammar, hover and completion providers, workspace supply, marker mapping, and idempotent
  registration.

### Modified Capabilities
- `editor-language-service`: gains the requirement that a snapshot can be analyzed against a
  program build context so library declarations are visible to every query, and the requirement
  that editor positions and ranges are expressed in UTF-16 code units.
- `sdk-node`: gains the requirement that the Node SDK exposes the language service over in-memory
  documents with an optional build context, returning structured results.
- `drawnui-fiddle`: the source-pane requirement now includes hover and completion answered by the
  language service, and highlighting sourced through the shared Monaco integration rather than a
  fiddle-local bridge.

## Impact

- `crates/nx-language-service/src/lib.rs` — `WorkspaceSnapshot` stores a `ProgramBuildContext`
  (it is `Clone`); `build_workspace_declarations` and `diagnostic_report` pass it instead of
  `empty()`; `LineIndex` converts UTF-16 both ways. Serde types gain
  `rename_all = "camelCase"` so the JSON the binding emits matches the SDK's existing casing.
  `nx-lsp` compiles unchanged; its position tests gain a non-BMP case.
- `bindings/node/native` — new `NativeNxLanguageSnapshot` napi class; `Cargo.toml` gains
  `nx-language-service`. `bindings/node/src` — `NxLanguageSnapshot` wrapper, declarations, tests.
  `bindings/node/README.md` documents it.
- New directories `packages/language-protocol`, `packages/language-http`,
  `packages/language-client`, `packages/monaco`; new root `pnpm-workspace.yaml`, `package.json`
  with `packageManager`, and `pnpm-lock.yaml`; the per-directory `package-lock.json` files are
  deleted. `src/vscode` stays a nested pnpm workspace outside the root one for now.
- `sample-apps/drawnui-react` — `src/editor/nxLanguage.ts` deleted; `NxEditor.tsx` uses
  `@nx-lang/monaco`; `server/index.mjs` mounts the handler; `server/compile.mjs`'s prefix
  arithmetic moves into the handler's prelude support and is shared with compile; `package.json`
  drops `vscode-textmate` and `vscode-oniguruma`, adds `shiki` and `@shikijs/monaco`;
  `Dockerfile` and `README.md` follow the workspace layout.
- **Behavior change for existing callers.** Any consumer of `nx-language-service` or `nx-lsp`
  sending positions in a non-ASCII line gets different — now correct — results.
- ReachMe, when it migrates: `apps/web-app`'s `monacoNxLanguage.ts` and the highlighting half of
  `NxCodeEditor.tsx` collapse into a call to `@nx-lang/monaco`; `apps/api` mounts the handler with
  the same registry-backed build context it already builds for validation; the `nx-language`
  `file:` link becomes `@nx-lang/language`. Until the new packages are published, they are
  consumable from the submodule as pnpm workspace entries the way `@nx-lang/sdk-node` is today.
