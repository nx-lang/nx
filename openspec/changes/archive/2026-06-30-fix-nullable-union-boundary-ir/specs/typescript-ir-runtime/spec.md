## ADDED Requirements

### Requirement: TypeScript IR runtime accepts values produced by valid generated IR
The TypeScript IR runtime SHALL normalize record, union, and component values produced inside the
same prepared IR program with the same effective schema used for public boundary inputs. For valid
IR emitted from successfully analyzed source, runtime evaluation SHALL NOT fail with
`nx-ir-boundary-*` diagnostics for nullable union absence or content-property fields that were
valid in native evaluation.

#### Scenario: Nullable union absence passes runtime normalization
- **WHEN** a prepared IR program evaluates a nullable union field to `null`
- **THEN** the TypeScript IR runtime SHALL accept the value for that nullable union field
- **AND** it SHALL return `null` in canonical output

#### Scenario: Invalid undeclared union case remains rejected
- **WHEN** a host boundary input or malformed IR value supplies `$type: "FlowCompletion.undefined"` for a `FlowCompletion` union that does not declare `undefined`
- **THEN** the TypeScript IR runtime SHALL reject the value with an `nx-ir-boundary-type` diagnostic
- **AND** it SHALL NOT reinterpret the undeclared case as `null`

#### Scenario: Content-populated required field does not report missing
- **WHEN** a prepared IR program constructs a component or record value whose required content property is supplied through element body content
- **THEN** the TypeScript IR runtime SHALL apply the content binding before required-field validation
- **AND** it SHALL NOT report `nx-ir-boundary-field` for the content property

#### Scenario: Public boundary validation still rejects malformed host input
- **WHEN** a host supplies JSON with an unknown field, a missing non-nullable required field, or an invalid union discriminator
- **THEN** the TypeScript IR runtime SHALL continue to reject the input with an `nx-ir-boundary-*` diagnostic
- **AND** it SHALL NOT treat this generated-IR parity requirement as permission to accept malformed host input
