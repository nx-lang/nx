## Context

See proposal.md for motivation. This design is one of nine changes for the ReachMe agent work; the
agreed design is `reachme/docs/ReachMe-Cloudflare-AI-Agent-Architecture.md`, §5.2, §5.6 and §6.1.
What shapes the approach is how much already exists:

- **Function values are finished.** `function-values` and `function-types` (archived as
  `2026-09-18-add-function-types`) made a function name a value that captures nothing, rendered by
  both IR runtimes as `{ "$type": "Function", module, name }`. `callFunction` in
  `runtime/typescript/src/index.ts` and `call_function` in `crates/nx-ir-runtime/src/component.rs`
  call such a record with arguments keyed by parameter name, normalize each argument against the
  parameter's declared type (`invokeFunction` calls `normalizeValue` per parameter), fill defaults
  and optional parameters in the callee, and drop arguments the function does not declare. That is
  exactly the call a tool host makes. Nothing in this change touches evaluation.
- **Only the type is missing.** `check_function_satisfies` in `crates/nx-types/src/ty.rs` fails
  with `UnsuppliedParameter` for any parameter the expected type does not declare. A type that
  supplies more parameters therefore accepts more functions, and `<function />: R`, which supplies
  none, accepts the fewest: only functions that declare no parameter. The type every function
  satisfies would have to supply every possible parameter name, each at the bottom type. It cannot
  be written, and it could not be called, since no value has the bottom type. `object` accepts
  every function, and everything else too. Checked against the working tree: `type Tool = {
  fn:object }` accepts `fn={double}` and `fn="double"`; `fn:<function />: object*` rejects
  `double(n:int)` with "the function declares parameter 'n', which the function type does not
  supply".
- **`object*` is the top of result types.** A function type's result may carry an occurrence. On
  the working tree, functions returning `string`, `string?`, `string*`, `Plan+`, `int` and a
  function type all satisfy `<function />: object*`. So "any result" needs no new notion: it is
  `object*`.
- **An element is never a function type.** `fn=<Function module="m" name="f" />` at a
  function-typed field is already rejected, "expects `<function />: object*`, found Function": an
  element with no declaration has the type `Type::Named(tag)`, and no named type satisfies a
  function type.
- **The runtimes already ignore a function type's parameters.** The TypeScript runtime's
  `normalizeValue` handles `ty.kind === "function"` by accepting a function reference or resolving
  a host `Function` record, with a comment that no parameter is re-checked. The Rust runtime goes
  further: `crates/nx-ir-runtime/src/module.rs` reads type kind `4` into a signature-less
  `Type::Function`. Boundary handling for "any function" is the code both already run.
- **Schema 5 is released.** v0.5.0 ships it and ReachMe pins 0.5.0. Unlike `add-function-types`,
  which amended an unreleased schema in place, a new type kind here has to arrive behind a
  required feature so a 0.5.0 runtime refuses the module by name.

## Goals / Non-Goals

**Goals:**

- A property can be declared to hold a function of any parameters, with a stated result, and the
  checker rejects a non-function, and a function of another result, there.
- "Any function at all" is that same type with the widest result, not a second construct.
- A host that reads such a property gets a record it can trust to name a real function of the
  linked program, and can call it by the function's own parameters.
- No existing program changes meaning, and no artifact that does not use the type changes by a
  byte.

**Non-Goals:**

- A name for the type, built in or in the prelude (D2).
- Calling a function reference value from NX, or recovering its parameters.
- `...` after stated parameters, or anywhere but a function type's parameter list.
- Reflection: reading a function's parameters, result or doc comment through the record. That is
  `add-declaration-schema-export`.
- Changing the existing TypeScript mapping of a function type with stated parameters.
- A `.NET`, Node or wasm API for calling a function value.

## Decisions

### D1. One construct: a function type whose parameter list is `...`

`function_type` in `crates/nx-syntax/grammar.js` accepts the token `...` in place of its repeated
parameter definitions: `<function ... />: R`. Everything else about a function type is unchanged:
`function` is a keyword only in that slot, the result is required, a suffix after the result binds
to the result, and an occurrence over the function type needs parentheses.

This puts the new type in the family it belongs to. Its rules are the function-type rules with the
parameter check removed: the result is covariant, a function type with parameters satisfies it,
and it displays in function-type spelling. "Any function" is the instance with the widest result,
`<function ... />: object*`, not a second notion.

