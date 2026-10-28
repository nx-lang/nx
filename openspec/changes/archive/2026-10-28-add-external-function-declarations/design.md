## Context

The archived math implementation synthesizes HIR function signatures from a closed MathOp table. NX already attaches CommonMark `///` documentation to ordinary declarations and displays it through editor services. External components establish the modifier convention.

## Goals / Non-Goals

**Goals:** source-authored math signatures/docs, source declaration locations, strict registry/source agreement and unchanged execution contracts.

**Non-Goals:** arbitrary FFI/callback registration, new IR nodes, additional math operations, executable external declarations without trusted bindings.

## Decisions

- Parse `[visibility] external name(parameters):Result` as a bodyless function using ordinary signature nodes. Also admit the existing element signature form; a registered implementation must match its declared calling form. Ordinary functions still require bodies and external functions cannot supply one. For recovery, the parser recognizes external declarations with a missing result annotation or an authored body; syntax validation diagnoses those shapes without consuming subsequent declarations.
- Keep existing HIR Function bodies executable after lowering: ordinary lowering refuses unregistered external declarations, while compiler-owned library lowering receives its module's closed runtime operation list and attaches validated StandardMath bodies. This avoids introducing a broad host-binding IR contract or placeholder executable bodies.
- Validate parameter names/order, exact float64 types, arity, calling form, export visibility and required/default/content flags against each operation; validate that every registered operation has exactly one external declaration. Diagnostics point to source declarations.
- Preserve parsed docs and source spans for declarations/parameters. Implementation argument nodes and numerical bodies remain synthetic, so numerical failures retain the authored caller's origin rather than a fictitious source implementation.
- Math NX source is the public documentation/signature authority. Runtime operation descriptors continue to define the numerical implementation contract and participate in versioning; mismatches fail library construction.
- Regenerate only math-linked IR fixtures, since math source/content version changes. Math-free images must stay unchanged.

## Risks / Trade-offs

- Grammar changes can alter recovery/highlighting → regenerate parser metadata and run parser/lowering/editor suites.
- Source/registry drift → exact signature and missing/duplicate binding tests.
- Real declaration spans can steal math error locations → preserve synthetic body spans and rerun call-origin tests.

## Migration Plan

Ship compiler and runtime packages together as already documented. Existing math imports and calls are unchanged; source edits intentionally change the math library version. This change has no external deployment or migration step.

## Syntax revision

The user selected `external name(parameters):Result`, dropping `let`. `external` is the declaration
keyword for runtime-provided functions; `let` continues to declare authored functions and values.
The incorrect `external let` form is rejected; it never shipped and has no compatibility or deprecation status. Trusted binding, documentation and error
recovery contracts are unchanged. Historical review entries describe the syntax reviewed at that time.

The source external marker is retained explicitly on HIR functions, including unregistered
declarations, so hover and completion show the correct keyword independently of implementation
body shape. The incorrect form is recognized only as an error-recovery token with horizontal whitespace
around `let`; validation points to that keyword and still rejects it. This cannot absorb a `let`
declaration on the following line. TextMate recovery recognizes declaration-shaped external
prefixes, preserving ordinary identifiers, optional properties and prose.
