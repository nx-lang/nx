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

## Source Analysis Pipeline For nx-api

`nx-api` currently exposes source-driven runtime entry points such as
`eval_source`, `initialize_component_source`, and
`dispatch_component_actions_source`. Those helpers currently parse and lower
source, then either return early on lowering diagnostics or proceed directly to
interpreter execution.

This keeps the runtime path simple, but it also means `nx-api` is not using the
same full analysis pipeline as `nx-types::check_str`. As a result, API callers
can miss downstream scope/type diagnostics whenever lowering already produced an
error.

If this is revisited in the future:
- Add a shared "analyze source" entry point that owns parse, lowering, scope
  building, and type checking, rather than extending `lower_source_module` to
  do more than lowering.
- Put that shared analysis entry point at the analysis boundary, ideally beside
  `nx-types::check_str`, so `nx-api` can reuse it without duplicating compiler
  pipeline logic.
- Keep runtime execution as a second phase: if static analysis returns any
  error diagnostics, return them all and do not interpret.
- Preserve the lowered `Module` from the analysis result so `nx-api` does not
  need to reparse or relower before interpretation.
- Keep file-name and span fidelity intact in the shared path; do not reuse
  helper layers that discard the caller-provided `file_name` in diagnostics.
- Narrow or remove `lower_source_module` afterward so its name once again means
  true parse/lower work rather than a partial analysis pipeline.
