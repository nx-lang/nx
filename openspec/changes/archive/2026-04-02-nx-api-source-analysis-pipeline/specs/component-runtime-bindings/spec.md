## ADDED Requirements

### Requirement: Source-based component lifecycle calls gate on shared analysis
Source-based component lifecycle entry points SHALL run the shared source-analysis pipeline before
component-specific validation or interpreter execution. If source analysis returns any error
diagnostics, initialization and dispatch SHALL return those diagnostics and SHALL not produce
component lifecycle results.

#### Scenario: Component initialization returns aggregated static diagnostics
- **WHEN** `initialize_component_source` is called with source that contains both a lowering error and a component state type error
- **THEN** the call SHALL return both static diagnostics from the shared analysis phase
- **AND** the call SHALL not return rendered output
- **AND** the call SHALL not return a component-state snapshot

#### Scenario: Component dispatch rejects static errors before snapshot processing
- **WHEN** `dispatch_component_actions_source` is called with source that contains both a lowering error and a type error and the host also supplies an invalid snapshot
- **THEN** the call SHALL return the shared source-analysis diagnostics for the source
- **AND** the call SHALL not attempt to interpret the component dispatch
- **AND** the call SHALL not return an invalid-snapshot runtime diagnostic for that request
