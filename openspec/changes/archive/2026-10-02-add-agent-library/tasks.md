## 1. Preconditions

- [x] 1.1 Confirm `add-function-reference-type` has landed: the function reference type `<function ... />: R` and the required feature `function-reference-type-v1`. Verified on 39d9932: the D6 source compiles as written and `nxlang ir explain` on its image prints `requires function-reference-type-v1` (design.md, Verification)
- [x] 1.2 Check whether `retire-hir-interpreter` has landed and record which evaluators exist, since it decides where tasks 3.5 and 3.7 run. Verify by listing `crates/` for `nx-interpreter`. Result: not landed; `crates/nx-interpreter` exists beside `crates/nx-ir-runtime` and the TypeScript IR runtime, so 3.5 and 3.7 run on the IR runtimes only

## 2. Standard library mechanism (`crates/nx-hir`, `crates/nx-api`)

- [x] 2.1 Rename `PRELUDE_ROOT_PREFIX` to `NX_RESERVED_ROOT_PREFIX` in `crates/nx-hir/src/lib.rs` (the list of standard libraries is the table of task 2.2 alone; a second list in `nx-hir` was removed after review); verify `cargo build --workspace` and the existing prelude tests pass unchanged
- [x] 2.2 Add the static standard library table (name, stability, modules as identity and embedded source, TypeScript package, C# namespace) and `standard_library(root)` in `crates/nx-api/src/artifacts.rs`, building each library once through `build_library_artifact_from_sources`; narrow the skip in `apply_prelude_bindings` to the prelude itself so a standard library's modules see the prelude. Verify with a test that two calls return the same `Arc` and that the artifact has no diagnostics
- [x] 2.3 Answer for roots under `@nx/` in `LibraryRegistry::get_loaded_library`, `ProgramBuildContext::visible_library` and the logical-identity lookups, so an empty registry and a context with limited visible roots both resolve a standard library; verify tests for both, and that a context limited to one host root still resolves `@nx/agent`
- [x] 2.4 Recognize an import path whose first segment is `@nx` before relative normalization, in the workspace, single-source, directory-library and in-memory-library pipelines; verify tests that `import "@nx/agent"` resolves from `main.nx`, from `app/flows/support.nx`, from a directory library module and from an in-memory library module
- [x] 2.5 Add the `unknown-standard-library` diagnostic for `@nx/nope` and `@nx/agent/agent.nx`, listing the libraries that exist; verify tests for both paths and for the namespace and selective import forms
- [x] 2.6 Accept a standard library root as an implicit import: pass the check in `unknown_implicit_import_diagnostics`, synthesize `import "@nx/<name>"` verbatim in `synthesize_implicit_imports`, and skip it when the module already writes that wildcard; verify tests for an implicit `@nx/agent`, an implicit `@nx/nope`, and a module that also writes the import
- [x] 2.7 Refuse every workspace module identity under `@nx/` in `LogicalModuleGraph::from_modules` and every host library root under `@nx/` in in-memory and directory loads, with diagnostics naming the identity or root; verify tests, including that `@nx/prelude.nx` is still refused with the reserved-root message
- [x] 2.8 Record a standard library as an ordinary dependency root of a library that imports it, and include it in a program's library closure; verify a test that a host library importing `@nx/agent` loads into an empty registry and that a program importing only that host library links `@nx/agent`
- [x] 2.9 Verify existing duplicate-import and ambiguous-name tests have standard-library counterparts: `import "@nx/agent"` twice, and a local `Tool` beside a wildcard import reported only on use
- [x] 2.10 Confirm the carried sources embed the same bytes on every platform. `.gitattributes` already has `*.nx eol=lf`, so no rule is added; verified `git check-attr eol crates/nx-api/src/prelude.nx` reports `lf`

## 3. The agent library source

- [x] 3.1 Add `crates/nx-api/src/std/agent/agent.nx` with the source in design.md D6; verify a test that the library builds with no diagnostics of any severity and that its exports, leaving out derived companions, are exactly the thirteen types
- [x] 3.2 Add a test that every exported type and every field of the library has documentation attached; verify it fails when a `///` line is removed
- [x] 3.3 Add source-level tests for the `agent-library` scenarios that are type-level: required `instructions`, `instructions` by name, any `model` string, abstract `Tool`, a non-function at `FunctionTool.function`, a forged `<Function module=... name=... />` at `FunctionTool.function`, a function of another result at `HttpTool.arguments`, an unknown `HttpMethod` case, `auth` rejected on `HttpConnection`, `HttpArguments` with no type argument; verify each passes
- [x] 3.4 Add tests that a second library imports `@nx/agent` and declares `RecordSearchTool extends Tool` with a defaulted field, `ChatToolContext extends ToolContext`, and an external component with `agent?: Agent` and a content string; verify the library loads and a program uses all three
- [x] 3.5 Add a test that a host library exporting `let surveyAgent: Agent = <Agent:markdown ...>` and declaring, in another module, an external component with `agent: Agent = {surveyAgent}` loads, and that a program omitting `agent` evaluates in the IR runtimes to a step holding that agent; verify it passes
- [x] 3.6 Add the worked example from design.md D12 as a checked-in fixture and verify it type-checks with `@nx/agent` as an implicit import. The fixture is the conformance corpus program `specs/ir-conformance/agent-library`, so both IR runtimes also evaluate it and its two images are pinned
- [x] 3.7 Add a source-level evaluation test for a `Tool+` field holding a subtype declared in another module. Run it on the Rust IR runtime if `retire-hir-interpreter` has landed; otherwise run it on the IR runtimes only and record the interpreter failure in that change's notes. Verify the expected value is three records with the host subtype's default applied

## 4. NX IR (`crates/nx-codegen`)

- [x] 4.1 Return the content-derived version from `module_version` for a module under a standard root: 16 lowercase hex digits of FNV-1a 64 over each module's identity, a zero byte, its source and a zero byte, in identity order; verify a test that pins the value for a fixed two-module fixture and that it is equal across the native, wasm and node builds; add a test that pins the version of the shipped `@nx/agent`, so an unintended change to the embedded bytes (a line-ending change included) fails
- [x] 4.2 Verify with tests that an entry using `Agent` names `@nx/agent/agent.nx` in its module table and holds none of its declarations, that the library image is emitted when named and for an every-module request, and that two different programs emit byte-identical library images
- [x] 4.3 Verify with a test that the `@nx/agent/agent.nx` image lists `function-reference-type-v1` among its required features, that an entry image whose own type table does not hold a function reference type does not, and that `nxlang ir explain` prints the feature
- [x] 4.4 Verify with a test that a program importing no standard library emits the same bytes as before this change, using an existing golden image
- [x] 4.5 Verify that the program fingerprint differs between two builds whose standard library source differs, using two test-only library entries built outside the table (`a_program_fingerprint_depends_on_the_standard_library_source`)
- [x] 4.6 Update `docs/nx-ir-format.md`: the whole `@nx/` root is reserved, a standard library module is an ordinary linked module, its version is content-derived, and no runtime carries its image; verify the document's module-table section and the prelude section agree

## 5. Runtimes (tests only)

- [x] 5.1 Add a TypeScript IR runtime test that prepares the entry and `@nx/agent/agent.nx` images of the D12 fixture, links them, evaluates `root`, and calls `findPlans`, `lookupOrder` and `openTicket` through `callFunction`; verify the evaluated shapes in the `agent-library` scenarios
- [x] 5.2 Add the same round trip for the Rust IR runtime in `crates/nx-ir-runtime/tests`, over the corpus images since that crate's tests use no compiler; verify it passes
- [x] 5.3 Add a link test that a resolver returning nothing for `@nx/agent/agent.nx` fails naming it, and that a library image with another version fails with the version error unless mismatches are allowed; verify in both runtimes

## 6. CLI and typegen (`crates/nx-cli`)

- [x] 6.1 Accept `@nx/<name>` as the `typegen` input before treating the argument as a path, requiring an output directory as for a library; verify `nxlang typegen @nx/agent --language typescript -o <dir>` writes `agent.ts`, `_nx.ts` and `index.ts`, and that `@nx/nope` fails listing `@nx/agent` and writes nothing
- [x] 6.2 Verify `nxlang typegen @nx/agent --language csharp --csharp-namespace NxLang.Agent -o <dir>` writes the C# contracts, and that generated doc comments carry the NX documentation in both languages. The one expected warning is that abstract `ToolContext` has no exported concrete descendant
- [x] 6.3 Emit the fixed TypeScript package (`@nx-lang/agent`) and C# namespace (`NxLang.Agent`) for references to a standard library's types, unaffected by `--typescript-package-prefix` and `--csharp-namespace` and with no assumed-target warning; verify tests for a host library that extends `Tool` and holds an `Agent` field
- [x] 6.4 Verify `nxlang run` and `nxlang codegen --target nx-ir` on a file that imports `@nx/agent`: the run prints the `Agent` as NX text, and codegen writes `@nx/agent/agent.nxir` beside the entry image (a standard library module keeps its directories in single-file output, so it cannot take the entry's file name)
- [x] 6.5 Update the CLI help text for `typegen`'s argument and verify `nxlang typegen --help` mentions the standard library form

## 7. Language service and LSP

- [x] 7.1 Index a standard library in `WorkspaceDeclarations` when a snapshot document imports it or the snapshot's implicit imports name it, with no build context; verify hover, completion and diagnostics tests for the `editor-language-service` scenarios
- [x] 7.2 Add the hover label that identifies a standard library declaration and names the library; verify the hover text for `Agent` and for the field `Agent.model`
- [x] 7.3 Verify completions offer no `@nx/agent` name in a document without the import, and offer inherited properties inside `<FunctionTool `. Property-name completion was offered only for components; it now covers a record written as an element, which every agent type is
- [x] 7.4 Add an LSP test parallel to `the_lsp_offers_and_describes_the_prelude_with_no_build_context` for an imported standard library; verify it passes

## 8. SDK parity (`bindings/wasm`, `bindings/node`, `bindings/dotnet`)

- [x] 8.1 Add wasm SDK tests for the `sdk-wasm` scenarios: single-source import, tenant source through a host library with implicit imports, emission of the library image, and a library load under `@nx/` refused; verify they pass and that the native crate's export list is unchanged
- [x] 8.2 Extend the wasm and Node parity tests with a workspace that imports `@nx/agent`: byte-identical entry and library images, equal diagnostics, equal hover and completion answers; verify they pass
- [x] 8.3 Add a .NET SDK test that builds a source importing `@nx/agent` and evaluates `root`; verify it passes
- [x] 8.4 Add a "Standard libraries" section to `bindings/wasm/README.md` and `bindings/node/README.md`: nothing is loaded and a load under `@nx/` is refused, the import form, implicit imports, the explicit import a host library module needs, emitting the library image, reading its version from the prepared image, and that the library is unstable; verify the README example compiles in a package test (`bindings/wasm/test/standard-libraries.test.ts` and the standard library test in `bindings/node/test/sdk-node.test.ts` run the same calls)

## 9. Documentation

- [x] 9.1 Add the import path note to `nx-grammar.md` and `nx-grammar-spec.md`: a library path beginning `@nx/` names a standard library; verify both documents agree
- [x] 9.2 Update `sites/website/src/content/docs/reference/syntax/modules.md` and `language-tour/modules-and-imports.md`: the reserved `@nx/` root, standard libraries beside the prelude, the three import forms; verify the website build passes
- [x] 9.3 Add `sites/website/src/content/docs/reference/libraries/agent.md`: the unstable notice and what it permits, the library source, each type with its fields, the worked example, how a host library extends `Tool`, `ToolContext` and `Connection`, and what is left to the host; the Reference sidebar is generated from the directory, so no sidebar entry is written; verify the page builds and its code samples are compiled by a docs sample test
- [x] 9.4 Add the release-notes entry: standard libraries, `@nx/agent` (unstable), and the breaking reservation of the `@nx/` root; verify it appears in `src/vscode/CHANGELOG.md` for the next version (0.6.0). The draft GitHub release notes are generated boilerplate, so the package-side notes are written from this entry when the release is published
- [x] 9.5 Add a playground example that imports `@nx/agent` and evaluates an agent, with its expected output, if the playground's evaluator handles it after task 3.7; otherwise leave it out and say so in the change notes. Verify the playground's example check passes. Added as `agent`: it uses only the library's own tool types, which the playground's evaluator handles

## 10. Verification

- [x] 10.1 Run `cargo test --workspace`; all pass
- [x] 10.2 Run `pnpm build` and `pnpm test`; all pass, including the wasm and Node parity tests
- [x] 10.3 Run `pnpm run verify:packages`; all pass
- [x] 10.4 Run `openspec validate add-agent-library --strict`; it reports the change valid
- [x] 10.5 Hand the library's identity, version rule, TypeScript package target and type list to `add-agent-host-package` and to ReachMe's `add-agent-tool-loop`, and verify both changes name `@nx/agent`, `@nx/agent/agent.nx`, `Document.text` and abstract `ToolContext` the same way, that `@nx-lang/agent` exports the generated types, and that ReachMe does not try to load the library into its registry
