## 1. Shared groundwork

- [x] 1.1 Move the doc-to-Markdown rendering (`typegen_doc` in `crates/nx-cli/src/typegen/model.rs`) to a method on `nx_hir::Doc`, with a companion that returns the summary block, and switch typegen to it; verify the existing typegen documentation tests pass unchanged
- [x] 1.2 Add `crates/nx-api/src/schema.rs` with the request and answer types (declaration reference, function options with host-supplied types, type options with direction, function answer, parameter entry, type answer), serializable with the property names in design.md D3; verify a unit test serializes an empty answer to the expected JSON
- [x] 1.3 Resolve a declaration reference (module identity defaulting to the entry, name) to a function or type in a `ProgramArtifact`, across root modules, linked library modules and compiler-carried library modules such as `@nx/agent/agent.nx`, including `<Target>.Update` and `<Target>.Property`; verify tests for an entry function, a non-exported function, a library function named `<root>/<module>`, and the `schema-unknown-declaration` diagnostic for a missing name and a missing module

## 2. Type mapping

- [x] 2.1 Implement the document builder: `$schema`, order-preserving objects, a `$defs` table keyed as the spec requires with `_2` suffixes on collision, and `$ref` for every nominal mention; verify golden tests for a recursive record, two same-named declarations from two modules, and a primitives-only function with no `$defs`
- [x] 2.2 Map primitives and `object`; verify a golden test over `string`, `boolean`, `int`, `int32`, `int64`, `float32`, `float64` and `object`
- [x] 2.3 Map occurrences: `+` and `*` arrays, a `?`-marked field or parameter as absence, and a standalone `T?` result as `anyOf` with `null`; verify golden tests for each, including an optional `+` field and an optional parameter that does not admit `null`
- [x] 2.4 Map concrete records and actions with effective fields (inherited first), `required`, `additionalProperties: false`, content fields, and names such as `aria-label` kept as written; verify golden tests for required, optional, defaulted and inherited fields
- [x] 2.5 Write `default` for string, numeric and boolean literals and constant-case references, and nothing for any other default; verify golden tests for each literal kind, a constant case, and a computed default
- [x] 2.6 Implement the two directions: `$type` always required in output; in input present and required only for abstract-record sites and payload cases, and one definition with `$type` required when a record is mentioned both ways; defaulted fields required in output only; verify golden tests for the same record in both directions and for the mixed-mention case
- [x] 2.7 Map unions: constant unions as string `enum`, payload cases as objects with a scoped `$type`, mixed unions as `anyOf`, a fieldless case of a union with a base as an object, and property unions as constant unions; verify golden tests for each
- [x] 2.8 Map an abstract-record site to `anyOf` its concrete descendant records and the cases of unions that extend it, across every module of the program in the specified order; verify golden tests with descendants in the entry module and in a library, and with a union extending the base
- [x] 2.9 Map applied generic records per instantiation with substituted arguments and `<Record>_<arg>` keys, and an unapplied generic record with the `object` schema for parameter-typed fields; verify golden tests for two instantiations of one record and for the unapplied form
- [x] 2.10 Map type aliases transparently and update records with every field optional and clearable fields nullable; verify golden tests for an alias to an occurrence type and for `User.Update`

## 3. Functions, documentation and diagnostics

