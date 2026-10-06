## ADDED Requirements

### Requirement: Generated type surfaces map the function reference type
For an exported contract member typed by a function reference type, `<function ... />: R` or an
alias of one, `typegen` SHALL emit the type of the rendered
`Function` record in both languages, whatever the result type, because that record is what a host
reads and supplies at such a member. The C# emitter SHALL type the member
`global::NxLang.Nx.NxFunctionRef`, the managed function reference type `dotnet-binding` provides,
and SHALL emit no declaration of it. The TypeScript emitter SHALL type the member `NxFunctionRef`,
an object type whose `$type` is the literal `"Function"` and whose `module` and `name` are strings,
and SHALL emit that type exactly once per generation where the output references it: into the
shared helper module beside `NxRecord` for library output, and inline for single-file output.
Output that references no such member SHALL be unchanged.
Occurrences and the optional mark over the type SHALL map as `typegen` maps them over any type.
The derived update and property companions of a declaration with such a member SHALL be generated,
typing the member the same way. A default on such a member SHALL NOT be emitted as a host default,
because it is not a literal. The
mapping of a function type with stated parameters SHALL be unchanged.

#### Scenario: C# types a function reference member as the function reference
- **WHEN** C# types are generated for `export type Tool = { fn: <function ... />: object* fallback?: <function ... />: object* all?: (<function ... />: object*)+ }`
- **THEN** `Fn` SHALL be typed `global::NxLang.Nx.NxFunctionRef`
- **AND** `Fallback` SHALL be that type made nullable and `All` a nullable list of it
- **AND** the output SHALL NOT declare a type named `NxFunctionRef`

#### Scenario: A stated result maps to the same record type
- **WHEN** C# and TypeScript types are generated for `export type Args = { q:string } export type Tool = { build: <function ... />: Args }`
- **THEN** `Build` SHALL be typed `global::NxLang.Nx.NxFunctionRef` in C#
- **AND** `build` SHALL be typed `NxFunctionRef` in TypeScript

#### Scenario: TypeScript single-file output declares NxFunctionRef once
- **WHEN** TypeScript types are generated for a single file containing `export type Tool = { fn: <function ... />: object* fallback?: <function ... />: object* all?: (<function ... />: object*)+ }`
- **THEN** the output SHALL declare `NxFunctionRef` once, with `$type: "Function"`, `module: string` and `name: string`
- **AND** `Tool` SHALL declare `fn: NxFunctionRef`, an optional `fallback` typed `NxFunctionRef`, and an optional `all` typed `NxFunctionRef[]`

#### Scenario: TypeScript library output declares NxFunctionRef in the helper module
- **WHEN** a library has two modules that each export a record with a field of a function reference type
- **AND** a caller requests TypeScript output
- **THEN** the helper module SHALL export `NxFunctionRef` once
- **AND** each generated module SHALL import it from the helper module

#### Scenario: Output without a function reference member is unchanged
- **WHEN** NX source declares no member typed by a function reference type
- **THEN** the generated C# and TypeScript SHALL be what they were before the type existed, including for a member declared at a function type with stated parameters

#### Scenario: A generated contract round-trips a rendered value
- **WHEN** a .NET host evaluates `let double(n:int): int = {n * 2} let root() = <Tool fn={double} />` from `main.nx` into the generated `Tool` type
- **THEN** `Fn` SHALL expose the module `main.nx` and the name `double`
- **AND** serializing the value through MessagePack and through JSON SHALL preserve both
