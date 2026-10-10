## ADDED Requirements

### Requirement: Rust runtime reports origins through its options
`RuntimeOptions` SHALL have `origins: Option<Arc<Origins>>`, `None` by default, which the runtime
fills as the `value-origin` capability's origins report, the way it fills `usage`. `Origins` SHALL
expose the last call's entries, each with the record's JSON pointer and a `SourceSpan` of its
origin. No method's signature or result type SHALL change. The serialized form of a component
instance SHALL hold no origins, so an instance `Program::restore_component_instance` restores SHALL
hold none: a record of its state has an origin again only once a call given a report builds it
again.

#### Scenario: A restored instance
- **WHEN** a host initializes a component whose state holds a record the program built with an
  origins report, serializes the instance, restores it and dispatches a batch with a report that
  leaves the record as it is
- **THEN** the record SHALL have no origin in the rendered output

#### Scenario: Reading a report
- **WHEN** a host sets `origins` and calls `Program::initialize_component` with an image built with
  its debug section
- **THEN** after the call the report's entries SHALL list the origins of the rendered output, equal
  to what the TypeScript runtime reports for the same call
