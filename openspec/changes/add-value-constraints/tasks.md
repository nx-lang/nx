## 1. Syntax

- [ ] 1.1 Add `where_clause` to the `type` rule in `crates/nx-syntax/grammar.js` (`where` and `value` contextual), parsing the supported terms as a restricted condition and anything else as a `value_expression` inside the clause; regenerate the parser and add corpus tests for fields with and without defaults, parameters, results, `let` annotations, aliases, negative bounds, and a field following a constrained field.
- [ ] 1.2 Update the highlighting queries (`crates/nx-syntax/queries`, editor grammars) for `where` and `value` in the clause.
- [ ] 1.3 Lower the clause into the HIR type reference (`nx-hir` `TypeRef`) as a list of terms with spans; keep unsupported conditions as an opaque term for the checker to report.

## 2. Type checker

- [ ] 2.1 Add a constrained form to `nx_types::Type` (base plus an interval with inclusive, exclusive or open ends) and resolve aliases into it, intersecting terms; report `invalid-constraint` and `empty-constraint` per the spec, with the `Score+` fix-it for occurrences.
- [ ] 2.2 Extend satisfaction and widening: constrained satisfies its base and wider types; interval containment between constrained types; arithmetic yields the base type; joins drop the constraint.
- [ ] 2.3 Check literals, signed literals, constant expressions and defaults against constrained sites and report `constraint-violation`; record every binding site where a runtime check is needed for the emitter.
- [ ] 2.4 Reject constrained types as type arguments and inside function types.
- [ ] 2.5 Tests in `nx-types` and `nx-api` for every scenario in `value-constraints`.

## 3. IR and emitter

- [ ] 3.1 Add type kind 7, node kind 28 and `constraints-v1` to `nx-ir` (model, image writer, reader validation, `explain`) and to `docs/nx-ir-format.md`.
- [ ] 3.2 Emit constrained types and check nodes in `nx-codegen` at exactly the sites the checker recorded; add IR tests including "no second check" for a parameter passed on.
- [ ] 3.3 Add conformance corpus programs: passing checks, inclusive and exclusive failures, `NaN`, host-boundary failures, nested record fields, defaults, operation counts.

## 4. Runtimes

- [ ] 4.1 Rust runtime: read kind 7, evaluate kind 28 (`nx-ir-constraint`), check host values against constrained types at the boundary (`nx-ir-boundary-constraint`, argument named), and count the check as one operation.
- [ ] 4.2 TypeScript runtime: the same, with `float32` rounding before comparison; run the corpus on both and compare.
- [ ] 4.3 `@nx-lang/agent`: add a test that a constraint failure maps to `invalid-input`.
- [ ] 4.4 Interpreter: accept constrained types (treat as base) so existing interpreter paths keep working.

## 5. Schemas and language service

- [ ] 5.1 Schema export: bound keywords, `int32` bound replacement, spelling with the constraint; extend the boundary-agreement corpus.
- [ ] 5.2 Hover and signature help show constraints, including through aliases.
- [ ] 5.3 Source tree: `bounds` on properties, state fields and aliases (Rust, wasm transport, `packages/language-protocol`).

## 6. Viewer

- [ ] 6.1 `packages/viewer`: intersect `bounds` with hints for clamping, stepping and range notes (`Above`/`Below` for exclusive ends), the derived float step, and refused choices; tests for every scenario.
- [ ] 6.2 Add constrained properties to the playground's "Visual test cases" example and check them in a browser.

## 7. Documentation

- [ ] 7.1 Website: a "Constrained types" section in `reference/syntax/types.md`, `where` in the style guide (name a constrained type used more than once), and update `reference/libraries/visualizer.md` to say how hints and constraints combine.
- [ ] 7.2 `nx-grammar.md` and `nx-grammar-spec.md`; `specs/future.md`: record the follow-ups (strings, sequence counts, named bounds, general predicates, integer overflow on the same check) and the interpreter gap.

## 8. Review

- [ ] 8.1 Run the review loop (`/openspecex-review`, `/openspecex-review-fix`, `/openspecex-review-verify`) until high-confidence findings are fixed and verified.
