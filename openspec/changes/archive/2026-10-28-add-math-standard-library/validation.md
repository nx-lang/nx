# Implementation validation

The implementation uses Rust and TypeScript IR runtimes, executable TypeScript/JavaScript, and
.NET through the existing native Rust SDK binding. No independent C# runtime or executable C#
backend is introduced. The proposal's earlier assumption that a C# executable backend existed
was corrected during implementation, with the user's authorization.

Completed checks:

- Rust regression run: 1,495 passing tests across 42 suites. The final compiler/API/interpreter
  regression rerun also passes, as do the final focused math and generated-source checks.
- `cargo check -q --workspace` and `cargo fmt --all -- --check` pass.

- Shared independent numerical corpus: 87 cases through Rust primitives, Rust IR, legacy Rust,
  TypeScript IR, generated TypeScript and generated JavaScript. Includes sign-bit checks, domains,
  non-finite host arguments, overflow and large/tiny inputs. References come from mpmath 1.3.0
  at 500 decimal digits; the generator and provenance are checked in.
- Standard-library imports, trusted identities, reserved-root rejection, lazy loading, aliases,
  selective imports, ordinary passed/returned function values, schema signatures and editor hover.
- Required-feature/operation/arity/reference validation, authored call origins, operation budgets,
  generated diagnostic categories and existing indirect-codegen refusal.
- `pnpm --dir runtime/typescript test` and the updated persisted-IR corpus runner pass.
- Node native SDK math evaluation and image emission pass; Rust/WASM evaluation and emitted-image
  linking pass; the .NET SDK math evaluation and image-emission test passes through Rust FFI.
- Website build/link validation passes; all 11 NX blocks in the three touched documentation pages
  compile through the updated Rust/WASM compiler. The geometry example is checked in Rust and
  TypeScript without any graphics renderer.
- Existing math-free binary fixtures remain byte-identical. New math fixtures cover node kind 26
  and all 22 operation IDs. The corpus size budget counts typed runtime descriptors as compiler
  input alongside the source constants, using the same six-times budget.
- Both math and graphics OpenSpec changes pass strict validation; `git diff --check` passes.

Commands include:

```sh
cargo test -q -p nx-value -p nx-ir -p nx-ir-runtime -p nx-api -p nx-codegen -p nx-interpreter
cargo test -q -p nx-language-service hover_describes_standard_math_signature
cargo check -q --workspace
pnpm --dir runtime/typescript test
pnpm --dir bindings/node build
pnpm --dir bindings/node exec vitest run test/sdk-node.test.ts -t 'standard math'
pnpm --dir bindings/wasm build
pnpm --dir bindings/wasm typecheck
pnpm --dir bindings/wasm exec vitest run test/standard-libraries.test.ts -t 'standard math'
cargo build -q -p nx-ffi
dotnet test bindings/dotnet/NxLang.sln -p:NxSdkNativeLibraryConfiguration=Debug --filter FullyQualifiedName~MathUsesNativeRustImplementation --nologo
pnpm --dir sites/website build
node sites/website/scripts/check-code-blocks.mjs sites/website/src/content/docs/reference/libraries/math.md sites/website/src/content/docs/language-tour/functions.md sites/website/src/content/docs/language-tour/modules-and-imports.md
openspec validate add-math-standard-library --strict
openspec validate add-nx-graphics-core-and-direct-skia --strict
```

## Claude review fixes

An independent Claude CLI review identified RF1–RF5; fixes and verification are tracked in review.md.
The corpus now contains 184 cases, including all argument positions for non-finite inputs, named
inverse-trig/clamp boundaries, inexact and negative-t interpolation, and representable extrapolation
whose unscaled intermediate products overflow. Runtime declarations now follow module registry
metadata, format/runtime arities are cross-checked, and generated helper transformations fail loudly
on source drift, with Rust checks for generated JavaScript syntax and domain-error categories.

The TypeScript full suite, registry-metadata test, website build/link validation, changed documentation
examples, strict OpenSpec validation, formatting and diff checks pass after these fixes.

The broad post-fix Rust regression run passes: 1,497 tests across 42 suites, plus the newly added
registry-metadata test run separately after that build. Math-free IR conformance images remain
unchanged.

The same Claude CLI reviewer independently verified RF1–RF5: five verified, zero reopened, zero
new findings, and no remaining questions. Verification also reproduced the corpus generator output
and compared Rust/TypeScript lerp over 200,000 cases. See review.md for commands and scope.