- [x] 3.1 Build a function's input schema from its parameters (paren and element style, content parameter, `required`, defaults) and its output schema from the declared or inferred result type, with the parameter entries (including `typeRef` for a record- or union-typed parameter), `resultType` and the `declaration` span of the function in its module's source; verify golden tests for a paren function, an element-style function, a function with no parameters, an unannotated function, and a test that the `declaration` span covers the declaration's lines and is absent when the artifact holds no source for the module
- [x] 3.2 Attach descriptions: function and type documentation and summary on the answer, parameter documentation on the property and the entry, record, union and payload-case documentation on `$defs` entries, field documentation on properties including beside a `$ref` and on inherited fields, and the documented-constant-case list on a union; verify golden tests, a test that undocumented source yields no `description`, and a test that adding one doc comment changes only one `description`
- [x] 3.3 Report `schema-inexpressible-type` for a function type, a component used as a type, an abstract record with no descendant and an unresolved inferred result, at any depth, labeled at the member, collecting all of them, and answer the input and output schemas independently; verify tests for a function-typed parameter with a valid result, a function-typed field two levels down in a result, and two problems reported in one call
- [x] 3.4 Report `schema-ambiguous-discriminator` when two shapes at one site share a `$type`; verify a test with two modules each declaring `Card` extending one abstract record
- [x] 3.5 Implement host-supplied types: match a parameter whose declared type is a listed type or a record extending it, leave it out of the input schema, mark its entry, skip its expressibility check, ignore listed types the program does not declare, and do not match a containing type; verify tests for a direct match, a subtype match across a library boundary (a host library's `ChatToolContext` extending the real `@nx/agent` `ToolContext`), an unlisted run, a `+` of the type, an undeclared listed type, and `ToolContext` itself with no descendant in the program producing no diagnostic; also verify that a function returning the real `HttpArguments` gives `body` the `object` schema
- [x] 3.6 Report the function reference type `<function ... />: R` (from `add-function-reference-type`), as `schema-inexpressible-type`, distinct from `object`; verify tests for `type FunctionTool = { function: <function ... />: object* }`, for `type HttpTool = { arguments: <function ... />: Args }` and for `type Pair = { run: <function ... />: object* data:object }` naming only `Pair.run`, and that `let pick(): Tool` over the real `@nx/agent` reports `FunctionTool.function` and `HttpTool.arguments` with no output schema

## 4. Wasm SDK

- [x] 4.1 Add `nx_wasm_program_function_schema` and `nx_wasm_program_type_schema` to `bindings/wasm/native`, JSON in and JSON out, and bump `ABI_VERSION` and `abiVersion` by one; verify the native ABI test and that the loader refuses a module of the previous version naming both versions
- [x] 4.2 Add `functionSchema` and `typeSchema` to `NxProgramArtifact` in `bindings/wasm/src`, with the types from design.md D3 exported from the package, throwing `NxEvaluationError` for `schema-unknown-declaration` and following the disposed-resource and crashed-host rules; verify SDK tests for a documented function, a library function built against a registry, an inexpressible parameter returned as data, an unknown function, and a disposed artifact
- [x] 4.3 Add a schema corpus under `bindings/wasm/test/fixtures` with one NX source per mapping in the spec; verify each corpus function's schemas equal the Rust golden documents

## 5. Node SDK parity

- [x] 5.1 Add the two methods to the Node native binding and to `NxProgramArtifact` in `bindings/node/src`, with the same types and error behavior; verify Node SDK tests for a source artifact, a workspace artifact, and a directory-loaded library function
- [x] 5.2 Extend `bindings/wasm/test/parity.test.ts` to compare both bindings' function and type answers over the schema corpus as JSON text, including diagnostics for an inexpressible type; verify the parity test fails when one side's output is altered

## 6. Agreement with the runtime boundary

- [x] 6.1 Add Ajv (2020-12 build) as a dev dependency of `@nx-lang/sdk-wasm`; verify `pnpm install` and that no published package lists it under `dependencies`
- [x] 6.2 Add a test that, for every corpus function, validates canonical arguments and minimal arguments (every optional property omitted) against the input schema, calls the function with them through `callFunction` in `@nx-lang/ir-runtime` on the image emitted from the same artifact, and fails on any `nx-ir-boundary-type`, `nx-ir-boundary-field` or `nx-ir-arguments` diagnostic; verify it passes and that it fails when a schema is deliberately widened in a scratch run
- [x] 6.3 In the same test, validate every returned value against the output schema, covering a `null` optional result, an empty `T*`, an omitted optional field, a defaulted field, a constant case, a payload case, a concrete descendant at an abstract site and an applied generic record; verify it passes
- [x] 6.4 Add a test that an input-direction argument record with no `$type` is accepted and normalized by the runtime; verify it passes

## 7. Documentation and release

- [x] 7.1 Document the two queries in `bindings/wasm/README.md` (scope table, a section with an example, options, diagnostics) and correct its ABI section to the current version and exports; verify the README example runs as a test or matches one
- [x] 7.2 Document the queries in `bindings/node/README.md`; verify the method names and types match the wasm README
- [x] 7.3 Update `runtime/typescript/README.md` where it describes the prepared types to direct a host that needs types or documentation to derive schemas with a compiler SDK when it compiles and store them with the image; verify the paragraph names both SDK packages
- [x] 7.4 Add a website reference page under `sites/website/src/content/docs/reference/concepts/` giving the NX-to-JSON-Schema mapping table, the two directions, descriptions, the types with no JSON form and the `int64` precision note, and link it from the comments reference page; verify the site builds
- [x] 7.5 Run `cargo test`, `pnpm --filter @nx-lang/sdk-wasm test`, the Node SDK tests, `pnpm --filter @nx-lang/ir-runtime test` and `pnpm run verify:packages`; all pass
- [x] 7.6 Add the 0.6.0 entries to `src/vscode/CHANGELOG.md` for `///` doc comments (they landed after 0.5.0 with no entry) and for this export, and state 0.6.0 as the minimum version in both SDK READMEs; verify by installing the packed `@nx-lang/sdk-wasm` tarball in a scratch project and reading a `///` comment back as a `description`
- [x] 7.7 Run `openspec validate add-declaration-schema-export --strict`; it passes

## 8. Linking the subtypes a schema lists

- [x] 8.1 Add `program_artifact_boundary_subtype_modules` to `nx-api`, walking every function's parameter and result types through fields, cases, aliases, type arguments and the shapes extending an abstract record, and answering with the modules that declare those shapes; verify a test with a subtype in a module the entry imports but never references and one in a module nothing imports, one reached through a field, and a program with none
- [x] 8.2 List those modules in the entry image's module table after the modules it references, and in no other image; verify an IR test of the entry's and the other images' tables and that the existing IR tests and conformance corpus are unchanged
- [x] 8.3 Verify that both runtimes accept such a subtype from a host when the program is linked from its entry: a Rust IR runtime test and a case in the wasm SDK's agreement test
- [x] 8.4 Update `docs/nx-ir-format.md`'s module table section and the changelog, and run `openspec validate add-declaration-schema-export --strict`
- [x] 8.5 Keep both walks finite for a record that applies itself to a growing argument (the subtype walk by declaration, the schema by describing an instantiation that applies its record twice, or nests applied types more than four deep, as the record named with no arguments), and count only the functions of the modules the entry reaches through its imports; verify tests for `inner?:<Box T=<Box T=T /> />` and `inner?:<Box T=<Pair L=T R=T /> />` and two mutually recursive records in the schema, the walk and IR emission, an abstract record reached through a type argument, and a function nothing imports

