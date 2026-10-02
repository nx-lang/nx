## ADDED Requirements

### Requirement: Rust runtime accepts any function at a function reference site
The Rust runtime SHALL read the function reference type kind, validating its result operand, and
the `function-reference-type-v1` feature, and SHALL normalize, validate, compare and render a value
at a site of a function reference type with the rules `typescript-ir-runtime` states for that
site: a function value of the linked program is accepted, a host-supplied `Function` record is
accepted when it names a function declaration of the linked program, whatever its signature, a
record that names none is refused with `nx-ir-function-value` naming the function, and any other
value is refused with `nx-ir-boundary-type` naming the site. For the same images and the same
inputs the canonical value it returns SHALL equal the TypeScript runtime's, and where that runtime
fails the Rust runtime SHALL fail with a diagnostic of the same code naming the same item. A call
by `Function` record SHALL be unchanged.

#### Scenario: A function reference field renders the Function record
- **WHEN** a prepared program declares, in `main.nx`, `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let root() = <Tool fn={double} />` and a caller evaluates `root`
- **THEN** the result's `fn` field, serialized to JSON, SHALL be `{ "$type": "Function", "module": "main.nx", "name": "double" }`

#### Scenario: A host-supplied record is validated against the program
- **WHEN** a host supplies a `Tool` whose `fn` is a `Function` record naming `double`, one naming `Nope`, and one whose `fn` is the string `"double"`
- **THEN** the first SHALL be accepted
- **AND** the second SHALL fail with `nx-ir-function-value` naming `Nope`
- **AND** the third SHALL fail with `nx-ir-boundary-type` naming `fn`

#### Scenario: The corpus program agrees with the TypeScript runtime
- **WHEN** the Rust runtime runs the function reference conformance program
- **THEN** every entrypoint SHALL produce the recorded result the TypeScript runtime produces

#### Scenario: A damaged function reference entry is refused
- **WHEN** a cell of an `anyFunction` type entry in a corpus image is overwritten with an arbitrary value
- **THEN** preparation SHALL either refuse the image with a diagnostic or read it as a valid image, and SHALL NOT panic