`...` has precedent for exactly this: Python's `Callable[..., R]`, Erlang's `fun((...) -> R)` and
Elixir's `(... -> R)` each mean any parameters and a result `R`. NX has no `...` token today; its
range operators are `..` and `..=`, in value expressions only.

`...` is the whole parameter list. The grammar accepts it after parameter definitions too, so that
`<function Item:Contact ... />: R` is a validation error with a message of its own rather than a
syntax error, and validation rejects the mix. Such a type would still not be callable, so it would
add nothing over the bare form.

*Alternatives rejected:*

- **A built-in type name `Function` with no result**, beside `Element` in `BUILTIN_TYPE_NAMES` (the
  first draft of this change). It needs no grammar change. But it cannot state a result, so
  `HttpTool.arguments` returning `HttpArguments` would be checked by the host and reported at
  normalization rather than in the editor. It is a second kind of function type with rules of its
  own. It needs a rule that a type parameter cannot take the name, and a guard so that an
  undeclared `<Function ... />` element, which has the type `Type::Named("Function")`, does not
  satisfy it. And adding the result later would cost a second type kind and a second required
  feature, because kind `6` would have been released with no operand.
- **`function` as a ninth primitive type name.** It needs a grammar change too, makes `function` a
  keyword in every type position rather than one slot, and by `primitive-type-names` a primitive
  is not displaced by a user declaration, so a module declaring `type function` would change
  meaning.
- **A generic `Function` with a result parameter**, `<Function R=HttpArguments/>`. NX has no
  default for a type parameter, so the common case would be `<Function R=object/>` everywhere, and
  a type parameter takes an exactly-one type, so a result such as `string*` could not be stated.
- **A word in place of `...`**, such as `<function any />: R`. It avoids `...` being read as prose
  elision in documentation, at the cost of the Python and Erlang familiarity and of a second
  contextual keyword. See Risks.
- **Use `object`.** No check at all; this is the status quo the proposal rejects.

### D2. No name for the widest type

The type every function satisfies is written out, `<function ... />: object*`. This change adds no
`Function` name, neither a built-in type name nor a prelude alias.

A prelude alias, `export type Function = <function ... />: object*`, was drafted and removed. It
would save a few characters at very few sites: an application rarely writes this type, and a
library writes it once per field and should choose the result deliberately. It would cost a
prelude change (regenerated prelude images in both runtimes, the corpus and the .NET prelude
companions), a requirement about the name (a module's own or imported `Function` taking it, the
lowercase suggestion), completion and hover requirements, and a `typegen` rule for a reference to
it. It would also show one spelling in source and another in every diagnostic, because an alias is
expanded where it is used, and it would be the third thing called "Function" beside the keyword
and the rendered record's `$type`.

The choice is asymmetric. Prelude names bind last, so adding the alias later breaks no program,
and removing one later would. A library that wants a short name declares its own alias today.

The cost is that `object*` as "any result" is not obvious: an author who writes
`<function ... />: object` excludes a function that returns `string*` or `string?`. The reference
states this where it introduces the type (task 9.2), and the mismatch diagnostic names the result
the function returns and the one the type expects.

### D3. A distinct variant in the checker and in HIR

`nx-hir` gains `ast::TypeRef::AnyFunction { return_type }` beside `TypeRef::Function`, and
`spell_function_reference_type` beside `spell_function_type`, so the checker's display, the hover
and `ir explain` share one spelling as they do today. As implemented this is a variant rather than
a flag on `TypeRef::Function`, for the reason given for the checker below: a site that reads a
function type's parameters and was not updated fails to compile instead of reading the type as a
function of no parameters.

`nx_types::Type` gains a variant for the type (working name `Type::AnyFunction { ret }`) rather
than a flag on `Type::Function`. Every place that reads `Type::Function { params, ret }` today
means "a function I can call with these parameters": `function_typed_value` for an element call,
`infer_call` for a paren call, `check_function_satisfies`. With a separate variant those keep
their meaning by construction, and the type is opaque without removing anything (D5).

### D4. Compatibility, join and display live beside the function-type rules

- `Type::is_compatible_with` and the checker's `type_satisfies_expected` gain two arms ahead of the
  function-to-function arm. (`Function { ret: A, .. }` or `AnyFunction { ret: A }`) →
  `AnyFunction { ret: R }` holds when `A` satisfies `R`; when it does not, the mismatch is the
  existing `FunctionMismatch::Result`. `AnyFunction` → `Function` never holds, with a dedicated
  mismatch message ("its parameters are not stated"). `AnyFunction` → `object` is the existing
  top-type rule.
