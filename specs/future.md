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
code moves. `$type` here is read by hand — hosts branch on it and the playground prints it.

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

## A Declaration Named After A Primitive Is Constructible But Unnameable

NX lets a module declare a type whose name is a primitive's. `type string = { id:int }` parses,
validates, and lowers with no diagnostic, and the record it declares is usable through inference —
`let v = <string id=1 />` then `{v.id}` type-checks clean. What the declaration cannot do is be
*named*: every type annotation spelled `string` resolves to the primitive instead, because
`resolve_type_ref_with_seen` (`crates/nx-types/src/semantics.rs:95`) consults `builtin_type` before
it ever reaches the declaration resolver. So `type Holder = { s:string }` gets the primitive, and

```nx
type string = { id:int }
let x:string = <string id=1 />
// Initializer for value 'x' expects string, found string
```

reports a mismatch that prints the same name on both sides. The annotation is the primitive, the
element is the record, and `Display` spells both `string`.

The split is the one behind RF1 in the `add-component-type-parameters` review: `object` is a
`Type::Named`, not a `Type::Primitive`, so `builtin_type` does not answer for it and a declaration
named `object` shadows normally — as do `Element` and `never`. Only the seven true primitives
(`string`, `int`, `int32`, `int64`, `float32`, `float64`, `boolean`) are affected. That makes the
language inconsistent across the eight names the reference calls primitives, in a way nothing
currently tells the author about.

Note that `primitive-type-names` already grants the general permission — "A user declaration MAY
take the name `never`, resolved by the same rules that govern any non-primitive name" — and that
rule does hold for `never`, `object`, and `Element`. It is the seven that do not follow it.

If this is revisited in the future:
- Decide which of the two directions is intended before touching anything. Either a declaration may
  take these names and must then win where it is spelled (extending the `never` rule to all eight),
  or it may not and the declaration should be rejected at validation with a diagnostic that names
  the primitive. The current state is neither, and it is the diagnostic that makes it visible.
- Rejecting is the smaller change and matches what `add-component-type-parameters` already did for
  type parameters: `crates/nx-syntax/src/validation.rs` refuses a type parameter named after a
  primitive or a built-in. Extending that to a module-level `type` declaration is the same check at
  a different node — but it removes a permission authors have today, so it wants a deprecation pass
  over the corpus first.
- Letting the declaration win is the larger change: `builtin_type` would have to be consulted only
  after declaration lookup fails, which reverses the precedence at every type annotation in the
  language and needs measuring against the whole corpus rather than the test suite.
- Either way, fix the same-name-twice diagnostic first. It is cheap and independent: when a mismatch
  reports two types that render identically, disambiguate them — by origin, or by marking the
  primitive — so the message is actionable whatever the resolution rule ends up being. The same
  rendering collision is reachable through two same-named records from different modules, which
  "Nominal Value Identity: `$type` Is A Name, Not An Identity" above describes from the value side.

## Reconsider `Element` As A Built-In Type Name

`Element` is a built-in type name that nothing declares. It is the type of any element value: an
expected type spelled `Element` is satisfied by a component instance, an element function, or an
HTML-style tag. The relation is one-way, so `div` satisfies `Element` but `Element` is not
satisfied by a string or a record. It is easy to miss because it is recognized by spelling rather
than by any declaration, and the `add-component-type-parameters` review (RF7) surfaced how many
places quietly depend on that. Whether NX wants a built-in with this name, or a built-in here at
all, is worth deciding on purpose rather than by inheritance from the first implementation.
Options range from keeping it as is, to renaming it to something that reads less like a user
record (`Node`, `Markup`, `View`, or a lowercase spelling beside the primitives), to removing it in
favour of a declared or structural type.

How it is used today:

- **The checker special-cases the name.** `type_satisfies_expected`
  (`crates/nx-types/src/infer.rs:4421`) accepts any element-like nominal type where the expected
  type's name is `Element`, and `named_type_is_element_like` (`crates/nx-types/src/infer.rs:4392`)
  answers true for the spelling `Element` itself and for any name that resolves in the element
  namespace. Its unit test `test_element_supertype_requires_exact_case` pins that `div` satisfies
  `Element` and not `element`.
- **It has no declaration.** The name resolves to an origin-less nominal type through
  `nominal_named_type`, on the same path as an undeclared name. Nothing marks it as reserved, and
  the resolver has no entry for it; the special cases above are the whole of its definition.
