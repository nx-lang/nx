# Future Considerations

## Numeric Width Semantics

The type system supports `int`, `int32`, `int64`, `float32`, `float64` but there are
several open questions about how width should behave at runtime.

`int` is the default integer type and is specified as exact over ±(2^53−1) on every
backend — the widest range that C# `long`, JavaScript `number`, and Rust `i64` all
represent exactly. Enforcing that range is deferred; see "Bounds checks are specified
but not enforced" below.

### Interpreter does not produce 32-bit values from source

The interpreter always produces `Value::Int` (`int`) and `Value::Float` (`float64`) for
numeric literals. `Value::Int32` and `Value::Float32` only appear when injected
by FFI or host code. This means `let x: int32 = 42` produces a 64-bit value at
runtime.

`int64` has no distinct runtime carrier either — it also evaluates to `Value::Int`, so
`Value::type_name` reports `int` for both. The two separate when `int64` gets its
checked, bigint-backed representation.

Options to address:
- **Type-directed literal narrowing**: thread the expected type into literal
  evaluation so `let x: int32 = 42` produces `Value::Int32(42)`.
- **Coercion at boundaries**: narrow values at `let` bindings and function call
  sites when the target type is known. Note that coercion is currently check-only —
  `coerce_non_record_value` returns a value unchanged or errors, and never converts.
- **Keep as-is**: treat `int32`/`float32` as FFI/serialization hints only, with the
  runtime always using 64-bit internally.

The first two options are the "value-directed" design costed under "Bounds checks are
specified but not enforced" below; making runtime values width-correct and enforcing
bounds are largely the same piece of work.

### Bounds checks are specified but not enforced

`int` is specified as exact over ±(2^53−1) on every backend, and arithmetic is
specified as checked rather than wrapping. Neither is enforced yet:
`crates/nx-interpreter/src/eval/arithmetic.rs` still uses `wrapping_add`,
`wrapping_sub`, and `wrapping_mul` for every integer type, so the specified range is
currently a documentation-level guarantee.

Enforcing it is deliberately deferred. It most likely wants to be implemented
together with user-declared integer ranges (`1..10`), since both need the same
`check_range(value, lo, hi)` primitive and the same runtime error — one bounds-check
mechanism, not two. Whether the two actually land as a single change is undecided;
the implementation notes below apply either way.

Measured cost on Node v24 for the JavaScript backend: an unchecked add is ~0.74 ns/op
and a `Number.isSafeInteger`-guarded add is ~2.51 ns/op. `Number.isSafeInteger` is a
V8 intrinsic and beats a hand-written comparison (1.03 ns vs 1.35 ns).

The same question remains open for the narrow types: `let x: int32 = 3000000000`
should be a runtime error, wrapping, or a compile-time error.

