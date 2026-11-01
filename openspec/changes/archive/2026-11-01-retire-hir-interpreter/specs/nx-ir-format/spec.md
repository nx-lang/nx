## MODIFIED Requirements

### Requirement: A conformance corpus defines NX IR behavior
The repository SHALL hold a corpus of NX programs, each with its expected artifact for every emitted
module, the explained text of each expected artifact, and its expected evaluation results for every
entrypoint the corpus names. A corpus program MAY also name component lifecycles: a component to
initialize, optional props, and ordered batches to dispatch against it, whose entries are written
as a host would send them, handler tokens included. The expected results SHALL then record the
rendered output of initialization and, for each batch in turn, the rendered output and the ordered
effects, all as canonical values with their handler tokens, as the Rust IR runtime produces them.
The expected results are committed data: a regeneration that changes one SHALL be a reviewed change
of behavior, and the emitter's tests SHALL fail when a committed result differs from what the Rust
IR runtime produces. The emitter's tests SHALL check that each program emits its expected artifact
byte for byte, SHALL report a difference as explained text, and SHALL check that each committed
explained text is the explanation of its committed artifact. Every supported runtime SHALL check that it evaluates each
expected artifact to the expected results, lifecycles included. One regeneration command SHALL
write the artifacts, their explained text and the expected results together. The corpus SHALL cover
every node kind, type kind and declaration kind, cross-artifact references, and a program emitted
without its debug section. For every corpus program, the artifact emitted without a debug section
SHALL be at most six times the UTF-8 length of the program's own module source.

#### Scenario: Emitter output is pinned by the corpus
- **WHEN** the emitter is changed so that a corpus program's artifact differs
- **THEN** the emitter's corpus test SHALL fail naming the program and the difference as explained
  text

#### Scenario: A change to the emitter is readable in review
- **WHEN** the emitter is changed so that a corpus program's artifact differs
- **AND** the corpus is regenerated
- **THEN** both the image and its explained text SHALL be updated
- **AND** the explained text's diff SHALL name what changed about the program

#### Scenario: Stale explained text is caught
- **WHEN** a corpus image is committed without regenerating its explained text
- **THEN** the emitter's corpus test SHALL fail naming the program

#### Scenario: A runtime is checked against the corpus
- **WHEN** a runtime evaluates a corpus artifact's named entrypoint
- **THEN** its canonical value SHALL equal the expected result recorded for it

#### Scenario: A runtime's dispatch is checked against the corpus
- **WHEN** a runtime initializes a corpus lifecycle's component and dispatches its batches in order
- **THEN** each rendered output, its tokens included, and each effect list SHALL equal the values
  recorded for it

#### Scenario: The size budget is enforced
- **WHEN** a corpus program of 8,000 bytes of source is emitted without a debug section
- **THEN** the artifact SHALL be at most 48,000 bytes

### Requirement: NX IR makes float32 arithmetic explicit
NX IR SHALL encode an addition, subtraction, multiplication or division whose checked type is
`float32` with its own binary operator: `fadd32` (`16`), `fsub32` (`17`), `fmul32` (`18`) or
`fdiv32` (`19`). A runtime SHALL evaluate each by computing the operation on its operands and
rounding the result to the nearest `float32`, so that a runtime which carries every number as a
`float64` computes the exact `float32` result, and widening or comparing that value
observes no difference. A remainder whose checked type is `float32` SHALL stay `mod`, since a
`float32` remainder is exact. A runtime SHALL round a number the host supplies at a `float32` site
to the nearest `float32`, and SHALL refuse an integer there that a `float32` does not represent
exactly, so every `float32` value it holds is one.

#### Scenario: A float32 product is emitted as fmul32
- **WHEN** NX source contains `let f(v:float32) = v * 3` and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `fmul32` operator
- **AND** for `let g(v:float32, d:float32) = v % d` the operator SHALL be `mod`
- **AND** for `let h(v:float32, x:float64) = v * x` the operator SHALL be `mul`