- **It is the standard type for content props.** `content body:Element` appears throughout the
  docs (`reference/syntax/functions.md`, `reference/syntax/elements.md`, `language-tour/elements.md`,
  `overview/design-goals.md`, both tutorials) and in the example library as `content:Element[]`
  (`examples/nx/ui/components.nx`, `examples/nx/core/html.nx`). The `content-properties` spec is
  written in terms of it, and `reference/syntax/types.md` uses it as a function return type
  (`type ItemRenderer = (User) => Element`). Lowering keeps it as an element function's return
  type (`crates/nx-hir/src/lower.rs`, test at line 2837).
- **The language service treats it as a built-in.** It is offered as a completion in type position
  from `BUILTIN_TYPE_COMPLETIONS`, which now reads `nx_syntax::BUILTIN_TYPE_NAMES`
  (`crates/nx-syntax/src/lib.rs`), separately from the eight primitives, and hover renders it as
  `(built-in type) Element`; the `editor-syntax-highlighting` spec has a scenario for that hover
  line.
- **At the host boundary it erases to the top type.** The codegen builder maps an unresolved
  `Element` reference to `object` in the IR (`crates/nx-codegen/src/builder.rs:2406`), which is the
  same treatment a component type parameter gets.
- **Validation refuses it as a type parameter name.** `crates/nx-syntax/src/validation.rs` rejects
  `Element:type` on the same terms as a primitive-named parameter, with a scenario in the
  `add-component-type-parameters` change.
- **It is not reserved.** A module-level `type Element = { id:int }` is accepted and shadows the
  built-in for that module, exactly as `type object = ...` does, and as `primitive-type-names`
  grants explicitly for `never`. That is the asymmetry recorded under RF9 in the same review, and
  it overlaps with "A Declaration Named After A Primitive Is Constructible But Unnameable" above.

If this is revisited in the future:
- Decide the question for `Element`, `object`, and `never` together. All three are origin-less
  names the checker knows by spelling, and a rule that reserves or renames one and not the others
  leaves the same inconsistency the section above describes for the seven primitives.
- If it is renamed, the checker's two spelling tests, the codegen erasure, `BUILTIN_TYPE_NAMES`, the
  validation rule, the hover text and its highlighting scenario, and every `content body:Element`
  in the docs, examples, and the `content-properties` spec move together. Grep for the spelling
  rather than trusting the test suite: most of the sites above are string comparisons.
- If it is removed, the replacement has to answer what `content body:` is typed with. The
  candidates are a declared abstract record every element extends, which gives it an origin and
  a declaration site, or a structural "any element" type the checker expresses without a name.
  Either one changes what `named_type_is_element_like` means, and the second removes the only
  spelling an author has for "a list of elements".
- Whatever is chosen, give it a declaration site or reserve the name. The current state, where a
  built-in has neither, is what made RF7 and RF9 a question rather than a lookup.

## Built-In Element Content And Property Values Are Never Type-Checked

An element expression is type-checked only where its tag resolves to a declaration. For a tag that
matches no function, component, record, or union, `infer_element_expression`
(`crates/nx-types/src/infer.rs:1332`) falls through to `nominal_named_type` without ever reaching the
element's properties or its content — so nothing under a built-in tag is inferred. Body content has a
second, narrower gate on top of that: `check_element_bindings`
(`crates/nx-types/src/infer.rs:1874`) hands content to `check_content_binding` only where the tag
declares a content property, so even a declared component's body is inferred only when it takes one.

The effect is easiest to see through the editor, which reports a type only where the checker recorded
one. In a workspace where `<P m={x} />` reports `string` for `x`:

- `<div id={x} />` reports nothing — a property value under a built-in tag.
- `<div>{x}</div>` reports nothing — content under a built-in tag.
- `<div>hello</div>` reports nothing on the text, though the text run lowers as a string literal with
  a real span.
- `let <Panel width:int /> = <div>{width}</div>` reports nothing on `width`, while the same reference
  in a function body reports `int`.

Hover being conservative there is the visible symptom, not the problem. The problem is that these
expressions are never checked at all: a name that does not resolve, or a value of the wrong type,
passes through a built-in tag's properties and body without a diagnostic.

