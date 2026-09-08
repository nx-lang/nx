## ADDED Requirements

### Requirement: Language service analyzes a snapshot against a program build context
A workspace snapshot SHALL be constructible with a program build context, and every query over that
snapshot — diagnostics, document symbols, hover, and completions — SHALL see the library modules
that context makes visible under the same visibility rules the compiler applies when building a
program with that context. A snapshot constructed without a context SHALL behave as one built with an
empty context.

#### Scenario: Hover resolves a library component
- **WHEN** a snapshot is built with a build context whose registry loaded a library declaring a
  component
- **AND** a client requests hover on a tag naming that component in a snapshot document
- **THEN** the language service SHALL return that component's signature

#### Scenario: Completions offer library declarations
- **WHEN** a client requests completions at a tag-name position in a document of such a snapshot
- **THEN** the language service SHALL offer the library's visible components
- **AND** WHEN a client requests completions at a property-name position inside a tag naming a
  library component, it SHALL offer that component's undeclared properties

#### Scenario: Diagnostics honor library types
- **WHEN** a snapshot document uses a library component correctly
- **THEN** the language service SHALL NOT report the component or its properties as unresolved
- **AND** WHEN the document supplies a property value of the wrong type, the language service SHALL
  report the same type diagnostic the compiler reports

#### Scenario: Snapshot without a context sees no libraries
- **WHEN** a snapshot is built without a build context
- **THEN** names declared only in a library SHALL be treated as unresolved

### Requirement: Editor positions are UTF-16 code unit offsets
The language service SHALL interpret the character component of an incoming text position as a
zero-based offset within its line counted in UTF-16 code units, and SHALL express the character
component of every outgoing position the same way. Byte offsets carried alongside a range SHALL
remain UTF-8 byte offsets into the document text.

#### Scenario: Query after a non-BMP character resolves correctly
- **WHEN** a line reads `let s = "😀" + name` and a client requests hover with the character offset
  that an editor counting UTF-16 units reports for `name`
- **THEN** the language service SHALL resolve the position to `name`

#### Scenario: Returned range counts UTF-16 units
- **WHEN** the language service returns a range for a construct that follows a non-BMP character on
  its line
- **THEN** the range's character offsets SHALL count that character as two units
- **AND** the range's byte offsets SHALL count it as four bytes

#### Scenario: Offsets past the end of a line clamp
- **WHEN** a client requests a position whose character offset exceeds the line's length in UTF-16
  units
- **THEN** the language service SHALL treat it as the end of that line

#### Scenario: ASCII behavior is unchanged
- **WHEN** a document contains only ASCII text
- **THEN** every position and range SHALL be identical to the values reported before this
  requirement
