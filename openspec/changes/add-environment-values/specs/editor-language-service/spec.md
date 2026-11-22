## ADDED Requirements

### Requirement: Language service completes and describes environment values
The language service SHALL offer, after `env.` in an expression and in an element's attribute
position, the environment values the module can see, each with its type and its documentation,
and after `env.name.` the fields of the value's record type. Hover on an `env.` read or override
SHALL show the declaration with its type and default, and its documentation. Hover on a function,
value or component SHALL list the environment values its body reads directly or through the
functions and values it calls. Hover on a top-level `provide` SHALL list the environment values and
field paths it sets, and for a bundle SHALL list what the bundle sets, so the line can be read
without opening the bundle. Go to definition on an `env.` read or override SHALL reach the
declaration.

#### Scenario: Completion after `env.`
- **WHEN** a module that imports `design.nx`, which exports `environment space: int = 4`, has the
  cursor after `env.` inside an element's attributes
- **THEN** the completions SHALL include `space` with the type `int`

#### Scenario: Hover lists what a component reads
- **WHEN** `component <Badge /> = { <Text value={label()} /> }`, `let label() = { env.locale }`,
  and the cursor is on `Badge`
- **THEN** the hover SHALL say that `Badge` reads `env.locale`

#### Scenario: Go to definition from a read
- **WHEN** the cursor is on `colorMode` in `env.colorMode` and a definition is requested
- **THEN** the result SHALL be the `environment colorMode` declaration

#### Scenario: Hover on a bundle provide lists what it sets
- **WHEN** `brand.nx` exports `let theme: env.Update = <env.Update radius=12 color=<SemanticColors.Update accent=#e11d48 /> />`,
  the main module contains `provide brand.theme`, and the cursor is on that line
- **THEN** the hover SHALL say that it sets `env.radius` and `env.color.accent`
