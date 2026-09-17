## ADDED Requirements

### Requirement: NX IR makes primitive-to-text conversion explicit
NX IR SHALL encode the conversion of a primitive value to its canonical text form as its own node
kind, `text`, with the layout `[20, node, str]`: the operand node and the name of the operand's
static primitive type (`int`, `int32`, `int64`, `float32`, `float64` or `boolean`). The emitter
SHALL wrap every non-string operand of a string `+` in a `text` node, so the `concat` binary
operator's operands are always strings and a runtime SHALL NOT need to coerce them. The named type
is what lets a runtime that carries every number as a `float64` print a `float32` operand in the
`float32` text form `implicit-primitive-conversions` requires.

Whether a `+` is emitted as `add` or `concat` SHALL follow the type analysis recorded for the
expression, never the syntactic form of its operands. A numeric operand that analysis widened (an
`int` at a `float64` site, or an `int` operand of a `float64` addition) SHALL be recorded at its own
type with no conversion node, because every supported runtime carries the integer and floating-point
types in one numeric representation and the widening is unobservable there; a `div` or `mod` whose
checked type is floating-point SHALL be emitted as the floating-point operator even when one operand
is an integer.

An `ir explain` rendering SHALL print a `text` node as a readable conversion naming the operand
type.

#### Scenario: A string plus an int is emitted as concat over a text node
- **WHEN** NX source contains `let f(count:int) = "Total: " + count` and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `concat` operator
- **AND** its right operand SHALL be a `text` node naming `int` over the slot of `count`
- **AND** its left operand SHALL be the string constant with no conversion node

#### Scenario: A float32 operand names its type
- **WHEN** NX source contains `let f(w:float32) = w + " px"` and NX IR is emitted
- **THEN** the left operand of the `concat` SHALL be a `text` node naming `float32`

#### Scenario: A string field access is emitted as concat
- **WHEN** NX source contains `type Item = { title:string }` and
  `let f(item:Item) = "Reorder " + item.title`, and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `concat` operator
- **AND** neither operand SHALL be wrapped in a `text` node

#### Scenario: A widened operand carries no conversion node
- **WHEN** NX source contains `let f(n:int, x:float64) = n + x` and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `add` operator whose operands are the two
  slots directly
- **AND** for `let g(n:int, x:float64) = n / x` the operator SHALL be `div`, not `idiv`

### Requirement: NX IR makes float32 arithmetic explicit
NX IR SHALL encode an addition, subtraction, multiplication or division whose checked type is
`float32` with its own binary operator: `fadd32` (`16`), `fsub32` (`17`), `fmul32` (`18`) or
`fdiv32` (`19`). A runtime SHALL evaluate each by computing the operation on its operands and
rounding the result to the nearest `float32`, so that a runtime which carries every number as a
`float64` computes the value the interpreter computes, and widening or comparing that value
observes no difference. A remainder whose checked type is `float32` SHALL stay `mod`, since a
`float32` remainder is exact. A runtime SHALL round a number the host supplies at a `float32` site
to the nearest `float32`, and SHALL refuse an integer there that a `float32` does not represent
exactly, so every `float32` value it holds is one.

#### Scenario: A float32 product is emitted as fmul32
- **WHEN** NX source contains `let f(v:float32) = v * 3` and NX IR is emitted
- **THEN** the body SHALL be a `binary` node with the `fmul32` operator
- **AND** for `let g(v:float32, d:float32) = v % d` the operator SHALL be `mod`
- **AND** for `let h(v:float32, x:float64) = v * x` the operator SHALL be `mul`

#### Scenario: A widened float32 product agrees with the interpreter
- **WHEN** `let scaled(v:float32): float64 = { v * 3 }` is evaluated by the TypeScript IR runtime
  with `v` holding the `float32` nearest `2.3`
- **THEN** the result SHALL be `6.899999618530273`, as the interpreter computes it
- **AND** `scaled(2.3) == 6.899999618530273` SHALL be `true`

## MODIFIED Requirements

### Requirement: NX IR program artifacts are versioned and deterministic
The system SHALL define a versioned NX IR artifact emitted from a successful `ProgramArtifact`, one
per emitted module. The artifact SHALL be a little-endian binary image whose fixed header carries a
format magic, IR schema version `4` and the image's total length, and whose sections carry the
expected runtime ABI `nx-ir-runtime-v2`, the required feature list, the module table, the module's
public entrypoints, and the module's tables and declarations. Equivalent `ProgramArtifact` inputs
with equivalent IR options SHALL produce byte-identical images. A reader SHALL refuse an image whose
magic or schema version it does not implement with a diagnostic naming the version found and the
version supported, and SHALL NOT interpret the bytes that follow the header.

Schema `4` differs from schema `3` by the `text` node kind, `20`, by the `float32` binary operators
`16` to `19`, and by `concat` taking string operands only. Every other kind, table and layout is
unchanged.

#### Scenario: Valid program artifact emits IR metadata
- **WHEN** a caller emits NX IR from a valid `ProgramArtifact` containing a `root()` function
- **THEN** the image SHALL carry schema version `4` and runtime ABI `nx-ir-runtime-v2`
- **AND** the image SHALL list `root` as a function entrypoint
- **AND** the module table's first entry SHALL carry the module's identity and fingerprint

#### Scenario: Equivalent inputs produce byte-identical images
- **WHEN** two equivalent `ProgramArtifact` inputs are emitted as NX IR with the same options
- **THEN** the emitted images SHALL use stable ordering for the module table, tables,
  declarations, fields, properties, entrypoints and nodes
- **AND** the two emitted images SHALL be byte-identical

#### Scenario: Invalid artifact is rejected
- **WHEN** a caller requests NX IR emission for a `ProgramArtifact` containing static error
  diagnostics
- **THEN** IR emission SHALL fail with diagnostics
- **AND** the system SHALL NOT emit a partial artifact

#### Scenario: An older schema is refused
- **WHEN** a runtime is given an image whose schema version is `3`
- **THEN** preparation SHALL fail with a diagnostic naming version `3` and the supported version `4`
- **AND** the reader SHALL NOT interpret the bytes that follow the header

#### Scenario: A document that is not an image is refused
- **WHEN** a runtime is given bytes that do not begin with the NX IR magic
- **THEN** preparation SHALL fail with a diagnostic saying the input is not an NX IR image