#### Scenario: A widened float32 product agrees with the Rust IR runtime
- **WHEN** `let scaled(v:float32): float64 = { v * 3 }` is evaluated by the TypeScript IR runtime
  with `v` holding the `float32` nearest `2.3`
- **THEN** the result SHALL be `6.899999618530273`, as the Rust IR runtime computes it
- **AND** `scaled(2.3) == 6.899999618530273` SHALL be `true`

### Requirement: NX IR carries the result type of a function and a value
A function declaration SHALL carry its declared result type, and a value declaration its declared
type, each absent when the source declares none. A runtime SHALL normalize a function's result and
a value's initializer to that type as it normalizes an argument at a parameter, so every engine agrees: `let g(): int+ = { 5 }` returns `[5]`, and a one-item sequence returned where `int?`
is declared is its item. Without a declared type the result SHALL be the body's value as it is.

A function declaration SHALL also carry result flags whose bit 0 is set when its result type,
declared or inferred, is a standalone `T?`. An entry call — a host evaluating the function, by
name or through a function value — SHALL then return an empty result to the host as `null`, as
`occurrence-types` requires; inside the program the empty value SHALL stay the empty array. An
`ir explain` rendering SHALL print a declared type as `: T` after the parameters and the flag as
`(optional result)`.

#### Scenario: A declared result is normalized in every engine
- **WHEN** NX source contains `let g(): int+ = { 5 }`, `let h(o?:int): int? = { for x in o { x } }`, `let k(xs?:int+): int+ = { xs ?? 5 }` and `type Ints = int+ let fives: Ints = { 5 }`
- **AND** the Rust IR runtime, the TypeScript IR runtime and generated JavaScript each evaluate `g()`, `h(1)`, `k({})` and `fives`
- **THEN** every engine SHALL give `[5]`, `1`, `[5]` and `[5]`

#### Scenario: An empty optional result is null at the host
- **WHEN** NX source contains `let maybe(): string? = { if false { "a" } }` and `let inferred() = { if false { 1 } }`
- **AND** NX IR is emitted for the program
- **THEN** both function declarations SHALL set the optional-result flag and only `maybe` SHALL carry a declared result type
- **AND** `evaluateFunction` in the NX IR runtime SHALL return `null` for each
- **AND** the explained text SHALL show `function maybe(): string? (optional result) =` and `function inferred() (optional result) =`

### Requirement: NX IR leaves a parameter a call omits to the function
A function declaration SHALL carry each parameter's default as a node of the function's own frame,
absent when the parameter has none, emitted with the parameters before it bound so that it reads
them through their slots. A call node SHALL carry its arguments by position, where an absent
argument is a parameter the call left out, and the argument list MAY stop before the trailing
parameters it leaves out. A runtime SHALL fill each parameter a call leaves out in the callee: the
default, evaluated in the callee's frame after the parameters before it and normalized to the
parameter's type, or the empty value when the parameter is optional; a required parameter left
out SHALL be refused, naming it. The caller's image SHALL NOT contain the default, so a program
linked against a newer library receives that library's defaults. An `ir explain` rendering SHALL
print a default as `name: T = <node>` and an absent argument as `_`.

#### Scenario: A default travels with the function and a call leaves the parameter out
- **WHEN** NX source contains `let f(a:int, b:int = { a + 1 }, c?:int) = { a }` and `let r() = { <f a=1 c=2 /> }`
- **AND** NX IR is emitted for the program
- **THEN** `f`'s second parameter SHALL carry a default node and its first and third none
- **AND** the call in `r` SHALL carry three arguments, the second absent
- **AND** the explained text SHALL show `function f(a: int, b: int = (a add 1), c?: int)` and `f(1, _, 2)`

#### Scenario: Every engine fills a parameter from the callee's module
- **WHEN** the `parameter-defaults` corpus program is evaluated by the Rust IR runtime, the TypeScript IR runtime and generated JavaScript
- **THEN** every engine SHALL give the recorded result for each entrypoint, including a default that reads a value private to the called function's module
