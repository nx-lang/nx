## ADDED Requirements

### Requirement: A built program artifact reports its warnings
A program artifact that builds SHALL report the diagnostics its build produced without failing —
warnings, info and hints — in the same shape a build failure's diagnostics take, each against the
identity of the module it belongs to. A program with none SHALL report an empty list.

#### Scenario: A warning from a build that succeeds is reported
- **WHEN** a caller builds an artifact from the source `/// See [Missing].` followed by
  `let root() = { 42 }`
- **THEN** the build SHALL succeed and the artifact SHALL evaluate to `42`
- **AND** the artifact's diagnostics SHALL be one `unresolved-doc-link` warning labeled at line 1

#### Scenario: A clean build reports no diagnostics
- **WHEN** a caller builds an artifact from `let root() = { 42 }`
- **THEN** the artifact's diagnostics SHALL be empty
