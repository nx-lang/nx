## 1. Schema and compiler prerequisites

- [x] 1.1 Write the schema 3 document in `docs/nx-ir-format.md`: artifact header, module table, string/type/constant/node tables, declaration layouts per kind, reference shape `[slot, name]`, debug section, and the numbered kind tables; verify a reviewer can hand-decode the corpus's smallest artifact from the document alone
- [x] 1.2 Confirm in `crates/nx-hir` that two top-level declarations of one name in a module are rejected, adding the diagnostic if they are not; verify a test with `let Card = 1` and `type Card = { }` reports an error naming `Card` and both spans
- [x] 1.3 Add `implicit_imports` to the workspace build and snapshot inputs in `crates/nx-api`, synthesizing a wildcard import into every module not itself listed at lowering; verify tests cover a control used without an import line, the implicit module analyzed without a self-import, an unknown identity failing the build, and a document that is a single trailing element compiling
- [x] 1.4 Expose `implicit_imports` through `crates/nx-ffi`, `bindings/wasm/native`, `bindings/node` and `bindings/dotnet`; verify each binding's workspace test passes the option and produces the same program as the Rust API

## 2. Emitter: schema 3 in `crates/nx-codegen`

- [x] 2.1 Replace the `NxIrProgram` model with the schema 3 artifact model: one module per artifact, module table with identity, version and fingerprint, tables and declaration list; verify `cargo build -p nx-codegen` and the model round-trips through serde
- [x] 2.2 Rewrite the IR builder to intern strings, types and constants, emit nodes into the table with `[slot, name]` references, declaration-local integer slots and module-local element ids, and drop expression ids; verify the corpus programs emit and `nxlang ir explain` renders them without a raw index
- [x] 2.3 Add emit options: which modules to emit (default entry) and whether to include the debug section, with each module's version string taken from the workspace module it was built from; verify emitting a snippet omits the catalog's declarations and lists it in the module table with its version, and that the artifact with and without debug differs only in the `debug` key
- [x] 2.4 Emit the debug section as spans parallel to declarations and nodes plus the source text; verify a runtime diagnostic in a debug-bearing artifact cites identity and span
- [x] 2.5 Bump `NX_IR_SCHEMA_VERSION` to 3 and the runtime ABI to `nx-ir-runtime-v2`, keeping the feature strings; verify the codegen tests assert the new header

## 3. Conformance corpus

- [x] 3.1 Create the corpus directory with NX sources covering every node, type and declaration kind, a two-module program with cross-artifact references, derived declarations, and a trailing-element document, each with expected artifacts (with and without debug) and expected canonical results; verify a manifest lists every kind and the codegen test fails if a kind is uncovered
- [x] 3.2 Add the emitter's corpus test in `crates/nx-codegen` that pins each artifact byte-for-byte and reports differences as explained text; verify a deliberate emitter change fails the test naming the program
- [x] 3.3 Add the size-budget test: every corpus artifact emitted without debug is at most six times its module source's UTF-8 length; verify it passes and prints the ratio per program

## 4. TypeScript runtime: prepare, link, evaluate

- [x] 4.1 Replace the IR types in `runtime/typescript/src/index.ts` with the schema 3 model and refuse schema 2 with a diagnostic naming both versions; verify a schema 2 fixture is rejected
- [x] 4.2 Implement `prepareNxIrModule`: header validation, name-indexed declarations, entrypoint tables, schema validators and default evaluators over the tables; verify a self-contained corpus artifact prepares and evaluates without linking and an unlinked multi-module artifact refuses evaluation
- [x] 4.3 Implement `linkNxIrProgram` with a resolver, strict version checking, `allowVersionMismatch`, eager missing-declaration checks and merged nominal shapes; verify tests cover a successful link, a version mismatch refused then allowed, a missing declaration, an unresolvable module, and one prepared module linked into many programs
- [x] 4.4 Port evaluation to the node table: slots as frame integers, references through the linked table, spans from the debug section when present and declaration names otherwise; verify `runtime.test.ts` and `emitted-ir.test.mjs` pass against the new emitter
- [x] 4.5 Add the runtime's corpus test evaluating every corpus artifact's named entrypoints; verify it runs in `pnpm test` for `runtime/typescript` and fails on a changed expected result
- [x] 4.6 Update `runtime/typescript/README.md` for prepare, link and the resolver; verify the README's example links a snippet against a separately prepared module

## 5. SDKs and CLI

