# Future Considerations

## Numeric Width Semantics

The type system supports `i32`, `i64`/`int`, `f32`, `f64`/`float` but there are
several open questions about how width should behave at runtime.

### Interpreter does not produce 32-bit values from source

The interpreter always produces `Value::Int` (i64) and `Value::Float` (f64) for
numeric literals. `Value::Int32` and `Value::Float32` only appear when injected
by FFI or host code. This means `let x: i32 = 42` produces a 64-bit value at
runtime.

Options to address:
- **Type-directed literal narrowing**: thread the expected type into literal
  evaluation so `let x: i32 = 42` produces `Value::Int32(42)`.
- **Coercion at boundaries**: narrow values at `let` bindings and function call
  sites when the target type is known.
- **Keep as-is**: treat `i32`/`f32` as FFI/serialization hints only, with the
  runtime always using 64-bit internally.

### Overflow semantics are undefined

If 32-bit values are produced at runtime, overflow behavior needs to be defined.
For example, `let x: i32 = 3000000000` — should this be a runtime error,
wrapping, or a compile-time error?

Options:
- **Runtime error** (safest, matches Rust debug / C# checked)
- **Wrapping** (matches C / Rust release)
- **Compile-time rejection** (requires constant evaluation)

### Type compatibility is widening-only at the type level but not enforced directionally

`Type::is_compatible_with` treats any integer width as compatible with any other
(same for floats). This means `i64 → i32` is implicitly allowed in argument
passing and assignment. If width should be enforced, this needs to be split into
directional "assignable" (widening only: i32 → i64 ok, i64 → i32 error) vs
"comparable" (either direction) checks.

### FFI boundary validation

Even without full runtime 32-bit support, FFI calls should validate that values
fit in the target width. Passing `i64::MAX` to a C# `int` parameter is silent
data corruption. A narrowing check at FFI call sites (in nx-ffi) would catch
this without adding complexity to the core interpreter.

## Braced List Minus Ambiguity

Braced value lists currently require prefix-unary expressions to be
parenthesized, which avoids ambiguity between list items and subtraction. This
may be worth revisiting for negative numeric literals, since users may expect
forms like `{-2 3}` to work naturally.

If this is revisited in the future:
- Consider allowing signed numeric literals as list-safe atoms only under
  constrained conditions, rather than allowing all prefix-unary expressions as
  bare list items.
- Do not let whitespace alone change `3-2` or `3 - 2` from subtraction into a
  list split.
- Consider a targeted warning or error for suspicious forms like `{3  -2}` to
  reduce confusion, since users may read that as subtraction written with
  uneven spacing and a binary minus operator normally should not have a space
  before it and no space after it.

## Type Inference HIR Clone Cleanup

`nx-types::infer` currently clones some HIR nodes to satisfy borrow-checker
constraints during inference. The most visible case is element inference, where
the code clones an `nx_hir::Element` before calling the helper that needs
`&mut self`, but similar clone-through-lookup patterns also exist for function
and record definitions.

This is not currently a correctness issue, and the element clone is shallower
than it first appears because `Element.children` stores `ExprId`s rather than
recursive child AST nodes. That makes this more of a cleanup and allocation
reduction opportunity than an urgent performance problem.

If this is revisited in the future:
- Treat it as a broader "stop cloning HIR during inference" refactor rather
  than a one-off fix for element expressions.
- Consider reshaping element inference around `ElementId` or other short-lived
  module lookups so `InferenceContext` can borrow the module briefly without
  cloning full structs.
- Review nearby definition resolution helpers at the same time, since function
  and record inference currently clone their definitions for similar reasons.
- Prioritize this work if profiling or editor latency shows element-heavy files
  spending meaningful time in inference; otherwise keep it as low-priority
  cleanup.

## Multi-File And Incremental Source Analysis

The shared source-analysis pipeline for `nx-types` and `nx-api` is now in
place, including path-aware import resolution and analyze-then-execute runtime
gating. The main work left in this area is broader compilation architecture,
not the single-source pipeline itself.

If this is revisited in the future:
- Extend the analysis model beyond single-source entry points so multi-file
  diagnostics can be computed and surfaced as one coherent result.
- Add caching or incremental compilation so repeated source-driven API calls do
  not always reparse, relower, rebuild scopes, and re-run type inference from
  scratch.
- Decide whether the public analysis API should grow a reusable session or
  project-oriented abstraction, rather than remaining string/file helpers only.
- Keep the current analyze-then-execute contract intact while expanding the
  implementation, so runtime-only validation still happens only after static
  analysis succeeds.

## Manifest-Rooted Packages

NX currently has an asymmetric source organization model: libraries are
directory-rooted collections of modules, while programs are still built
primarily from a single source entry point. A future packaging design could
unify those concepts around an explicit package manifest. This is also the
underlying architecture gap behind RF2 in the
`update-declaration-visibility-keywords` review: non-library programs do not
yet have a first-class multi-module package model, so whole-program visibility
across peer modules has no clear implementation path.

If this is revisited in the future:
- Introduce a declarative `package.nx` file at the root of each package rather
  than relying on separate manifest conventions for libraries and executable
  packages.
- Use `kind: app` and `kind: library` in `package.nx` to distinguish
  executable and reusable packages while preserving one shared package model.
- Build multi-module analysis, dependency resolution, and runtime entrypoint
  selection around packages so app packages and library packages follow the
  same root-level metadata and module discovery rules.
- Resolve the RF2-style case by making peer-module visibility within an app
  package an explicit package-level behavior rather than an accidental
  extension of the current single-source program artifact model.
