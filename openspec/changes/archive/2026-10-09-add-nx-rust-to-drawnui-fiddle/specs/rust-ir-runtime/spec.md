## MODIFIED Requirements

### Requirement: Rust runtime resource-limit diagnostics name the limit
Every diagnostic the Rust runtime reports with the code `nx-ir-resource-limit` SHALL carry the name
of the limit that was reached and, where the limit is a number, its value, as data beside the
message. A limit the TypeScript runtime also has SHALL carry the name the TypeScript runtime gives
it: `maxOperations`, `maxInputSize`, `maxCallDepth`, `maxRangeLength` and `maxExpressionNesting`. The limits only
the Rust runtime has SHALL be named `maxStackBytes` for the native stack an evaluation may use,
`maxValueNesting` for the nesting of a value at the host boundary or in component state, and
`maxComponentDepth` for the nesting of component instances in an instance tree. A diagnostic with
any other code SHALL carry no limit.

#### Scenario: The operation budget is named
- **WHEN** an evaluation fails for an exhausted operation budget of five thousand
- **THEN** the diagnostic's limit SHALL have the name `maxOperations` and the value `5000`

#### Scenario: The two runtimes name a shared limit alike
- **WHEN** the same image fails in both runtimes for runaway recursion under the default options
- **THEN** both diagnostics SHALL name the limit `maxCallDepth` with the value `100`

#### Scenario: A value nested too deeply names its own limit
- **WHEN** a host passes a value nested 300 levels deep
- **THEN** the diagnostic's limit SHALL have the name `maxValueNesting` and the value `256`

#### Scenario: A tree nested too deeply names its own limit
- **WHEN** a visit of an instance tree is for a node 101 component instances deep under the
  default options
- **THEN** the diagnostic's limit SHALL have the name `maxComponentDepth` and the value `100`
