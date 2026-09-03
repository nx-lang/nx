## MODIFIED Requirements

### Requirement: Language service exposes conservative completions
The language service SHALL expose completion items based on the current document context. MVP
completion sources SHALL include NX keywords, primitive type names, visible top-level declarations,
component or tag names, and component property names when the current syntax context and available
metadata make those completions valid.

#### Scenario: Type position includes primitive and visible type completions
- **WHEN** a client requests completions in an NX type annotation position
- **THEN** the language service SHALL include primitive NX type names
- **AND** it SHALL include visible record, union, component, or action type names that are
  available in the current workspace snapshot

#### Scenario: Component property position includes property completions
- **WHEN** a client requests completions inside an element opening tag for a known component
- **THEN** the language service SHALL include undeclared properties accepted by that component
- **AND** it SHALL NOT include properties already supplied in that opening tag

#### Scenario: Property value position includes contextual member completions
- **WHEN** a client requests completions immediately after `=` in a property value position whose
  declared type is a discriminated union
- **THEN** the language service SHALL include the constant cases of that union as bare names
- **AND** it SHALL NOT include lexically visible variables, which cannot appear unbraced in that
  position

#### Scenario: Property value position without a nominal type offers no member completions
- **WHEN** a client requests completions after `=` in a property value position whose declared type
  is not a discriminated union, or whose element or property is unknown
- **THEN** the language service SHALL offer no contextual member completions for that position

#### Scenario: Declaration completions do not offer the removed enum keyword
- **WHEN** a client requests completions in declaration position
- **THEN** the language service SHALL NOT offer `enum` as a declaration keyword
