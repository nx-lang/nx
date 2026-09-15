## ADDED Requirements

### Requirement: Registration works against a host-supplied Monaco from 0.52 onward
The Monaco integration SHALL register NX against whatever Monaco namespace the host hands it,
whether imported as an ES module or exposed as a global by Monaco's AMD loader, and SHALL declare
compatibility with Monaco 0.52.0 and later. It SHALL not import `monaco-editor` itself at runtime.

#### Scenario: An AMD-loaded Monaco is registered
- **WHEN** a host loads Monaco 0.52 through its AMD loader and passes the resulting global
  namespace to the registration
- **THEN** NX SHALL be registered, highlighted, and served hover and completion through that
  namespace, with no second copy of Monaco loaded

#### Scenario: The peer range admits 0.52
- **WHEN** a host that pins Monaco 0.52 installs the package
- **THEN** the package manager SHALL report no unmet peer dependency for `monaco-editor`

#### Scenario: A host theme that Shiki does not know keeps working
- **WHEN** the host's active Monaco theme is not one of the themes the integration loads
- **THEN** the editor SHALL keep the host's theme and NX tokens SHALL still be colored by the
  integration's first loaded theme
