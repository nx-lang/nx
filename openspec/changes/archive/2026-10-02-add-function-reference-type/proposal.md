## Why

A function type in NX names one signature, and a function satisfies it only when the type supplies
every parameter the function declares (`function-types`, "A function satisfies a function type by
parameter name"). That rule is right for a template, where NX or the host calls the value with a
fixed set of arguments. It leaves no way to declare a property that takes *any* function, whatever
its parameters. `<function />: R` is not that type: it supplies no parameters, so only a function
that declares none satisfies it. The only type that accepts every function today is `object`, which
also accepts a string, a number and a record, so a property declared at it is unchecked:
`fn:object` takes `fn={double}` and `fn="double"` alike.

The agent library (`add-agent-library`) needs exactly that property. `FunctionTool.function` holds
the function a host exposes to a model, and `HttpTool.arguments` holds the function that builds a
request's arguments. In both, the host calls the function, with arguments it has validated against
the function's *own* declared parameters, so there is no shared parameter list to write down. The
second also has a result the library fixes: an arguments function returns `HttpArguments`. Without
a type for "a function of any parameters", the library would have to name functions by string and
lose the check that the name refers to a function at all; with a type that says nothing about the
result, the `HttpArguments` check would move from the compiler to the host.

## What Changes

- **A function type may leave its parameters unspecified.** `<function ... />: R` is a function
  type whose parameter list is the token `...`: the type of a function of any parameters whose
  result satisfies `R`. This is the *function reference type*. `...` stands for the whole
  parameter list and cannot be mixed with parameter definitions. The result type is required, as in
  every function type, and takes an occurrence suffix on the same terms. This is the form Python
  writes `Callable[..., R]` and Erlang `fun((...) -> R)`.
- **Any function at all is `<function ... />: object*`.** Every result type satisfies `object*`, so
  every function value satisfies that type. There is no built-in name for it: an author writes the
  type with the result the site needs, and a library that wants a short name declares its own
  alias. `Function` is not a type name, as it is not today.
- **What satisfies it.** A function type of any parameters satisfies `<function ... />: R` when its
  result satisfies `R`, and so does a function reference type whose result satisfies `R`. Nothing
  else does. A function reference type satisfies another whose result it satisfies, and `object`,
  and never a function type with stated parameters: the parameters are not known, so they cannot
  be recovered.
- **The value is opaque in NX.** NX code may bind such a value to a field, property, parameter,
  `let` or result, put it in a sequence, and compare it with `==`, `!=` and a match pattern, on the
  terms `function-values` already gives function values. It may not invoke it: `<f ... />` and
  `f(...)` on a binding of a function reference type are rejected with a diagnostic that says why
  and names the fix (declare the binding at a function type with its parameters). There is no cast
  to a function type with parameters.
- **Two unlike functions join to `<function ... />: object*`.** Where inference joins two function
  types and neither satisfies the other, the result is that type rather than `object`, so
  `{findPlans listRegions}` can be bound to `tools: (<function ... />: object*)+`. The join
  satisfies `object`, so nothing that checked before stops checking.
- **NX IR carries the type.** The type table gains a kind, `anyFunction` (`6`), whose one operand is
  the result type. A module whose type table holds it lists a new required feature,
  `function-reference-type-v1`, so a runtime that predates the type refuses the module by name.
  The schema version stays 5. No node kind changes: a function still reaches an expression as a
  `reference` node.
- **Both IR runtimes validate the boundary.** At a site typed by a function reference type the
  TypeScript and Rust runtimes accept a function value of the program, and a host-supplied
  `{ "$type": "Function", module, name }` record that names a function declaration of the linked
  program. Anything else is refused. The value renders as the same record. The runtimes do not
  re-check the result type of a host-supplied record, as they re-check no parameter at a
  function-typed site today. `callFunction` and `call_function` are unchanged: they already bind
  arguments by name, validate each against the function's declared parameter type, and fill
  defaults and optional parameters. The TypeScript runtime exports a type for the record,
  `NxFunctionRecord`.
- **Generated types.** C# types a member of a function reference type `NxLang.Nx.NxFunctionRef`,
  which a function-typed member already uses. TypeScript types it `NxFunctionRef`, an object type
  with `$type: "Function"`, `module` and `name`, emitted once beside `NxRecord` when referenced.
- **Docs.** `nx-grammar.md`, `nx-grammar-spec.md`, the language reference (`types.md`,
  `functions.md`), `docs/nx-ir-format.md` and the TypeScript runtime README describe the type.

No program that compiles today is newly rejected, and no artifact of a program that does not write
`...` changes. `...` is a new token in one position that was a parse error before.

Out of scope: a built-in or prelude name for the type; `...` beside stated parameters
(`<function Item:Contact ... />: R`), and `...` anywhere outside a function type's parameter list;
invoking a function reference value from NX; reading a function's parameters, result type or
documentation from NX or from the runtime (`add-declaration-schema-export` owns that); a `.NET` or
Node API for calling a function value; changing how a function type with stated parameters maps to
TypeScript.

## Capabilities

### New Capabilities

- `function-reference-type`: the function type with unspecified parameters, `<function ... />: R`
  — its spelling, what satisfies it, what NX code may and may not do with a value of it, the join
  of unlike function types, and its display.

### Modified Capabilities

- `nx-ir-format`: a type kind for the function reference type, and the
  `function-reference-type-v1` required feature.
- `typescript-ir-runtime`: boundary validation and rendering at a site of a function reference
  type, reading the new type kind and feature, and the exported `NxFunctionRecord` type.
- `rust-ir-runtime`: the same boundary validation and rendering, with the TypeScript runtime's
  results.
- `cli-code-generation`: the TypeScript and C# mapping of a member of a function reference type.

## Impact

- **Dependencies within the agent work.** This change depends on none of the other eight.
  `add-agent-library` depends on it for `FunctionTool.function`, which it declares
  `<function ... />: object*`, and `HttpTool.arguments`, which it declares
  `<function ... />: HttpArguments` so that NX checks the result. `add-declaration-schema-export`
  works from the program artifact at compile time, not from the IR, and reports a member of a
  function reference type with `schema-inexpressible-type`; whichever of the two changes lands
  second adds that test (task 3.6 there, task 7.5 here). `add-agent-host-package` reads the
  `Function` record from an evaluated `Agent` value and calls it with `callFunction`. The
  `@nx/agent` library image lists `function-reference-type-v1`, so every runtime that loads it must
  support the feature. In ReachMe, `add-agent-function-tools` and `add-agent-http-tools` depend on
  it through those. `add-ir-runtime-evaluation-budget` is independent.
- `crates/nx-syntax`: `grammar.js` gains `...` as the parameter list of a function type, the
  parser is regenerated, and validation rejects `...` beside parameter definitions.
  `BUILTIN_TYPE_NAMES` is unchanged.
- `crates/nx-hir`: the function type reference records that its parameters are unspecified, and
  `spell_function_type` spells it.
- `crates/nx-types`: a function type with unspecified parameters in `ty.rs` (compatibility,
  display, join), its resolution in `infer.rs`, and the two invocation diagnostics.
- `crates/nx-codegen`, `crates/nx-ir`: the type kind, the feature, image validation and
  `ir explain`; a conformance program under `specs/ir-conformance/`. Generated JavaScript treats the
  type as it treats a function type, with no boundary schema.
- `crates/nx-ir-runtime`, `runtime/typescript`: the type kind in the readers and in boundary
  normalization; `NxFunctionRecord` exported from `runtime/typescript/src/index.ts`.
- `crates/nx-interpreter`: the same acceptance while the HIR interpreter exists (see design.md on
  sequencing with `retire-hir-interpreter`).
- `crates/nx-cli/src/typegen`: both language backends and the TypeScript helper module.
- `crates/nx-language-service`: no code expected; a hover and a highlighting test for the new
  spelling.
- `bindings/dotnet`: a test that a member of a function reference type round-trips as
  `NxFunctionRef`. No new managed type.
- Docs listed above. No change to the prelude, or to `bindings/wasm`, `bindings/node` or
  `bindings/c`, which pass images and values through.
