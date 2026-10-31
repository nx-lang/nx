## 1. Syntax

- [x] 1.1 Add range (`in a..b`, `in a..=b`) and comparison (`< <= > >=` literal) patterns, joined by `&&`, to `pattern` in `crates/nx-syntax/grammar.js`, and a `where_clause` (`where` contextual) holding one such pattern to the `type` rule; parse a leading operand or other expression in the clause for the checker to report. Regenerate the parser; corpus tests for fields with and without defaults, parameters, results, `let` annotations, aliases, negative bounds, a field after a constrained field, and match arms.
- [x] 1.1a End an unbraced match-arm body before a following arm's pattern (declared conflict resolved by `=>`), fixing the existing `5 -1 =>` misparse; tests for `-`, `<`, `>`, `<=`, `>=` after a body in value, element and property-list matches.
- [x] 1.1b Let a field, prop or parameter named `where` directly follow another field's type (the keyword, aliased as a name when `:` or `?:` follows); tests for records, component props and function parameters.
- [x] 1.2 Update the highlighting queries (`crates/nx-syntax/queries`, editor grammars) for `where` and the new patterns.
- [x] 1.3 Lower the clause into the HIR type reference (`nx-hir` `TypeRef`) as a list of terms with spans; keep unsupported conditions as an opaque term for the checker to report.

## 2. Type checker

- [x] 2.1 Add a constrained form to `nx_types::Type` (base plus an interval with inclusive, exclusive or open ends) and resolve aliases into it, intersecting terms; report `invalid-constraint` and `empty-constraint` per the spec, with the `Score+` fix-it for occurrences; reject `int64` bounds outside ±(2^53−1); normalize integer exclusive ends to inclusive.
- [x] 2.2 Extend satisfaction and widening: constrained satisfies its base and wider types; interval containment between constrained types; arithmetic yields the base type; joins take the narrowest type every item satisfies, else the unconstrained join of the bases; optional properties and `?`/`*` items of a constrained type.
- [x] 2.3 Check literals, signed literals, constant expressions and defaults against constrained sites and report `constraint-violation`; record every binding site where a runtime check is needed for the emitter.
- [x] 2.4 Reject constrained types as type arguments and inside function types.
- [x] 2.4a Type numeric match patterns: numeric scrutinee, bounds at its base type, `empty-constraint`, literal-match rules for `else`, and narrowing of an identifier scrutinee to the arm's interval, `int?` scrutinees, and the `unreachable-pattern` warning.
- [x] 2.5 Tests in `nx-types` and `nx-api` for every scenario in `value-constraints`.

## 3. IR and emitter

- [x] 3.1 Add type kind 7, node kinds 28 and 29 and `constraints-v1` to `nx-ir` (model, image writer, reader validation, `explain`) and to `docs/nx-ir-format.md`.
- [x] 3.2 Emit constrained types, check nodes (item-wise over sequences and optional values) and interval patterns in `nx-codegen` at exactly the sites the checker recorded, resolving constrained aliases from other modules, including interface-only ones (`builder.rs` keeps them as plain references today); bounds as integer or float constants, `float32` pre-rounded; add IR tests including "no second check" for a parameter passed on.
- [x] 3.3 Add conformance corpus programs: passing checks, inclusive and exclusive failures, `NaN`, host-boundary failures, nested record fields, defaults, operation counts.

## 4. Runtimes

- [x] 4.1 Rust runtime: read kind 7, evaluate kind 28, match kind 29 (`nx-ir-constraint`), check host values against constrained types at the boundary (arguments, props, state, patches, action records; `nx-ir-boundary-constraint`, argument named), and count one operation per value checked.
- [x] 4.2 TypeScript runtime: the same, with `float32` rounding before comparison; run the corpus on both and compare.
- [x] 4.3 `@nx-lang/agent`: add a test that a constraint failure maps to `invalid-input`.
- [x] 4.4 Interpreter: accept constrained types (treat as base) and evaluate numeric match patterns.

## 5. Schemas and language service

- [x] 5.1 Schema export: bound keywords, integer exclusive ends as inclusive, `float32` ends as inclusive `float32` values, `int32` bound replacement, spelling with the constraint; extend the boundary-agreement corpus.
- [x] 5.2 Hover and signature help show constraints, including through aliases.
- [x] 5.3 Source tree: `bounds` on properties, state fields, union-case fields, aliases and annotated value declarations, and for sequence and optional item types (Rust, wasm transport, `packages/language-protocol`).

## 6. Viewer

- [x] 6.0 `packages/viewer/src/edit.ts`: recognise a constrained numeric type (spelled with `where`) as a number, through aliases too, wherever `NUMBER_TYPES` decides today.
- [x] 6.0a `visualizer-declarations` hint fit: treat a constrained numeric type (inline or alias) as its base, so numeric hints raise no `editor-hint-mismatch`; tests for both scenarios.
- [x] 6.1 `packages/viewer`: intersect `bounds` with hints for clamping, stepping and range notes (`Above`/`Below` for exclusive ends), the derived float step, ignored non-overlapping hints, no step onto an exclusive end, and refused choices; tests for every scenario.
- [x] 6.2 Add constrained properties to the playground's "Visual test cases" example and check them in a browser.

## 7. Documentation

- [x] 7.1 Website: a "Constrained types" section in `reference/syntax/types.md`; range and comparison patterns in `reference/syntax/if.md`; `where` in the style guide (name a constrained type used more than once, write float bounds as `0.0..=1.0`); constraint keywords in `reference/concepts/declaration-schemas.md`; and in `reference/libraries/visualizer.md` and the `@nx/visualizer` `min`/`max` doc comments, how hints and constraints combine ("Not a constraint" stays true of hints).
- [x] 7.2 `nx-grammar.md` and `nx-grammar-spec.md`; `specs/future.md`: record the follow-ups (strings, sequence counts, named bounds, general predicates, integer overflow on the same check) and the interpreter gap.

## 8. Review

- [ ] 8.1 Run the review loop (`/openspecex-review`, `/openspecex-review-fix`, `/openspecex-review-verify`) until high-confidence findings are fixed and verified.
