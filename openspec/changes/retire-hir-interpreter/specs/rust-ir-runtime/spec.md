## ADDED Requirements

### Requirement: CLI source and IR execution use the Rust IR runtime
The command line's `run` SHALL accept NX source and NX IR with explicit input-format selection.
Existing source-file invocations SHALL retain their source interpretation. Source execution SHALL
analyze and compile the selected program to IR before execution. Both input kinds SHALL execute
through the Rust IR runtime with shared output and diagnostic handling. The CLI SHALL use a default
budget of 1,000,000 IR operations per invocation. The runtime library's option defaults SHALL remain
unchanged. No second Rust evaluator or fallback to direct HIR execution SHALL exist.

#### Scenario: Running source and its emitted IR produces the same value
- **WHEN** a caller runs source `let root() = { 1 + 2 }` and runs its emitted IR with explicit IR input selection
- **THEN** both executions SHALL produce `3` through the Rust IR runtime

#### Scenario: Imported IR requires supplied modules
- **WHEN** an input IR image imports a module not supplied by the caller's module catalog or the runtime prelude
- **THEN** running SHALL fail with a link diagnostic identifying the missing module
- **AND** SHALL NOT fall back to compiling source or implicitly searching files by the imported identity

#### Scenario: Runtime errors preserve available locations
- **WHEN** a caller runs a program whose root divides by zero
- **THEN** execution SHALL fail with `nx-ir-division-by-zero`
- **AND** SHALL report the failing source location when debug/source information is available
- **AND** SHALL identify the module/declaration without inventing source coordinates when it is unavailable

#### Scenario: CLI operation budget resets per invocation
- **WHEN** one CLI invocation exceeds its default operation budget and another runs a small program
- **THEN** the first SHALL fail with the runtime's operation-limit diagnostic
- **AND** the second SHALL execute with a fresh budget

### Requirement: Tooling APIs separate source compilation from IR execution
The public Rust tooling facade and C, .NET, Node and WebAssembly compiler bindings SHALL NOT expose
legacy source/ProgramArtifact evaluation or component lifecycle wrappers. Building, validation,
schemas, language queries, IR emission and canonical-value formatting SHALL remain available where
already supported. Rust IR execution SHALL remain available through the existing facade runtime
namespace and standalone runtime crate. This change SHALL NOT introduce replacement source-execution
convenience APIs or replacement C/.NET execution bindings.

#### Scenario: A Rust caller composes compilation and execution
- **WHEN** a Rust host builds source and emits IR using the tooling facade
- **THEN** it SHALL be able to prepare, link and execute the images through the facade's IR runtime namespace
- **AND** compilation SHALL NOT execute the source

#### Scenario: Tests do not enlarge the public execution surface
- **WHEN** semantic tests need to execute NX source
- **THEN** they SHALL compose build, emission and IR execution in test-only helpers
- **AND** SHALL NOT require a production source-evaluation export

### Requirement: Rust runtime returns values that keep their NX spelling
Beside `NxValue` results, the Rust runtime SHALL offer results in a value form in which a constant
union case is distinguishable from a string and names its union and case, a function value is
distinguishable from a record and names its declaration, and a `float32` value is distinguishable
from a `float64`. NX text written from such a value SHALL spell a constant case as its qualified
case, a function by its name, and a `float32` in its canonical text form, including where the
value sits at a site typed `object`.

#### Scenario: An enum case prints as a case
- **WHEN** `type ThemeMode = light | dark` and `let root(): object = { ThemeMode.dark }` are
  evaluated and the result is written as NX text
- **THEN** the text SHALL be `ThemeMode.dark` and SHALL NOT be `"dark"`

#### Scenario: A float32 prints as a float32
- **WHEN** a function returning the `float32` nearest to `0.1` is evaluated and the result is
  written as NX text
- **THEN** the text SHALL be `0.1`

### Requirement: Rust runtime behavior is validated against the corpus and source-level expectations
The Rust runtime's tests SHALL prepare, link where the module table requires, and evaluate every
artifact of the conformance corpus, with and without its debug section, and SHALL compare each
named entrypoint's value and each lifecycle's rendered outputs, tokens and effects with the
corpus's expected results. The tests SHALL cut every corpus image at every four-byte boundary and
overwrite every cell of at least one corpus image, and SHALL fail if any such image causes a panic.
The repository's integration tests SHALL also compile NX source and execute emitted IR through
test-only helpers, comparing each result with an expected value written in the test, covering
primitives, arithmetic and its error cases,
conditionals, match expressions, sequences and occurrences, the presence operators, loops and
ranges, recursion and its limit, records and inheritance, type parameters, unions, enums, function
parameters and values, property references, update records and intrinsics, text content, and the
component lifecycle.

#### Scenario: The corpus is part of the runtime's test run
- **WHEN** the Rust runtime's tests run
- **THEN** every corpus entrypoint SHALL be evaluated and every corpus lifecycle initialized and
  dispatched in order
- **AND** a difference from an expected result SHALL fail the run naming the program and the
  entrypoint or lifecycle

#### Scenario: Damage is survived
- **WHEN** the damage tests run
- **THEN** every truncated image SHALL be refused
- **AND** every overwritten image SHALL be refused or evaluated without a panic

#### Scenario: Source-level expectations do not depend on another engine
- **WHEN** a source-level test evaluates an NX program
- **THEN** the expected value SHALL be stated in the test
- **AND** SHALL NOT be computed by evaluating the program another way

## REMOVED Requirements

### Requirement: Rust runtime behavior is validated against the corpus and the interpreter
**Reason**: The interpreter is deleted, so there is no second native engine to compare with.
**Migration**: Replaced by "Rust runtime behavior is validated against the corpus and source-level
expectations", which keeps the corpus and damage tests and replaces the differential run with
tests that state their expected values.
