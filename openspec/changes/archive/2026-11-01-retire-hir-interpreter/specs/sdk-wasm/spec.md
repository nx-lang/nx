## ADDED Requirements

### Requirement: WASM compiler SDK leaves execution to an IR runtime
The WASM compiler SDK SHALL NOT expose `evaluateNx()` or its native evaluation export. It SHALL
retain IR generation and `formatValue()` without linking the retired interpreter. Removal of a
native export SHALL be reflected in a new ABI version and corresponding wrapper compatibility checks.

#### Scenario: Compile execute and format are separate operations
- **WHEN** a host builds source, executes the emitted IR in a runtime and formats the canonical result
- **THEN** the formatter SHALL return annotated NX text without executing source

#### Scenario: An older wrapper is rejected
- **WHEN** the wrapper built by this change loads a module that still exports `nx_wasm_program_evaluate_nx`
- **THEN** loading SHALL fail with the version mismatch error, naming versions 12 and 11
- **AND** it SHALL fail before calling any export whose shape changed

## MODIFIED Requirements

### Requirement: The wasm SDK reaches standard libraries with no registry
Every wasm SDK entry point that analyzes or builds NX source SHALL resolve standard libraries from
the source the module carries: a single-source build, a workspace build, workspace validation, a
library load, a language snapshot and the in-process language service. No registry, build context,
document or option SHALL be needed for an `import "@nx/<name>"` to resolve, and the root of a
standard library SHALL be accepted wherever the SDK accepts implicit imports. A program that links a
standard library SHALL emit that library's module image by its identity through the same emit call
that emits any library module, with metadata carrying the identity and an image that records the
library's version, which an IR runtime answers when it prepares the image. The
answers SHALL equal the Node SDK's for the same inputs.

#### Scenario: A single source imports the agent library
- **WHEN** a caller builds an artifact from the source `import "@nx/agent"` followed by `let root() = { <Agent name="support">Be brief.</Agent> }`, with no build context
- **THEN** the build SHALL succeed
- **AND** the emitted IR, executed through an IR runtime, SHALL produce an Agent with instructions `Be brief.` and name `support`

#### Scenario: Tenant source uses the agent library through a host's libraries
- **WHEN** a host loads `libraries/chat-link`, whose module contains `import "@nx/agent"`, into a registry
- **AND** builds a tenant workspace against a build context from it with implicit imports `["libraries/chat-link", "@nx/agent"]`
- **THEN** the tenant source SHALL use both libraries' declarations with no import line

#### Scenario: The library image is emitted beside the entry
- **WHEN** a caller asks that artifact for NX IR for the entry and `@nx/agent/agent.nx`
- **THEN** the result SHALL carry two artifacts, and the second's metadata SHALL name `@nx/agent/agent.nx`
- **AND** the second image, prepared by `@nx-lang/ir-runtime`, SHALL answer the library's version, equal to the version the entry image records for it
- **AND** `@nx-lang/ir-runtime` SHALL prepare both, link the entry against the library image and evaluate `root`

#### Scenario: Parity with the Node SDK
- **WHEN** the same workspace importing `@nx/agent` is built through the wasm SDK and the Node SDK
- **THEN** the emitted entry and library images SHALL be byte-identical and the diagnostics equal

### Requirement: Program artifacts spell runtime values as annotated NX text
A program artifact built by the wasm SDK SHALL offer `formatValue(value, options)`, which SHALL take
a value in the canonical JSON output format, as the IR runtimes return it, and return an
`NxValueText` annotated against the artifact's declarations and type information. Formatting
SHALL NOT execute the program. An
`ActionHandler` record, which the runtimes return for a handler, SHALL be spelled as that record,
with its `action` and `token` properties, rather than refused. Canonical JSON does not tell a whole
`float64` from an `int`, nor a constant union case from a string, so a number or a string SHALL be
read by the type its position declares, a record's property, in the entry module or a library the
program links, or the type `options.type` names for the whole value. A whole number SHALL be an
`int` when no position declares its type or the position's type is a type parameter, which the
value does not carry. `options.state` SHALL name a component of the entry module whose
state the value is, and the value SHALL then be spelled one `name=value` per line, in the order the
component declares its state fields, each read by its field's type and declared at that field. A
value that is not in the canonical format, or that has no NX spelling for another reason, SHALL fail
with an `NxEvaluationError` whose diagnostic says where in the value the problem is, and the
artifact SHALL remain usable afterwards. The program is not evaluated.

#### Scenario: A rendered component
- **WHEN** a host builds a program declaring `external component <Button label:string emits {
  Tapped { } } />` and calls `formatValue` with `{ "$type": "Row", "content": [{ "$type":
  "Button", "label": "Add", "onTapped": { "$type": "ActionHandler", "action": "Button.Tapped",
  "token": "h1-1" } }] }`
- **THEN** the text SHALL spell the `Row` element holding a `Button` element whose `onTapped`
  property is `<ActionHandler action="Button.Tapped" token="h1-1" />`
- **AND** the `Button` node's declaration SHALL be the span of its declaration

#### Scenario: The same text as root's
- **WHEN** a host evaluates a program's `root` through an IR runtime and passes the result to
  `formatValue`, and that value holds no action handler and no number outside a declared property
  or in a property typed by a type parameter
- **THEN** the text SHALL spell that canonical value using the declared types
- **AND** formatting SHALL NOT execute root or any component

#### Scenario: A whole float
- **WHEN** a program declares `type Size = { width:float64 }` and a host formats
  `{ "$type": "Size", "width": 3 }`
- **THEN** the text SHALL be `<Size width=3.0 />`

#### Scenario: A number with its type given
- **WHEN** a host formats `3` with `{ type: "float64" }`, and `3` with no options
- **THEN** the texts SHALL be `3.0` and `3`

#### Scenario: A component's state
- **WHEN** a program declares `component <Counter />` with `state { count:int = 0 ratio:float64 =
  1 }` and a host formats `{ "ratio": 1, "count": 4 }` with `{ state: "Counter" }`
- **THEN** the text SHALL be `count=4` and `ratio=1.0` on two lines, each a property node declared
  at its state field

#### Scenario: A value outside the canonical format
- **WHEN** a host calls `formatValue` with a record whose `$type` is not a string
- **THEN** it SHALL throw an `NxEvaluationError` naming the value's pointer
- **AND** the artifact SHALL answer later calls

### Requirement: Annotated text nodes carry their value's pointer
Every node of an `NxValueText` the wasm SDK returns, from `formatValue()`, SHALL
carry `pointer`: the RFC 6901 JSON pointer of its value within the value's canonical JSON form. A
property node SHALL carry the pointer of the property's value, the whole value's pointer SHALL be
the empty string, and an item of a sequence SHALL be addressed by its index.

#### Scenario: Pointers into a rendered output
- **WHEN** a host formats the `Row` of the previous requirement
- **THEN** the `Button` record's node SHALL carry `/content/0`, the `onTapped` property's node and
  the `ActionHandler` record's node SHALL both carry `/content/0/onTapped`, and the `Row` node SHALL
  carry the empty string

## REMOVED Requirements

### Requirement: Program artifacts evaluate root to annotated NX text
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Generate IR, execute through an IR runtime, and pass canonical values to formatValue. Rich CLI spelling is handled separately from canonical-value formatting.
