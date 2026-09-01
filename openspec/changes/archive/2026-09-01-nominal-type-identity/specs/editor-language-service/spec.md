## MODIFIED Requirements

### Requirement: Language service exposes conservative completions
The language service SHALL expose completion items based on the current document context. MVP
completion sources SHALL include NX keywords, primitive type names, visible top-level declarations,
component or tag names, component property names, and the members or payloadless cases of a
property's declared type, when the current syntax context and available metadata make those
completions valid.

Element and property lookup SHALL follow the import graph of the document being edited, preserving
import aliases and the identity of the declaring module. Completions SHALL NOT be drawn from
declarations that are not visible to that document, and SHALL NOT be selected by matching a
declaration name against an authored tag as plain text.

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
- **THEN** the language service SHALL include the constant cases of that union, as bare names
- **AND** it SHALL NOT include lexically visible variables or unrelated declarations, which cannot
  appear unbraced in that position

#### Scenario: Property value position without a nominal type offers no member completions
- **WHEN** a client requests completions after `=` in a property value position whose declared type
  is not a discriminated union, or whose element or property is unknown
- **THEN** the language service SHALL offer no contextual member completions for that position

#### Scenario: Member completions are offered for an element reached through an import alias
- **WHEN** a client requests completions after `=` on a property of an element written under an
  import alias, such as `<ui.Img fit=`
- **THEN** the language service SHALL resolve the element through that alias
- **AND** it SHALL offer the members of the property's declared type

#### Scenario: Declaration completions do not offer the removed enum keyword
- **WHEN** a client requests completions in declaration position
- **THEN** the language service SHALL NOT offer `enum` as a declaration keyword

#### Scenario: Completions are not drawn from declarations the document cannot see
- **WHEN** another document in the workspace declares a type sharing a name with one used by the
  document being edited, and is not imported by it
- **THEN** the language service SHALL NOT offer members of that other declaration
