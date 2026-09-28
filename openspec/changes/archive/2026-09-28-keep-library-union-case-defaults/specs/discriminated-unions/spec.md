## ADDED Requirements

### Requirement: A union case field default applies across a library boundary
A union case field declared with a default SHALL be omissible at every construction of the case,
including a construction in a module that imports the union from a library, and the constructed value
SHALL carry the default. A field with neither a default nor the `?` mark SHALL remain required there.

#### Scenario: A consumer omits a defaulted field of a library union case
- **WHEN** a library declares `export type ScaleConfig = | numeric { min:int = 0 max:int = 10 step:int = 1 } | labeled { options:string+ }`
- **AND** a consumer that imports it evaluates `<ScaleConfig.numeric max=5 />`
- **THEN** analysis SHALL report no diagnostic
- **AND** the value SHALL have `min` 0, `max` 5 and `step` 1

#### Scenario: A required library union case field stays required
- **WHEN** the same consumer writes `<ScaleConfig.labeled />`
- **THEN** analysis SHALL report `missing-union-case-field` for `options`
