## ADDED Requirements

### Requirement: TypeScript runtime reports origins through its options
`NxRuntimeOptions` SHALL accept `origins`, an object the runtime fills as the `value-origin`
capability's origins report, the way it fills `usage`. When the call returns, the object's
`entries` SHALL be an array of `{ path, module, start, end }`. An `origins` value the runtime cannot
write to SHALL be refused with `nx-ir-options` before anything runs.

#### Scenario: Reading a report
- **WHEN** a host passes `{ origins: {} }` to `initializeComponent` with an image built with its
  debug section
- **THEN** after the call, `options.origins.entries` SHALL list the origins of the rendered output

#### Scenario: A report that cannot be written
- **WHEN** a host passes a frozen object as `origins`
- **THEN** the call SHALL fail with `nx-ir-options` before evaluating anything
