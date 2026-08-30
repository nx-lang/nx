## MODIFIED Requirements

### Requirement: Language service exposes conservative completions
The language service SHALL expose completion items based on the current document context. MVP
completion sources SHALL include NX keywords, primitive type names, visible top-level declarations,
component or tag names, component property names, and the members or payloadless cases of a
property's declared type, when the current syntax context and available metadata make those
completions valid.

Declaration lookup for these completions SHALL be understood to operate over the workspace snapshot
by declaration name rather than through the document's import graph. A declaration that the edited
document cannot see may therefore be selected when it shares a name with one that it can, and an
element written under an import alias may not be matched at all. Import-aware element and property
lookup is specified separately.

#### Scenario: Type position includes primitive and visible type completions
- **WHEN** a client requests completions in an NX type annotation position
- **THEN** the language service SHALL include primitive NX type names
- **AND** it SHALL include visible record, union, enum, component, or action type names that are
  available in the current workspace snapshot

#### Scenario: Component property position includes property completions
- **WHEN** a client requests completions inside an element opening tag for a known component
- **THEN** the language service SHALL include undeclared properties accepted by that component
- **AND** it SHALL NOT include properties already supplied in that opening tag

#### Scenario: Property value position includes contextual member completions
- **WHEN** a client requests completions immediately after `=` in a property value position whose
  declared type is an enum or a discriminated union
- **THEN** the language service SHALL include the members of that enum, or the payloadless cases of
  that union, as bare names
- **AND** it SHALL NOT include lexically visible variables, which cannot appear unbraced in that
  position

#### Scenario: Property value position without a nominal type offers no member completions
- **WHEN** a client requests completions after `=` in a property value position whose declared type
  is not an enum or a discriminated union, or whose element or property is unknown
- **THEN** the language service SHALL offer no contextual member completions for that position
