## 1. Syntax

- [x] 1.1 Add `function_type` and `parenthesized_type` as bases of `type` in `crates/nx-syntax/grammar.js` with the `'function'` literal, add `FUNCTION_TYPE` and `PARENTHESIZED_TYPE` to `syntax_kind.rs`, regenerate the parser, and verify `tests/fixtures/valid/function-types.nx` (inline prop, alias, no-param, content param, `(...)?`, `(...)[]`, multi-line) parses with no error nodes in an `insta` snapshot
- [x] 1.2 Extend `validation.rs`: reject a default and a second `content` in a function type, add the `FUNCTION_TYPE` arm to `validate_type_parameter_definition`, and handle the duplicate-nullable case across `(` `)`; verify `tests/fixtures/invalid/` cases for each produce the diagnostics the `function-types` and `type-reference-suffixes` specs name
- [x] 1.3 Verify `let function = 1 let v = {function}` still parses and checks, and that `Name: <functon />` and an unclosed `<function` yield a single clear error node each (fixture + snapshot)
- [x] 1.4 Update `queries/highlights.scm` to capture the `function` keyword, parameters and result of a function type and the parentheses of a parenthesized type; verify with `query_tests.rs`

## 2. HIR

- [x] 2.1 Change `ast::TypeRef::Function` to named parameters (`FunctionParam { name, ty, is_content }`), lower `FUNCTION_TYPE` and `PARENTHESIZED_TYPE` in `lower_type`, and verify a lowering test shows the alias `type T = (<function Item:Contact Index:int />: DrawnNode)?` as nullable-of-function with both names
- [x] 2.2 Recurse into `TypeRef::Function` in `erase_type_parameters` and verify the existing erasure test extended with a function-typed prop yields `object` inside the function type
- [x] 2.3 Make `PreparedItemKind::Function` visible in the `Value` namespace as well as `Element`, report a function and a `let` value of one name as a duplicate declaration, and verify prepared-module tests for both

## 3. Type checker

- [x] 3.1 Change `Type::Function` to named parameters, update every producer (`bind_function_signature_from_parts`, imported signatures) and consumer, and verify `cargo test -p nx-types` still passes
- [x] 3.2 Rewrite function compatibility in `is_compatible_with` by name with the subset rule, contravariant parameters, content pairing and covariant result; verify unit tests for each `function-types` "satisfies a function type" scenario, including the rejection diagnostics naming the parameter
- [x] 3.3 Render `Type::Function` in NX spelling (parenthesized under a suffix) in `qualified_display`/`write_postfix_type`; verify a display test and that no diagnostic prints `=>`
- [x] 3.4 In `infer_element_expression`, resolve a tag that is a lexical function-typed binding to a call checked against the type's parameters (all required, unknown rejected, content to content) returning `ret`; verify the `function-values` element-invocation scenarios including "A non-function binding is not a tag"
- [x] 3.5 Reject a paren call whose callee is a lexical function-typed binding with a diagnostic showing `<f n=... />`; verify with an analysis test
- [x] 3.6 Verify substitution of a use-site type argument reaches a function-typed prop and that an unbound parameter reports "TItem was not specified" with `TItem=`, per the `component-type-parameters` delta scenarios
- [x] 3.7 Add `crates/nx-types` tests for the "function name is a value" scenarios: bound to a function-typed prop, forwarded through a component, paren function as value, mismatch against `string`, lexical shadowing

## 4. Interpreter and API

- [x] 4.1 Add `Value::Function { module, name }`, resolve an undefined variable that names a function item to it, and define equality; verify an interpreter test evaluating `<List ItemTemplate={Row} />` yields the function value in the field
- [x] 4.2 In `eval_element_expr`, treat a tag bound to a `Value::Function` in scope as a call: bind fields by name, drop undeclared, error on a missing declared parameter; verify the "invoked as an element", "ignores a parameter the type supplied" and paren-function-as-element scenarios evaluate to the expected values
- [x] 4.3 Render `Value::Function` in `nx-api` as `{ $type: "Function", module, name }` and refuse it in `from_nx_value`; verify serialization and round-trip-refusal tests beside the `ActionHandler` ones
- [x] 4.4 Run `cargo test --workspace` and the FFI/.NET smoke tests to verify nothing that passes images through has changed behavior

## 5. IR and codegen

- [x] 5.1 Add type kind `FUNCTION = 4` to `ir.rs` with the `[4, result, count, name, type, flags…]` layout, emit it from `CodegenTypeRef::Function` (deleting the top-type stand-in), intern it, and verify an `ir_tests.rs` shape test over explained text shows `(<function Item:object Index:int />: string)?` for a nullable function-typed prop and one table entry for two identical types
- [x] 5.2 Emit a `reference` node for `Expr::Ident` naming a function in `builder.rs`, add node kind 21 `namedCall` for an element invocation of a function-typed binding, add the `function-values-v1` feature when either (or a function type) is present, and verify explained-text tests for both nodes and for the feature list being unchanged in a program without them
- [x] 5.3 Validate the new type kind and node kind in `ir_image.rs` (indices in range) and render them in `ir_explain.rs`; verify the damaged-cell and truncation tests still pass with a fixture containing both
- [x] 5.4 Add a conformance program `specs/ir-conformance/function-values/` that declares a function type, binds a function to a prop and to a parameter, invokes the parameter as an element (exact and subset), renders a function value in a result, and record results through `nx-api`; regenerate any other corpus image whose tables change and review the explained-text diff
- [x] 5.5 Map a function type in `crates/nx-cli/src/typegen` to `(args: { Name: Type; … }) => Result` and verify a typegen test