- `semantics::common_item_supertype` returns `AnyFunction { ret: object* }` instead of `object`
  when both sides are function types, of either variant, and neither satisfies the other. The
  checker's own `common_item_supertype` falls through to it. The result is not joined: the join of
  two result types under occurrences is more machinery than the case needs. This is why
  `{findPlans listRegions}` can be bound to `tools: (<function ... />: object*)+` after being
  bound to a `let` with no annotation. It is not breaking: the join satisfies every site `object`
  satisfied.
- `Display`, `qualified_display` and `write_postfix_type` print `<function ... />: R`, and
  parenthesize it under a suffix, as `function-types` already requires of a function type.
- The type is exactly-one, so `Type::seq` wraps it like any item type and `optional-properties`
  applies with no special case. Written out, an occurrence over it needs parentheses,
  `(<function ... />: R)+`; an alias of it takes a suffix directly.
- A type parameter in the result is erased where a function type's is: `erase_type_parameters`
  recurses into the result.

### D5. Opaque by construction; two diagnostics for the attempt to call

Nothing has to be removed to make the value opaque: an element call resolves a tag to a function
call only through `function_typed_value`, which matches `Type::Function`, and `infer_call` rejects
a callee that is not a function type. The work is the message. Left alone, `<f n={n} />` on a
function reference parameter falls through to "an element with no declaration", and the author
sees a result-type mismatch naming `f` as a type. Both paths get a dedicated diagnostic (working
code `function-reference-not-callable`) that says the value cannot be called because its
parameters are not stated and shows a function type with parameters as the fix.

*Alternative rejected:* allow `<f a={x} />` on such a value and bind by name at run time, as
`namedCall` already does. The checker could type the result but not check an argument, so every
such call would be unchecked. This is what TypeScript's and Dart's `Function` allow, and both now
lint against it. The agent library does not need it: the host calls the function.

There is no downcast. NX has no type-test expression (match patterns compare values), so none is
reachable, and the spec says none is added.

### D6. IR: type kind `6`, `anyFunction`, behind `function-reference-type-v1`

Entry layout `[6, type]`: the result type. It is interned like every type, so a module has at most
one entry per result. Emitted from a new `CodegenTypeRef` case in `crates/nx-codegen/src/ir.rs`;
the kind constant and name go in `crates/nx-ir/src/model.rs` beside `FUNCTION` and `SEQ`;
`NxIrImage::open` validates the entry length and that the operand is a type index; `ir_explain`
prints `<function ... />: R`. A module whose type table holds an entry of the kind lists the new
feature. The schema version and `NX_IR_RUNTIME_ABI` are unchanged, because no existing image
changes and no evaluation rule changes.

Node kinds do not change. A function bound at such a site is the same `reference` node, and
`function-values-v1` is listed on its existing terms. A module can list the new feature alone (a
library that only declares `type Tool = { fn: <function ... />: object* }`) or both.

*Alternatives rejected:*

- **Erase to `object` in IR.** Simplest, and no feature. But the runtimes would stop validating
  host input at the field, the host could be handed a forged record from an `object`-typed path
  with no way to tell, and nothing that reads the IR could tell such a field from an `object`
  field.
- **A `function` entry (kind `4`) with a sentinel parameter list.** Overloads a layout that two
  released readers validate strictly, and a 0.5.0 runtime would read it as a function type.
- **Reuse `function-values-v1`.** A 0.5.0 runtime implements that feature and would reach an
  unknown type kind, which it reports as a malformed image rather than as a missing feature.

### D7. Runtimes: one more type kind into the existing function branch

- **TypeScript** (`runtime/typescript/src/index.ts`): the type reader gains kind `6` as
  `{ kind: "anyFunction", result }` in `PreparedType`; the supported-feature set gains the new
  constant (`NX_IR_REQUIRED_FEATURE_FUNCTION_REFERENCE_TYPE_V1`); `normalizeValue` handles
  `anyFunction` with the body of the `function` case (accept a `FunctionReferenceValue`, else
  `asFunctionRecord` and `resolveFunctionRecord`, else `nx-ir-boundary-type`). Canonicalization,
  equality and `callFunction` already work on the value and are untouched. `NxFunctionRecord` is
  exported as a type; `asFunctionRecord` stays internal.
- **Rust** (`crates/nx-ir-runtime`): `module.rs` validates the operand and maps kind `6` to the
  existing signature-less `Type::Function`, and the feature list gains the constant.
  `normalize.rs` is unchanged.