If this is revisited in the future:
- Treat it as "every lowered expression is visited exactly once" rather than as an editor fix. The
  language service reads whatever the checker recorded and needs no change of its own.
- Expect new diagnostics on code that reports none today, and budget for triaging them. That is the
  real cost of the change and the reason it did not ride along with an editor change.
- Watch for double-reporting where content is already checked against a declared content property —
  the new visit must not re-check what `check_content_binding` already checked.
- Note that this closes the second half of "Findings not fixed" in the `resolve-editor-positions`
  review; the first half, literal spans, was fixed in that change.

## String Literal Quotes And Escapes: The Semantics Are Unconfirmed

**Observed.** A string literal is one opaque token. The grammar accepts a backslash before any
character (`seq('\\', /./)` in `crates/nx-syntax/grammar.js`), and `unquote_string_literal`
(`crates/nx-hir/src/lower.rs`) strips the outer quotes and does nothing else. Nothing inside is
decoded: not backslash escapes, and not the character entities the scanner already recognizes
everywhere else. Measured today:

| written | value |
| --- | --- |
| `"a\nb"` | `a\nb` — six characters, a literal backslash |
| `"\""` | `\"` — a backslash and a quote |
| `"say &quot;hi&quot;"` | `say &quot;hi&quot;` — kept as written |
| `"a&#10;b"` | `a&#10;b` — kept as written |
| `"Tom & Jerry"` | `Tom & Jerry` — a bare `&` is fine |
| `'say "hi"'` | `L1: Syntax error` |
| `let p = "C:\"` | `L1: Syntax error` — the literal never terminates |
| a literal spanning two lines | a newline in the value |

**The decision to confirm.** Which mechanism an NX string uses for a character that would otherwise
be syntax has never been decided, and the current state is not a decision — it is a grammar that
lexes escapes and a lowering that ignores them. Nothing in the reference documents an escape set.
The four candidates are not exclusive:

- **Backslash escapes** (`\n`, `\t`, `\r`, `\\`, `\"`, possibly `\u{...}`). What the grammar's
  escape branch and the TextMate grammar already presume, and what the playground's examples
  presume *against* — `sites/playground/src/examples/nx/text.nx` tells authors that "a backslash in
  a string stays a backslash", which is true today.
- **Character entities**, which is the answer this language already gives for text content:
  `$.entity` is an external token admitted in `text_run` and `embed_text_run`, and
  `crates/nx-syntax/src/scanner.c` scans named, decimal `&#DDDD;` and hex `&#xHHHH;` forms with a
  bare `&` falling back to text, exactly HTML's rule. Extending it to string literals would make
  the two consistent and reuse the scanner. It is not backward compatible for a string that happens
  to contain a `&name;`-shaped run.
- **Single-quoted literals**, which is how XML and HTML answer this in the first instance: delimit
  with the quote the content does not use. This is purely additive — `'` has no token in the grammar
  and there is no `char_literal` rule — and the corpus already reaches for it from the other side:
  `sites/playground/src/examples/nx/svg.nx` writes its embedded SVG with single-quoted XML
  attributes precisely so the NX string can keep its double quotes.
- **No escapes at all**, making a backslash ordinary as it is in XML, which requires one of the two
  mechanisms above to exist first or a `"` stays unwritable.

These semantics want confirming deliberately before any of the bugs below are fixed, because each
fix presumes an answer, and because two of the candidates change what existing sources mean.

**Bugs in what is there today.** Each is a consequence of the above, and each should be fixed
whichever mechanism is chosen:

1. **A double quote cannot be written in a string at all.** `"\""` yields a backslash and a quote,
   and there is no other spelling, since `'...'` does not parse. Found from the DrawnUI fiddle,
   whose NX Welcome preset wants the `"Clicked 3 times!"` its C# and TSX twins put in a label and
   omits the quotation marks instead.
2. **A string cannot end with a backslash.** `let p = "C:\"` reports `L1: Syntax error`: the escape
   branch consumes the closing quote, so the literal runs on to the next quote in the file or to the
   end of it. A Windows path and a regex are both unwritable, and the diagnostic names neither the
   string nor the backslash — it points at the line and says nothing else.
3. **`"\n"` is not a line break.** A joined `string` body reads a line break in its text as one
   space (see `implicit-primitive-conversions`), and a braced string is how an exact line break is
   put back — but only a literal that really spans two lines does it, not `{"\n"}`.
