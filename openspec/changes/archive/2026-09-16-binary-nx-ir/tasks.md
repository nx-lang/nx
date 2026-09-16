## 1. Image writer and reader in `crates/nx-codegen`

- [x] 1.1 Add `ir_image.rs` with the header, section directory and cell rules of design D1 and D2, and `write_nx_ir_image(&NxIrArtifact) -> Vec<u8>`; verify a unit test writes the corpus's `snippet` artifact and hand-checks the magic, version, length, directory offsets and the `root` declaration's cells against the document
- [x] 1.2 Add `NxIrImage::open(&[u8])` performing the eager validation of design D4 and the borrowed accessors (strings, module table, entrypoints, table entries, debug spans), plus `to_artifact()`; verify a round-trip test asserts `open(write(a)).to_artifact() == a` for every corpus artifact with and without debug
- [x] 1.3 Add the damage tests: truncate each corpus image at every four-byte boundary and overwrite each cell of the smallest one with `0`, `1`, `0xFFFFFFFF` and a large value; verify `open` returns `Err` or a valid view every time and never panics
- [x] 1.4 Delete the serde derives on the IR model, `NxIrFormat`, `serialize_nx_ir`, `NxIrEmitOptions.format` and `explain_nx_ir_json`; change `GeneratedNxIr.json` to `bytes: Vec<u8>` and add `explain_nx_ir_image(&[u8])`; verify `cargo build --workspace` and `ir_tests.rs` pass with their JSON assertions rewritten against the model or the explained text
- [x] 1.5 Rewrite `ir_corpus_tests.rs` to pin `expected/<identity>.nxir` and `.stripped.nxir` byte for byte, write `.nxir.txt` and `.stripped.nxir.txt` beside them under `NX_UPDATE_CORPUS=1`, and fail when a committed text differs from the explanation of its committed image; regenerate the corpus, delete the JSON files, add `*.nxir binary` to `.gitattributes` and update `specs/ir-conformance/README.md`; verify the pin, stale-text, coverage and size-budget tests pass and the budget test prints ratios under six

## 2. TypeScript runtime

- [x] 2.1 Replace `TableReader` with a typed-array reader over the image: header and directory validation, `Uint32Array` views per pool, lazy memoized string decoding, `BigInt` fingerprints as decimal strings, a copy only when the input view is unaligned; verify a test prepares each corpus image from an `ArrayBuffer` and from a `Uint8Array` at offset 2
- [x] 2.2 Change `prepareNxIrModule`, `tryPrepareNxIrModule`, `prepareNxIrProgram` and `tryPrepareNxIrProgram` to take `Uint8Array | ArrayBuffer`, remove `NxIrArtifact` and `NxIrItem` from the public API, and read node cells and debug spans from the image during evaluation; verify `runtime.test.ts`, `emitted-ir.test.mjs` and `corpus.test.mjs` pass
- [x] 2.3 Replace the test suite's `ArtifactBuilder.build()` with an image writer following design D2 and add the runtime's damage test over the corpus images; verify every truncation and cell overwrite yields a diagnostic from `tryPrepareNxIrModule` and never a throw
- [x] 2.4 Update `runtime/typescript/README.md` for byte input, lazy strings and the alignment note; verify the README's example prepares bytes returned by the wasm SDK

## 3. SDKs, FFI and CLI

- [x] 3.1 Define the bundle framing of design D6 in `crates/nx-codegen` (a writer and a reader over `[{ identity, metadata, offset, length }]` plus images) and use it in `nx_wasm_program_nx_ir` and `nx_codegen_nx_ir`; add `nx_wasm_ir_explain` and `nx_ir_explain`; verify Rust unit tests round-trip a two-artifact bundle and `crates/nx-ffi/tests/ffi_smoke.rs` reads the images back
- [x] 3.2 Update the wasm loader (`abi.ts`, `host.ts`, `types.ts`) so `generateNxIr` returns `{ identity, bytes: Uint8Array, metadata }` copied out of module memory, and add `NxHost.explainNxIr(bytes)`; verify `sdk-wasm.test.ts` covers bytes surviving result release, the snippet-without-catalog and catalog-alone scenarios, `explainNxIr` matching the CLI's text for the corpus snippet, and a malformed image throwing without trapping
- [x] 3.3 Update `bindings/node` (napi objects with a `Buffer`, `explainNxIr`) and the parity test to compare `bytes` across the wasm and Node SDKs; verify `pnpm test` in `bindings/node` and `parity.test.ts` in `bindings/wasm` pass
- [x] 3.4 Update `bindings/dotnet` (`NxGeneratedNxIr.Bytes`, bundle reader, `NxRuntime.ExplainNxIr`) and its tests; verify `dotnet test bindings/dotnet/NxLang.sln` passes and an end-to-end test asserts the bytes equal the Node SDK's for the same input
- [x] 3.5 Make the CLI's `nx-ir` target write `<identity>.nxir` and `nxlang ir explain` read an image through `explain_nx_ir_image`, exiting with a failure code and a diagnostic on a malformed or unsupported file; verify the CLI tests cover a single file, a workspace, an explained artifact without indices, a schema 2 refusal and a truncated file

## 4. Playground and documentation

- [x] 4.1 Change the Vite plugin to export the catalog image as base64 decoded on first use in `render/catalog.ts`, make `compileWithCatalog` and the worker return a `Uint8Array` with its buffer transferred, and update `check-examples.mjs` and `emit-example-ir.mjs`; verify `pnpm run typecheck`, `pnpm test` and `pnpm run check-examples` pass in `sites/playground`
- [x] 4.2 Rewrite `docs/nx-ir-format.md` for the image: header, directory, sections, the cell rules, a hex-annotated worked example of the corpus `snippet`, and the CLI section; verify a reviewer can hand-decode `snippet`'s image from the document alone
- [x] 4.3 Update `bindings/wasm/README.md`, `bindings/node/README.md` and the CLI docs for bytes, `explainNxIr` and `.nxir` files; verify no README mentions IR JSON text or the compact/pretty choice
- [x] 4.4 Run `cargo test --workspace`, `pnpm -r test` and the .NET tests; verify all green before the `linkable-compact-nx-ir` release is tagged

## 5. Fiddle repository (with `linkable-compact-nx-ir` 7.x)

- [x] 5.1 On the fiddle's `nx-linkable-ir` branch, commit the catalog as `nx/catalog/drawnui.nxir`, compare bytes in the runtime build's staleness check, and bundle the image as base64 decoded once in `prepare`; verify `npm run runtime` fails on a stale artifact naming the regenerate command
- [x] 5.2 Change `moduleFor` to emit `const IR = "<base64>"` and `mount` to decode with `Uint8Array.fromBase64` or `atob`; verify the runtime tests link a share against the bundle and the preset budget test passes printing each size under 128 KB
  - Note: 5.1 and 5.2 are applied in the working tree of the fiddle's `nx-linkable-ir` branch (`~/src/DrawnUi.FiddleEngine`) but not yet committed there; the commit and its verification wait on the published `@nx-lang` packages.