- **The result is not re-checked at the boundary.** A value the program produced was checked by
  the compiler. A host-supplied record is resolved against the linked program and accepted
  whatever the function's result, which is how both runtimes treat a function type with parameters
  today: no part of the signature is re-checked. See Open Questions.
- A `Function` record is resolved against `declarationsByName`, as today, so it may name any
  top-level function of a linked module, exported or not. That is the existing rule for
  function-typed sites and this change keeps it.

### D8. Generated types describe the record

A host reads and supplies the `Function` record at such a member, so both backends type it as the
record, whatever the result type. C# reuses `global::NxLang.Nx.NxFunctionRef`
(`bindings/dotnet/src/NxLang.Sdk/NxFunctionRef.cs`), which `csharp.rs` already emits for
`TypeRef::Function`. TypeScript emits `NxFunctionRef`, an interface with `$type: "Function"`,
`module` and `name`, into the helper module beside `NxRecord` (or inline in single-file output)
only when referenced, the mechanism `typescript.rs` uses for prelude declarations. The generated
type is independent of `@nx-lang/ir-runtime`, as all generated TypeScript is; `NxFunctionRecord` in
the runtime is the same shape for hosts that do not generate types. `typegen` works from
`TypeRef`, so it maps a function type whose parameters are unspecified, and an alias of one as it
maps any alias.

This leaves one inconsistency in place, on purpose: a function type with stated parameters maps to
a TypeScript function type, `(args: { n: number }) => number`, although the value on the wire is
the same record. That mapping was decided in `add-function-types` (its D6) and is pinned by
typegen tests, so it is not changed here (Open Questions).

Executable source codegen (`crates/nx-codegen/src/emit.rs`) holds a function value as the
JavaScript function, so in generated TypeScript the type is `(...args: never[]) => unknown`: every
generated function is assignable to it, whether it takes its parameters by position or as one
object, and it cannot be called. The result is `unknown` whatever `R` is, because NX admits results
TypeScript's spelling of `R` does not (`int` satisfies `object*`, and `string` satisfies `string*`
by the one-level lift); the checker has already related the result to `R`. Its boundary schema is
`nxAnySchema`, as for a function type. The corpus harness prints such a value as a `Function` record without `module`
and compares it with the recorded record by name.

### D9. Language service needs tests only

A hover on a member shows its type as written, through `spell_type_ref` (D3), and there is no new
name to complete or describe. `...` in a function type needs a highlighting check. No
`editor-language-service` requirement changes.

### D10. The HIR interpreter, while it exists

`crates/nx-interpreter` still evaluates source for `nxlang run`, `nx-api` and the bindings, and it
already has `Value::Function`. It needs the new type accepted wherever it coerces a value to a
declared type; `interpreter.rs` has one such test today,
`matches!(expected, Type::Function { .. })`. The active change `retire-hir-interpreter` deletes
that crate and routes native evaluation through IR. Whichever lands second absorbs the difference:
if this change lands first, task group 4 applies; if `retire-hir-interpreter` lands first, task
group 4 is dropped and the IR path covers native evaluation.

### Sequencing with other changes

- **Agent set.** Depends on none. `add-agent-library` uses `<function ... />: object*` for
  `FunctionTool.function` and `<function ... />: HttpArguments` for `HttpTool.arguments`.
  `add-declaration-schema-export` exports schemas from the program artifact at compile time
  (`artifact.functionSchema`, `artifact.typeSchema`), so it classifies the checker's function
  reference type, not IR type kind `6`, and reports it with the diagnostic
  `schema-inexpressible-type`; the paired test is its task 3.6 and task 7.5 here, added by
  whichever lands second. The `@nx/agent` image that `add-agent-library` carries lists
  `function-reference-type-v1`. `add-agent-host-package` relies on the boundary rule in D7 and on
  `callFunction`, and may type the record with `NxFunctionRecord`.
  `add-ir-runtime-evaluation-budget` is independent; both edit `runtime/typescript/src/index.ts`
  in different places.
- **The host's `HttpArguments` check moves to the call** (decision 8 of `add-agent-host-package`).
  The compiler checks the result where the program binds the function, so normalization does not.
  The runtimes do not re-check a host-supplied record's result, so the host package checks the
  `$type` of the value an arguments function returned before it builds a request.