4. **The editor grammar paints three things the language does not have.**
   `src/vscode/syntaxes/nx.tmLanguage.json` scopes `string.quoted.single.nx` for `'...'`, includes
   `#entities` inside both string patterns, and matches `\\[\\"nrt]` as
   `constant.character.escape.nx`. Single quotes are a syntax error and neither entities nor escapes
   are decoded, so the highlighter is describing an intended language rather than the real one.
   Whatever is chosen, that grammar and the formatter have to be brought into agreement with it.
5. **Nothing tells an author any of this.** No reference page states the escape set, so every item
   above is discovered by experiment. The playground recorded it as F11 in
   `sites/playground/docs/FINDINGS.md` rather than in the language's own documentation.

**What would settle it.** Confirm the mechanism, then decode in exactly one place in lowering so
every backend sees the same value; decide whether an unknown escape or entity name is an error or
is kept; document it in the expressions reference; and add conformance corpus cases for each form,
beside the ones in `specs/ir-conformance/`. Search `examples/`, `specs/ir-conformance/` and
`src/vscode/samples/` before changing any decoding, since a literal backslash or a `&name;`-shaped
string changes meaning. If single-quoted literals are part of the answer they can land first and
alone: they are additive, they need no decoding decision, and they close bug 1 by themselves.

**Related.** "Entities in text content are kept as written" is this same question asked from the
text side and shares the scanner — decide the two together, since whether a string carries entities
and whether text decodes them is either one rule or two that have to be explained separately.
"Typed Text Bodies: What A Text Type Means" turns on the same answer for a body handed to a second
parser.

## Typed Text Bodies: What A Text Type Means

`implicit-primitive-conversions` made a typed body (`<Note:markdown>`) bind its text: lowering has
an arm for `EMBED_TEXT_RUN`, the escapes `\@`, `\{` and `\}` are decoded, the body keeps its line
breaks and loses the indentation its lines share, and `Element` records the `text_type` the tag
names. What a text type *means* is still open.

Today the text type is recorded and nothing reads it: every typed body binds as one string, exactly
as a plain body does, and a host that wants Markdown rendered has to know from the property which
processor to run.

If this is revisited in the future:
- Decide what a text type gives a host: the joined string as now, the text and its interpolated
  values separately (so a processor can escape a value it did not write), or a processor named in
  the type system rather than the tag.
- Decide whether an unknown text type is an error, and where the known ones are declared.
- Entities in a typed body are still kept as written, and decoding them for a Markdown processor
  would turn `&lt;script&gt;` into raw HTML. See "Entities in text content are kept as written".

## Editor Hover: The Positions Still Unanswered

`resolve-editor-positions` made hover answer at declarations, references, expressions, literals,
component tags, property names, and type annotations. The positions below still report nothing, each
for its own reason and none of them the same reason as the section above. Each was measured against
the tree that change left behind, on a fixture checked to produce no diagnostic first — hover
declines inside a syntax error by design, so a fixture that does not parse reports nothing for a
reason that has nothing to do with the position.

- **A field name in a record or action declaration.** `<Panel ti|tle="x" />` reports
  ``property `title` `` and its declared type, but `a|:int` inside `type R = { a:int }` reports
  nothing. The property-name context the resolver produces is scoped to an element's opening tag; the
  declaration side was never given one. The annotation next to it does hover, which makes the gap
  look arbitrary to a reader.
- **Declaration details that are the bare kind word.** `Declaration::detail` is `"union"` for a
  union, `"record"` for a record, `"action"` for an action, and `"value"` for a value with no
  annotation (`crates/nx-language-service/src/lib.rs`). Hover suppresses the second line rather than
  repeating the kind under itself, so `type Mode = light | dark` hovers as ``union `Mode` `` and
  nothing more. Enriching those details the way `markup_signature` enriched the component case —
  union cases, record fields, a value's inferred type — improves hover and completion detail
  together, because both read that one field.
- **Operator and punctuation positions.** Hover on the `+` in `count + 1` reports nothing. This one
  is deliberate: the lookup is bounded by the construct the position resolved to, which is what stops
  a cursor in an unrelated string from being answered with its enclosing element's type. Widening it
  for operators means resolving the operator to its binary expression in the position resolver, not
  relaxing the bound.
