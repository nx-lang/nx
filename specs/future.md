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