Options:
- **Runtime error** (safest, matches Rust debug / C# checked)
- **Wrapping** (matches C / Rust release)
- **Compile-time rejection** (requires constant evaluation)

#### JavaScript runtime: the plumbing already exists

The TypeScript IR runtime is close to ready. Every IR expression already carries its
inferred type (`crates/nx-codegen/src/ir.rs` sets `ty` on each emitted expression),
and `evalDivision` and `evalModulo` in `runtime/typescript/src/index.ts` already
consume it through `isIntegerSemanticType`. What is missing is that `add`, `sub`, and
`mul` in `evalBinary` ignore `expression.ty`, and `evalUnary` never receives it.

Work needed: a range table and a `checkRange` helper alongside the existing
`checkedInteger`, wired into the five arithmetic binary operators; `ty` threaded into
`evalUnary` for `neg` (one call site); and a runtime diagnostic code for overflow.

`int64` cannot be range-checked on this backend while it is carried as a `number` —
its specified range is not representable. See "`int64` is still a JavaScript
`number`" below.

#### Interpreter: the types are computed and then discarded

The interpreter cannot distinguish `int` from `int64` at an arithmetic site, because
it has no per-expression type information at all. That information does exist — it is
dropped just before the interpreter would receive it:

- `TypeEnvironment` holds `expr_types: FxHashMap<ExprId, Arc<Type>>`
  (`crates/nx-types/src/env.rs`), populated by inference through
  `Primitive::numeric_promotion`, so binary nodes already carry the correctly
  promoted integer type.
- Every `ModuleArtifact` carries that environment (`crates/nx-types/src/check.rs`).
- `build_resolved_program` (`crates/nx-api/src/artifacts.rs`) walks
  `&[ModuleArtifact]`, keeps `artifact.lowered_module`, and discards
  `artifact.type_env` — the field directly beside it.

Two designs, in increasing cost.

**Type-directed** (roughly 1–1.5 days). Route the existing types through to
evaluation:
- `ResolvedModule` gains the expression-type map beside `lowered_module`.
- `build_resolved_program` passes `artifact.type_env` instead of discarding it.
- `Interpreter` keys the environments by `SourceId`, so `eval_expr` can look one up
  from the `module` it already holds — no module-id threading through eval
  signatures.
- `eval_binary_op` gains its own `ExprId`; it currently receives only `lhs` and
  `rhs`. Its single call site in `eval_expr` already has it. Same for `neg`.
- `arithmetic.rs` takes bounds and uses `checked_*` plus a range check.

This mirrors how the JavaScript runtime already works — static type on the expression
node — which keeps the two runtimes structurally aligned. It needs no new `Value`
variant, so it avoids exhaustive-match churn and leaves existing `Value::Int(...)`
test expectations valid.

Two decisions it forces:
- **Missing type entries.** Defaulting to the `int` bounds when `get_expr_type`
  returns `None` fails safe. Defaulting to "unchecked" would silently skip checks,
  which is the worse failure mode.
- **Direct-HIR paths have no inference.** `ResolvedProgram::single_root_module` and
  the `interpreter_direct_hir.rs` tests build programs with no type environment, so
  overflow tests must run through the real analysis pipeline.

Its limit: this makes *expressions* width-correct, not *values*. `Value::Int` still
cannot distinguish an `int` from an `int64` inside a record field or array element,
and the FFI boundary cannot either — `NxValue` has `Int32` and `Int(i64)` but no
`Int64`. That is sufficient for arithmetic, where the declared type is known at every
operation.

**Value-directed** (roughly 3–4 days). Add `Value::Int64` and make coercion convert
rather than check. This is full width correctness, and it is where the cost lives:
- Three mirror enums, not one: `Value`, `NxValue`, and the private `SerializedValue`
  in `interpreter.rs` — plus their serde, and the .NET and Node bindings that
  deserialize `NxValue` JSON.
- Roughly 50 non-test match sites.
- `coerce_non_record_value` currently returns a value unchanged or errors. Making it
  convert changes semantics at every typed boundary — parameters, returns, fields,
  array elements — and moves test expectations. This part is not mechanical.

The value-directed work is best done with the `int64`-as-`bigint` change rather than
before it: a JavaScript `bigint` crossing FFI needs a distinct Rust carrier, so both
are solving the same boundary problem and would otherwise design it twice.

Also fixed by this work: `interpreter.rs` negates with `Value::Int32(-n)`, which
panics in debug builds on `i32::MIN`. It is reachable only through an FFI-supplied
`int32`, since evaluation never produces `Value::Int32` from source.

### `int64` is still a JavaScript `number`

`int64` is specified as a full 64-bit signed integer, but the TypeScript backend
still emits it as `number`, which is exact only to 2^53−1. Carrying it as `bigint`
(with `BigInt64Array` for arrays, following Kotlin/JS 2.2.20) is the intended
direction and is deferred to its own change. Note that `JSON.stringify` throws on a
`bigint`, so the IR's existing string encoding for large integer literals stays
mandatory.

### Type compatibility is widening-only at the type level but not enforced directionally

`Type::is_compatible_with` treats any integer width as compatible with any other
(same for floats). This means `int64 → int32` is implicitly allowed in argument
passing and assignment. If width should be enforced, this needs to be split into
directional "assignable" (widening only: int32 → int → int64 ok, the reverse an
error) vs "comparable" (either direction) checks.

`Primitive::numeric_promotion` already encodes the rank order int32 < int < int64,
so the widening direction is defined even though compatibility does not enforce it.

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

## Brace Recovery Reports A Closed Brace As Unclosed

Admitting the empty list (`empty-list-spelling`) made `{` immediately followed by `}` a valid parse.
That is correct for valid source, but it changed the path error recovery takes through *invalid*
source, and one of the new paths reports a brace that is closed as unclosed:

```
error src/vscode/samples/tally-survey.nx:31:22: Unclosed brace
   31 |         if allowBoth {
      |                      ^ unexpected syntax here
note: Add a closing '}' to match the opening brace
```

The brace on line 31 is closed on line 33. The real error is earlier and unrelated — the file uses
an unsupported positional attribute form, `<Option "Yes, borrowed"/>` — and the file failed to parse
both before and after the change. But recovery now cascades further from it: that one file went from
21 diagnostics to 35, which is the whole of the repository corpus's 225 → 239. No valid program is
affected, and every one of the repository's other 110 `.nx` files produces byte-identical output.

The diagnostic is wrong about the thing it points at, which is worse than reporting less. An author
whose file has one real error is told to close a brace that is already closed, and the true error is
buried in the cascade.

If this is revisited in the future:
- Treat it as an error-recovery problem, not a grammar problem. The grammar change is correct and
  the conflict sets are unchanged; what regressed is which recovery branch the parser reaches once
  `{}` is a legal shape.
- Measure with a whole-corpus before/after diagnostic diff rather than the test suite. The suite
  stayed green through this; only running every `.nx` file against a baseline binary surfaced it.
- Prefer suppressing cascaded diagnostics after the first hard parse error in a region over
  special-casing the brace rule. The count going 21 → 35 is the signal: recovery is re-entering and
  re-reporting, not finding 14 new distinct problems.
- Fix the sample. `src/vscode/samples/tally-survey.nx` uses positional element attributes that NX
  does not support, so it has never parsed. It is a fixture for the TextMate grammar, and the same
  file is called out under "TextMate Grammar: The Bare-Identifier Catch-All" for 6 invalid prose
  lines. Making it valid removes the only file in the corpus that exercises this path.


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

## TextMate Grammar: The Bare-Identifier Catch-All

`src/vscode/syntaxes/nx.tmLanguage.json` ends its `#qualifiers` rule with a catch-all
that matches any identifier or dotted name and scopes it `entity.name.qualifier.nx`.
That scope means "the module qualifier in `Foo.bar`", but the pattern claims far more
than that. A variable reference in an expression — `ready` in `if ready { … }`,
`total` in `total + 1`, `item` in `x={item}` — lands on it, so a variable is painted
as a qualifier. No default theme styles `entity.name.qualifier`, so all of it renders
as plain foreground and the mis-scoping is invisible rather than ugly.

The obvious fix — repoint the catch-all at `variable.other.readwrite.nx` — is wrong,
because the catch-all is load-bearing in four unrelated places at once. Across the
repository's 19 `.nx` files it claims 139 bare identifiers and 18 dotted names, and
the bare ones are not all variables:

- **Genuine variable references** (120 of the 139). These do want a variable scope.
- **Imported names** (2 tokens today, but every import in every future file). There is
  no import rule in the grammar at all. `import { UiCommon } from "../ui"`
  (`docs/drawnui-proposal/graphics/graphics.nx:17`) tokenizes by
  accident: `import`/`from`/`as` match `#keywords-core`, the braces become a
  `values-braced-expression`, and the imported names fall through to the catch-all.
  Repointing it would scope an imported type as a variable.
- **Markup prose** (17 tokens). A non-colon element's content falls through with no
  context of its own, so `Social media` in
  `<Option value="social">Social media</Option>` is scoped `entity.name.qualifier.nx`
  (`src/vscode/samples/survey.nx:20`). Note that this form
  is not valid NX — `nx-grammar.md:389-392` gives a non-colon element an
  `ElementsExpression` body, and bare text needs the `TextElement` colon form or a
  `TextChildElement` nested inside text content. It appears on 19 lines across
  `survey.nx` (13) and `tally-survey.nx` (6), and nowhere in
  `docs/drawnui-proposal/` or `examples/nx/`.
- **The `<:>` text fragment.** `<:>Yes, borrowed</>` is matched as a start tag with an
  empty `support.type.text.nx`, not as text, so its content falls through too
  (`src/vscode/samples/tally-survey.nx:318`).

So the catch-all is doing the work of four missing rules, and each has to exist before
it can be narrowed. This is the reason the `fix-component-signature-highlighting`
change fixed only the two pieces that are *positional* and therefore safe in isolation:
a `for` header (`for name[, name] in iterable`, matched from `\G`) and a reserved
literal used as a condition. Everything else in that area is still on the catch-all.

If this is revisited in the future:
- Add the missing rules first, then narrow the catch-all last. Narrowing it first is
  what makes the change look small and then regress imports and prose.
- Give `import` its own context covering the clause braces, the imported names, and
  `as`, rather than letting a `values-braced-expression` absorb it. Decide whether an
  imported name is a type reference or a binding, since NX imports carry both.
- Give a non-colon element's content its own context ending at the matching close tag,
  so an `ElementsExpression` body admits elements and control forms but does not treat
  stray words as expression tokens.
- Handle `<:>` … `</>` as a text fragment alongside the existing `<>` fragment rule.
- Only then repoint the bare-identifier catch-all at `variable.other.readwrite.nx`,
  keeping `entity.name.qualifier.nx` for the segments before a dot.
- Expect the corpus to change substantially — roughly 120 tokens go from unstyled to
  the variable colour, which is the point. Verify with a whole-repository token-stream
  diff rather than the test suite alone; the suite stayed green through a regression
  that only the corpus diff caught.
- Fix the 19 invalid lines in the two sample files to the colon form, or accept that
  invalid source renders as expressions. Sample files are the grammar's fixtures, so
  they are worth making valid either way.

## Nominal Value Identity: `$type` Is A Name, Not An Identity

A record value carries its type as a bare name — `$type: "User"`, or `"Shape.circle"` for a union
case — and the NX IR now carries each record's and union's base chain so a runtime can accept a
derived value where its base is expected (`crates/nx-codegen/src/ir.rs`, and `resolveSubtype` at
`runtime/typescript/src/index.ts:1073`). Fields reach the IR already flattened, so the chain answers
only what flattening cannot: whether a value stamped with one name is acceptable where another type
was asked for.

That resolution is by name, and a name is not an identity. Two modules may each declare
`type User extends Base`, and a value carrying `$type: "User"` at a `Base`-typed site does not say
which one it is. The runtime reports that ambiguity rather than choosing, because picking one would
normalize against the wrong field list and hand back a quietly wrong value. Note that the ambiguity
predates the base chain: two same-named records already serialize identically, so canonical output
has never distinguished them.

The obvious fix — qualify `$type` with the declaring module — is larger than it looks, because
`$type` is not an IR-runtime wire detail. It is the language's public value contract:

- `crates/nx-value/src/lib.rs:131` serializes every value's `type_name` as `$type`, so this is the
  canonical JSON form for the whole language, not just the IR path.
- `crates/nx-cli/src/typegen/languages/csharp.rs:348` emits
  `[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]` with `[JsonDerivedType(..., "U.case")]`
  attributes whose discriminator strings are bare names baked into generated C#.
- `crates/nx-codegen/src/runtime.rs:148` stamps the same thing in the generated JavaScript runtime.

109 files in the repository reference `$type`. Qualifying it changes the serialization contract of
every NX value in every target at once, including generated C# attributes. It is also the design
that ecosystem experience argues against for a discriminator specifically: `System.Text.Json` uses
short author-chosen discriminators, and the fully-qualified alternative is JSON.NET's
assembly-qualified `$type`, which welds internal structure into the wire format and moves whenever
code moves. `$type` here is read by hand — hosts branch on it and the fiddle prints it.

If this is revisited in the future:
- Put identity *alongside* `$type`, never inside it. A separate field is ignorable by a host that
  does not care; a mangled `$type` is not.
- Start with an in-runtime tag rather than a wire field: a symbol key survives object spread, is
  invisible to `JSON.stringify`, and needs no IR or format change. It covers every value the program
  itself constructs, which is the case that matters, and degrades to name resolution when a host
  round-trips a value through JSON.
- Only add a wire field (`$decl` or similar) if host round-trips turn out to matter in practice —
  that is, if the ambiguity is actually reached by values that left the runtime and came back.
- Do not qualify only on collision. A discriminator whose shape depends on what else is in the
  program means adding an unrelated module silently rewrites the wire format of existing values.
- Whatever is chosen has to land in the interpreter too. Parity between the interpreter and the IR
  runtimes is asserted value-for-value by `runtime/typescript/test/emitted-ir.test.mjs`, so a change
  to the stamped form in one is a change in both.