- **The offset between `{` and a sign.** `{|-42}` reports nothing, though `{-|42}` and `{-4|2}` both
  report `int`. The chain descent gives a boundary offset to the node that *ends* there before the
  one that starts there, so the brace claims it. That tie-break is what makes other positions work —
  an unbraced `|-42` answers only because whitespace is no node and nothing competes — so changing it
  is not local. Widening the literal rule to also accept a node the offset merely abuts would fix
  this one position and would need re-measuring against every boundary case the resolver tests.
- **`null`.** It infers as `T0?`, an unsolved inference variable, so hover declines rather than
  render a variable id at a reader. It is the one literal form with no context-free type; answering
  it means either solving the variable from the binding site before hover reads it, or deciding
  in the renderer that an unsolved nullable spells `null`.
- **A unit literal.** `let value = {(|) 2}` reports nothing, where the `2` beside it reports `int`.
  `()` is valid only as an item of a list or value list (`grammar.js`), and it lowers to no literal
  expression, so unlike the cases above there is nothing recorded for a lookup to find.

## Editor Navigation: Go-To-Definition And Rename

`resolve-editor-positions` built a position resolver that answers "what construct is the cursor on?"
for hover and completion. Go-to-definition and rename need the same question answered and one thing
more: a *definition identity* for every reference — which declaration this name binds to, not merely
that it is a reference. That was an explicit Non-Goal of that change, and nothing was added for it.

What exists to build on: `PositionContext` already distinguishes `Reference` from `Declaration`,
`ComponentTag`, `PropertyName`, and `TypeAnnotation`, and the resolver was deliberately shaped not to
preclude the richer result — it classifies the position without deciding what a consumer does with
it. What is missing is resolution from a reference back to the declaration that binds it, across
documents, which is a `nx-hir`/scope question rather than a language-service one.

Rename additionally needs the inverse direction — every reference to one declaration — which nothing
currently indexes, and it needs to know which occurrences are the same name by identity rather than
by spelling. Expect that to be the larger half of the work.

## Language Service Cost Per Request

Two costs were accepted by design in `resolve-editor-positions` and are worth revisiting together
if editor latency ever shows up in profiling, rather than separately on suspicion:

- **The workspace is analyzed once per snapshot**, and a snapshot is what a request is served from.
  Nothing is cached across snapshots. This is the same gap as "Multi-File And Incremental Source
  Analysis" above, reached from the editor side.
- **Each position request re-parses the queried document**, because analysis discards the syntax
  tree and `ModuleArtifact` deliberately does not retain it — every consumer of the compile pipeline
  would pay for a structure only the editor reads. Measured at 0.09 ms against a snapshot build that
  type checks the whole workspace.
- **`LoweredModule::innermost_expr_at` scans the arena linearly.** Modules hold hundreds of
  expressions and it runs once per position query, so an index is not yet worth its invalidation
  surface — and a genuinely sublinear interval query needs an interval tree, because lowered spans
  neither nest reliably nor are all present. The query lives in `nx-hir` precisely so that decision
  can be made there without touching a caller.


## Web Editor Packages: What `add-web-editor-packages` Left For Later

The change that added [`language-protocol`](../openspec/specs/language-protocol/spec.md),
[`language-http-service`](../openspec/specs/language-http-service/spec.md) and
[`monaco-language-integration`](../openspec/specs/monaco-language-integration/spec.md) shaped four
packages for publication and stopped short of publishing them. Each item below is a change of its
own; none blocks using the packages from the repository's workspace today.

### Publishing the packages

- **The release pipeline attaches one npm tarball per release** and publishes only
  `@nx-lang/language`. Extending it to `@nx-lang/language-protocol`, `@nx-lang/language-http`,
  `@nx-lang/language-client`, `@nx-lang/monaco`, `@nx-lang/ir-runtime` and `@nx-lang/sdk-node`
  means a release manifest that lists several tarballs, the one-tarball assertion in
  `package-publish.yml` relaxed to that list, and trusted-publishing registration for each new
  name. Every new package already passes `pnpm run verify:package` (pack, check the manifest,
  install the tarball into a scratch project, import every export), so the pipeline change is
  mechanical.
- **`@nx-lang/sdk-node` needs per-platform native prebuilds** before a registry consumer can
  install it: a napi build matrix (Linux x64/arm64, macOS, Windows) attached to the release and
  wired through `@napi-rs/cli`'s optional-dependency convention, plus a first release of
  `@nx-lang/language` itself, which has a pipeline but has never been published — the Monaco
  package declares it as a peer dependency and this repository's workspace links `src/vscode` in
  its place until then.