- [x] 5.1 Add the workspace build and the emit call to `bindings/wasm/native` and the `NxHost` API in `bindings/wasm`, keeping the single-source build as a one-module wrapper, and delete `buildProgramWithPrelude` and `prelude.test.ts`; verify `sdk-wasm.test.ts` covers the snippet-without-catalog and catalog-alone scenarios and the parity test matches the Node SDK byte-for-byte
- [x] 5.2 Replace the wasm language service's `prelude` option with host documents plus `implicitImports`; verify `language-service.test.ts` covers hover and completion through an implicit import and a host-document diagnostic reported against its own URI
- [x] 5.3 Update `bindings/node`, `crates/nx-ffi` and `bindings/dotnet` emission and their tests to schema 3 with the emit options, and give each SDK's workspace module the optional version string the wasm SDK takes; verify the wasm and Node SDKs emit a versioned workspace byte-for-byte alike, and `pnpm test` in `bindings/node` and `dotnet test bindings/dotnet/NxLang.sln` pass
- [x] 5.4 Make the CLI's `nx-ir` target write one pretty-printed artifact per module with debug data, named from the module identity; verify the CLI tests cover a single file and a workspace entry with a referenced module
- [x] 5.5 Add `nxlang ir explain <artifact>` rendering declarations, references, types, nodes and spans with every index resolved, refusing other schema versions; verify a test explains the corpus's snippet artifact and the output contains no bracketed index
- [x] 5.6 Update `bindings/wasm/README.md` and the CLI docs for workspace builds, implicit imports, emit options and explain; verify the READMEs no longer mention the prelude build

## 6. Playground and release

- [x] 6.1 Move `sites/playground` to a two-module workspace with the catalog implicitly imported, deleting `compile/catalog.ts`'s prelude use and the worker's shifting; verify `pnpm run typecheck`, `pnpm test` and `pnpm run check-examples` pass and a trailing-element example still compiles
- [x] 6.2 Update the archived spec text for `fiddle-nx-language` and the `playground` spec's implementation notes where they mention the prelude; verify `openspec validate --strict` passes for this change
- [x] 6.3 Emit the playground's catalog artifact at build time (a Vite plugin serving `virtual:nx-catalog-artifact` through `emitCatalogArtifact`), make a compile emit the visitor's module alone, and have the renderer prepare the catalog once and link each compile against it; verify the compile, session, renderer and Vite config tests cover the snippet-only artifact, the bundled artifact and a failed link, and that `pnpm run check-examples` links every example the same way
- [x] 6.4 Run `cargo test --workspace`, `pnpm -r test` and the .NET tests; verify all green
- [ ] 6.5 Tag a release and confirm `npm view` answers with the new versions of `@nx-lang/sdk-wasm`, `@nx-lang/ir-runtime` and `@nx-lang/language-core` (after the fiddle has been tried against the checkout)

## 7. Fiddle repository (after 6.5)

- [ ] 7.1 Branch from the fiddle's current main, pin the published packages, and rebuild; verify `npm run runtime` builds and the existing C# and TSX flows still run, share and play in the dev host
- [x] 7.2 Emit the catalog artifact in `dev/build-nx-runtime.mjs` from `nx/catalog/drawnui.nx` with the version from `catalog-meta.json`, bundle it into nx-runtime.js, and fail the build on a stale artifact; verify a changed catalog source fails naming the regenerate command
- [x] 7.3 Change `nx/src/runtime.ts`: `prepare` prepares the catalog once, `compile` builds the two-module workspace and emits the snippet without debug, `mount` links strictly then with `allowVersionMismatch` and draws the failure label naming both versions; verify tests cover a share linked against the bundle, a regenerated catalog of the same version still drawing, and a version mismatch drawing when names resolve
- [x] 7.4 Add the preset budget test: every NX preset's share module is under 128 KB; verify it passes and prints each size
- [x] 7.5 Move NX registration from `ServiceCollectionExtensions.cs` to `Fiddle.DevHost/Program.cs` and the script tag to the dev host's `index.html`; verify the dev host offers NX and a host with only the engine's registration does not
- [x] 7.6 Update `docs/ADDING-A-LANGUAGE.md`'s NX section, `nx/CATALOG.md` and `AGENTS.md` for the catalog artifact, the share contents and host registration; verify the docs describe what a share carries and where NX is registered
- [ ] 7.7 Open the new pull request against main describing the share format and the registration change, and close the previous one; verify the PR's checks pass in the dev host