- **`retire-hir-interpreter`:** see D10.
- **`infer-unannotated-return-types`:** both edit `crates/nx-types/src/infer.rs`. A function with
  an inferred result is a function value and satisfies `<function ... />: R` by its inferred
  result.
- **`add-go-to-definition`:** the type has no name and no declaration, so there is nothing to
  navigate to. No requirement is added here.
- **`support-stateful-component-value-evaluation`, `add-ir-runtime-performance-harness`:** no
  overlap.

## Risks / Trade-offs

- [`...` reads as prose elision. The docs and specs write `<f ... />` and `type Name = { ... }` to
  mean "details omitted", so `<function ... />: R` as literal syntax can be misread as an
  abbreviated example] → The reference page introduces it as a token, examples that use it state a
  result, and prose that elides writes the elision another way near a function type. The word form
  in D1 stays available if this proves confusing before release.
- [`...` is also the spelling an element-shaped language would use for prop spread,
  `<Row ...props />`] → The two do not collide in the grammar: spread takes an operand and sits in
  an element's property list, and this token stands alone in a function type's parameter list. If
  NX adds spread, the reference documents both.
- [`<function ... />: object` looks like "any function" and is not: it rejects a function whose
  result carries an occurrence] → D2: the reference says `object*` is the any-result form, and the
  diagnostic names both results.
- [The any-function type is long to write, `(<function ... />: object*)+` for a list] → A library
  declares an alias where it repeats. A prelude alias can be added later without breaking a
  program (D2).
- [A user element named `Function` renders a record with `$type: "Function"`, which a host cannot
  tell from a function record when it arrives through an `object`-typed path] → This exists today.
  Such an element satisfies no function type, and at those sites the runtimes resolve a
  host-supplied record against the linked program. A host should read function records only from
  members declared at a function type; the runtime README says so.
- [A host-supplied record at a `<function ... />: R` site may name a function whose result is not
  `R`] → Unchanged from function-typed sites, where no part of the signature is re-checked. The
  record comes from evaluating the program, where the compiler checked it, or from the host
  itself. A host that accepts such records from elsewhere checks the value the function returns, as
  `add-agent-host-package` does before it builds a request.
- [A `Function` record may name a private function, so a host-supplied record can make a host call
  a function the library author did not export] → Unchanged from function-typed sites. Hosts that
  pass untrusted JSON into props should not accept function-typed input from it.
- [The join change types `{f g}` as a sequence of `<function ... />: object*` where it was
  `object+`] → Strictly narrower and still satisfies `object+`. Snapshot tests that pin the old
  text are updated.
- [The checker no longer relates a function's parameters to the site it is bound at, so a host can
  be handed a function it cannot call sensibly, for example one whose parameters are not
  expressible as a tool schema] → That is the point of the type. The host validates when it
  normalizes (`add-agent-host-package` reports inexpressible parameter types as diagnostics).
- [A 0.5.0 runtime given a module that uses the type] → Refused at preparation with
  `nx-ir-required-feature` naming `function-reference-type-v1`. ReachMe bumps its pin in
  `add-agent-tool-loop`.
- [Two spellings of one wire shape in TypeScript, `NxFunctionRef` for a function reference type and
  an arrow type for a function type with parameters] → Accepted for now; see Open Questions.

## Migration Plan

Additive. A compiler with this change emits identical artifacts for programs that do not write
`...`. Release with the next NX minor; the TypeScript runtime, the Rust runtime and the compiler
ship together, as for every required feature. Rollback is reverting the change; no stored artifact
depends on it until a host compiles a program that uses the type.

## Open Questions

- **Should the prelude name the widest type, as `Function`?** Default taken: no (D2). It can be
  added later without breaking a program, if the written-out form proves a burden.
- **Should the runtimes check a host-supplied record's result against `R`?** The image carries the
  result type and a function's declared result, so it is possible. Default taken: no, to match
  function-typed sites, where nothing is re-checked. Doing it for one kind and not the other would
  be inconsistent, and doing it for both is a separate change.
- **Should a function type with stated parameters also map to the record type in generated
  TypeScript?** It would make the two mappings consistent and match what is on the wire. Default
  taken: leave the arrow mapping alone, since changing it is a separate, breaking typegen change.
- **Should a `Function` record supplied by a host be limited to exported functions?** Default
  taken: no, keep the existing rule for function-typed sites.
- **Should `...` be allowed elsewhere, for example as a type argument (`<Holder T=... />`)?**
  Default taken: no. Nothing needs it, and the IR erases type arguments.

None of the open decisions in §9 of the architecture document touches this change.