### Folding `src/vscode` into the root workspace

The extension keeps its own nested pnpm workspace. The end state is one workspace in which
`@nx-lang/language` is a real package directory under `packages/language` holding the grammar,
language configuration and snippets; the extension is a member that depends on it and copies the
assets into its tree at VSIX packaging time (grammar contributions must be file paths inside the
VSIX); `src/vscode/scripts/package-language.mjs`, which fabricates the package today, is deleted;
and `@nx-lang/monaco` depends on it as `workspace:*` rather than through a `file:` link. That fold
moves the lockfile and working directory four CI workflows reference (`build`, `release`,
`package-publish`, and the VSIX publishing job) and must verify `vsce` packaging under a root
workspace, which has a history of friction with pnpm's symlinked dependencies — the extension
already bundles, which is the standard remedy.

### ReachMe's migration

In the ReachMe repository, once the packages are consumable, first confirm its `monaco-editor`,
`shiki` and `@shikijs/monaco` versions satisfy `@nx-lang/monaco`'s peer ranges (Monaco 0.56 or
later, Shiki 4); the ranges record what the playground and the package's tests exercise, not a feature
the package needs, so an older Monaco there means testing the package against it and widening the
range rather than moving ReachMe. Then: add `external/nx/packages/*` to its
`pnpm-workspace.yaml` beside the two entries it has (or, once published, depend by version);
replace `apps/web-app`'s `monacoNxLanguage.ts` and the highlighting half of `NxCodeEditor.tsx`
with `registerNxLanguage` from `@nx-lang/monaco`, supplying its multi-file draft as the workspace
callback and its authorization header through `@nx-lang/language-client`; mount
`createNxLanguageHandler` in the Hono API under `/api/language/*` with the same registry-backed
build context it already validates with; rename the `nx-language` `file:` link to
`@nx-lang/language`; and remove the submodule once every package it consumed is on the registry.

### Deleting the prelude

`@nx-lang/language-http`'s `prelude` option exists because an imported external component loses
its defaults and inherited properties (NXE12/NXE13), which forces the DrawnUI playground to analyze the
catalog and the visitor's text as one module. Fixing NXE12/NXE13 is the condition for deleting the
option, its shift helpers, and `server/compile.mjs`'s use of them: the catalog becomes a library
loaded through a `LibraryRegistry` and passed as the handler's `buildContext`, which the handler
already supports.

## Playground: What `add-playground-site` Left For Later

The playground at `nxlang.org/playground` (`sites/playground`, spec `openspec/specs/playground`)
shipped as the DrawnUI fiddle under a public address: gallery, editor view, Railway behind
Cloudflare, the service declared in `.railway/railway.ts` and deployed by
`.github/workflows/deploy-playground.yml` with `railway up`. Compilation and language queries were
server-side then; `add-wasm-sdk` moved both into the visitor's browser, as a WebAssembly module in a
Web Worker. The items below are what the site deliberately does not do yet.

### Shareable edited source