## 6. TypeScript runtime

- [x] 6.1 Add the function type kind, `namedCall` and `function-values-v1` to the reader and validators in `runtime/typescript/src/index.ts`; verify a module listing the feature is refused by a runtime build with it removed (test toggles the feature set) and accepted otherwise
- [x] 6.2 Canonicalize `functionReference` as `{ $type: "Function", module, name }` in rendered output and descriptors; verify `runtime.test.ts` shows the record for `evaluateFunction(program, "root")` with `ItemTemplate={Row}`
- [x] 6.3 Evaluate `namedCall` by name with the subset rule and export `callFunction(program, record, args)` with the same binding, missing-parameter diagnostic and dropped extras; verify tests for the invocation, the drop and the missing-parameter failure
- [x] 6.4 Accept a host-supplied `Function` record at a function-typed prop in descriptor construction and `initializeComponent` (resolving module and name, failing by name otherwise); verify the "reaches a child instance" and "names no declaration" scenarios
- [x] 6.5 Run `test/corpus.test.mjs` and `test/emitted-ir.test.mjs` to verify the new corpus program passes in the TypeScript runtime with results identical to the Rust-recorded ones

## 7. Catalog and playground

- [x] 7.1 Extend `scripts/generate-catalog.mjs` to detect the `ItemsSource`/`ItemTemplate` pair, emit `TItem:type`, `ItemsSource:TItem[]?`, `ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)?`, drop both from `omitted.json` and record `templates` in `catalog-meta.json`; regenerate and verify `skia.nx` compiles (`scripts/compile-example.mjs`) and the summary line reports the templated control
- [x] 7.2 Export `Registry`/`applyProps` from `reconciler.ts`, add `src/render/materialize.ts` that builds host controls from an NX value (elements, children, authored components via `initializeComponent`, handlers skipped and reported inert), and verify a unit test materializes a nested layout and a component descriptor
- [x] 7.3 Add `src/render/templateCell.ts` (`NxTemplateCell` overriding `OnBindingContextChanged` to `callFunction` with `Item`/`Index`, rebuild children, report failures by index) and make `coerceProps` turn a `Function` record on a `templates` prop into its factory while passing `ItemsSource` through uncoerced; verify a unit test that binds a cell to an item and to a second item and checks the drawn content each time, and one where the function fails and the cell stays empty with a diagnostic
- [x] 7.4 Route template-call failures and inert-handler notices into the diagnostics pane via `useNxDrawing`; verify by a failing template in the editor showing the index
- [x] 7.5 Port `cells.nx` and `uneven-cells.nx`: element-function cells, collections built with nested `for` over digits at the originals' item counts, `TItem=` with `ItemsSource`/`ItemTemplate` and the originals' recycling and measuring settings; update `examples.json` coverage (`static` naming code-behind if the jump toolbar stays out, never `reduced`) and drop list virtualization from the capability vocabulary in `types.ts`; verify `scripts/check-examples.mjs` passes and both examples scroll with recycled cells in the browser (Playwright screenshot after scrolling)
- [x] 7.6 Measure evaluation time of the 100 000-item collection in the browser; if it exceeds a second, reduce to 10 000 and say so in the example note (record the measurement in the PR)
- [x] 7.7 Update `sites/playground/docs/CATALOG.md` (templated controls, `Item`/`Index` divergence, `ItemTemplate` leaves the omitted list) and `sites/playground/README.md`

## 8. Editors and docs

- [x] 8.1 Render function types in NX spelling in `crates/nx-language-service` hover and verify the two new hover scenarios in the `editor-language-service` delta plus the reworded `function` assertion
- [x] 8.2 Add function-type and parenthesized-type rules to `src/vscode/syntaxes/nx.tmLanguage.json` (and the markdown code-block grammar) with the scopes the `editor-syntax-highlighting` delta names; verify with the grammar's scope tests including `let function = 1`
- [x] 8.3 Update `nx-grammar.md` and `nx-grammar-spec.md` (Type productions, AST mapping, validation rules) and the language reference `types.md` (replace the arrow-form "Function Types" section) and `functions.md` (functions as values, invoking a function-typed value, the subset rule); verify the doc examples compile with `nxlang`
- [x] 8.4 Replace `examples/nx/template-candidate.nx` with `examples/nx/templates.nx` (aliases, multi-template `DataTable`, forwarding component, a template drawn as a plain child) and verify `nxlang run` evaluates it
- [x] 8.5 Update `docs/nx-ir-format.md` and `runtime/typescript/README.md` for the type kind, `namedCall`, the feature and `callFunction`; verify `cargo test --workspace`, `npm test` in `runtime/typescript`, and the playground test suite all pass
- [x] 8.6 Map a function type to `NxLang.Nx.NxFunctionRef` in `crates/nx-cli/src/typegen/languages/csharp.rs` and add that DTO to `bindings/dotnet/src/NxLang.Sdk` beside `NxActionHandlerRef` (the `System.Delegate` fallback serializes in neither supported format); verify the `dotnet-binding` delta's scenarios with `NxFunctionValueTests` and the typegen test, and `dotnet test bindings/dotnet/NxLang.sln`