An edit lives only in the session. A visitor who writes something worth showing has no address for
it: `/playground/<id>` always opens the example as authored. The URL scheme leaves the query and
fragment of that address free for this — the smallest version encodes the source in the fragment
(compressed, so a typical example fits a browser's URL limit), and a stored version would need
somewhere to keep it and a policy for how long. Either one is a client change plus, for storage, a
route; nothing in the current address scheme has to move.

### NX IR size

Every compile produces the whole program's IR as pretty-printed JSON, and the program is the
visitor's source plus the entire flattened catalog: a few hundred kilobytes of text per keystroke
pause, built inside the worker and structured-cloned to the main thread. It is fast enough that no
one notices, and it is the single largest thing the pipeline moves. Two independent wins are
available: emitting compact JSON rather than pretty-printed, and not re-emitting the catalog's
declarations on every compile. Neither changes a seam.

### The catalog as a library artifact

The catalog is prepended to the visitor's source as a prelude, so every compile reanalyzes ~600
lines of external component declarations that never change. It is a prelude rather than an import
because an imported external component loses its defaults and its inherited properties
(NXE12/NXE13). Once that is fixed, the catalog can be a library artifact analyzed once per worker
and shared by every compile — the win is proportional to how much of each compile is the catalog,
which today is most of it. The prelude arithmetic in `@nx-lang/language-core` would go with it.

### A static host, without the Node server

The server now serves `dist/` under the prefix, redirects `/`, and answers a health route. Nothing
it does needs a process: a static host with a rewrite rule and a fallback document would serve the
same site, and the health check that gates a Railway deployment would go with the thing being
deployed. What has to be decided first is what replaces the deploy gate — a static host has no
health check to poll — and where the redirect at `/` lives, which is the same question the section
below asks. Keeping the Node process meanwhile costs one small container and no complexity.

### Splitting the domain across services

Everything the site serves is under `/playground`, so a second service on `nxlang.org` — a home
page at `/`, the docs — is an edge change, not a code change. The root redirect is the one thing
that moves: it lives in the playground's server and would have to be replaced by whatever serves
`/`. Routing by path to a second Railway service needs either a Cloudflare Origin Rule with a host
override (confirm the plan supports it) or a Worker in front of both. Whichever is chosen, the new
service must go through Cloudflare the way the playground does: the playground has no
Railway-generated domain on purpose, because that hostname would answer outside the edge, where the
cache rules do not apply.

## The DrawnUI Fiddle: What `add-nx-to-drawnui-fiddle` Left For Later

NX is a language of the DrawnUI fiddle engine (`DrawnUi.FiddleEngine`, the engine behind
drawfiddle.com): the fiddle compiles NX in the visitor's browser with `@nx-lang/sdk-wasm` against a
DrawnUI catalog it generates from the `drawnui-react` package it ships, evaluates `root` with
`@nx-lang/ir-runtime`, and draws the result with DrawnUi.React. Its shares carry the compiled NX IR
and play without the compiler. Nothing DrawnUI-specific entered this repository for it; what did was
the prelude-aware build in the wasm SDK, compact IR, the Monaco peer range, and the npm release
track for the workspace packages. The items below were deliberately left out.

### Removing DrawnUI from the playground

The playground still vendors DrawnUi.React and keeps its own catalog, generator, coercion and
renderer under `sites/playground`, duplicating what the fiddle now owns. With the fiddle as the
public DrawnUI playground for NX, the playground can drop its DrawnUI target and become a
general-purpose site. Blocked on deciding what a general-purpose playground draws instead — the
examples, the gallery and the renderer all assume DrawnUI — which is a change of its own rather
than a deletion.

### The catalog as a library artifact, for shares

Every NX share artifact carries the whole program's IR, and the program is the snippet plus the
flattened catalog: about 880 KB of compact JSON for a snippet of a few lines, of which the snippet
is a few kilobytes. Shipping the catalog's IR once with the fiddle's runtime bundle and storing
only the snippet's in the share needs the IR to reference declarations across artifacts, which is
the library-artifact work above: an imported external component must keep its defaults and
inherited properties (NXE12/NXE13) before the catalog can be a library, and the runtime must link
two IR documents before a share can be one of them. Until then compact JSON is the mitigation, and
the private backend's artifact limit decides whether NX shares can be opened to the public.

### The fiddle's compile in a Web Worker

The fiddle compiles on the main thread: a compile takes on the order of 100 ms and a trap is
recovered by rebuilding the host in under 2 ms, so the tab stays responsive and a crash costs one
run. The playground's worker adds a deadline and isolation that the fiddle does not have, so a
compiler hang — none is known; the compiler is a type checker and code generator with no
non-terminating paths — would freeze the tab. Blocked on nothing but a reason: the worker's
message protocol, the deadline and the crash handling are the playground's `src/worker`, and
moving them to the fiddle is a port that adds a thread hop to every compile and every hover.

## Logical operands: the IR runtime coerces, the interpreter demands a boolean

**Observed.** The two runtimes disagree about what a non-boolean operand of `&&` or `||` means. The
TypeScript IR runtime passes each operand through a truthiness helper, `truthy` in
`runtime/typescript/src/index.ts`, which is `Boolean(value)`, so a string or a number is accepted
and coerced. The Rust interpreter rejects it, raising a type error naming `logical and` at
`crates/nx-interpreter/src/interpreter.rs`.

**Why it might matter.** This is the same family as the short-circuit divergence that was fixed by
making the IR runtime non-strict in the right operand of `and` and `or`. That one was observable and
wrong. This one may not be observable at all, because the type checker probably rejects a
non-boolean operand before either runtime sees it, in which case the coercion is dead code rather
than a semantic difference. It was not verified either way.

**What would settle it.** Try to compile a program whose `&&` operand is not a boolean, for example
`let root() = { "a" && true }`. If static analysis rejects it, the coercion is unreachable and the
helper can be replaced with a check that fails loudly, which is the safer thing for a runtime that
reads images written by strangers. If static analysis accepts it, the two runtimes genuinely
disagree and one of them is wrong, and a conformance corpus case should pin whichever behavior the
language intends.

**Related.** The conformance corpus gained short-circuit cases in
`specs/ir-conformance/expressions/main.nx`. A case for this would belong beside them.

## `int32` overflow: the interpreter wraps, JavaScript does not

**Observed.** An `int32` result outside the `int32` range differs between backends. The interpreter
wraps it (`wrapping_add`, `wrapping_sub` and `wrapping_mul` in
`crates/nx-interpreter/src/eval/arithmetic.rs`), so `2147483647 + 1` at `int32` is `-2147483648`.
The TypeScript IR runtime and generated JavaScript carry an `int32` as a `number` and compute
`2147483648`. The TS runtime refuses that value once it reaches an `int32` parameter or field
(`normalizePrimitiveValue`), but generated JavaScript carries it on silently. Negating `int32::MIN`
and dividing it by `-1` hit the same edge, and the interpreter's `a / b` on `i32` panics on the
second.

**Why it might matter.** It is the one remaining way a narrow numeric type can compute a different
value on different backends. `float32` arithmetic was made to agree by giving the IR `float32`
operators (`fadd32` and siblings, `implicit-primitive-conversions` RF14). Integer arithmetic was
not changed, because the `primitive-type-names` spec already says arithmetic should be checked
rather than wrapping, and leaves that enforcement to a later change that also covers `int`'s
±(2^53−1) range and user-declared ranges.

**What would settle it.** The range-enforcement change: make integer overflow an error in the
interpreter (`checked_add` and siblings, including `MIN / -1`), and have the JS targets check the
result of each `int32` operation. A check after the fact is enough for checked arithmetic, since a
product that leaves the `int32` range stays outside it even when a `float64` rounds it. That may
mean `int32` IR operators like the `float32` ones, or a range check the runtime derives from the
operator's checked type. Add conformance corpus cases for overflow on each operator.

**Related.** `int` has the same shape at a larger scale: the interpreter wraps an `i64`, and
JavaScript loses precision beyond 2^53.

## Entities in text content are kept as written

**Observed.** The grammar lexes `&amp;`, `&#10;` and `&#x0A;` in a text run as `entity` tokens, and
the language tour says `<Tag:raw>` exists "to prevent interpretation of braces or entities". No
backend decodes them, though. Lowering copies a run's source text (`text_run_value` in
`crates/nx-hir/src/lower.rs`), so `<Label>a &amp; b</Label>` binds `text` to `a &amp; b`. The
escapes `\@`, `\{` and `\}` are decoded there (`implicit-primitive-conversions` RF21). Entities
were left alone because decoding them is a design choice, not a bug fix.

**Why it might matter.** `implicit-primitive-conversions` made a text body a user-visible string
at a `string` content property, so the undecoded form now reaches hosts and output. Decoding raises
two questions:
- **Which names.** The scanner accepts any `&name;`. Candidates are XML's five (`amp`, `lt`, `gt`,
  `quot`, `apos`) plus numeric references, or HTML's full table. An unknown name could be kept, or
  it could be a diagnostic.
- **Where.** A typed body such as `<Note:markdown>` is handed to a host that parses it again.
  Decoding `&lt;script&gt;` before a Markdown renderer sees it turns escaped text into raw HTML. A
  plain body at a `string` site has no second parser, so decoding it is safe.

**What would settle it.** Choose the name set and whether typed bodies decode. A likely answer is to
decode XML's five and numeric references in plain text, reject an unknown name, and pass typed
text through unchanged for its host to interpret. Put the decoding in `text_run_value` and add
checker diagnostics and a conformance corpus case. Then make the tour's `raw` sentence true, or
reword it.

**Related.** The same function decodes the escapes. `\{` in a plain body does not parse today,
although the `text_run` grammar lists `escaped_lbrace`.
