## ADDED Requirements

### Requirement: Language service completes and describes environment values
The language service SHALL offer, after `env.` in an expression and in an element's attribute
position, the environment values the module can see, each with its type and its documentation,
and after `env.name.` the fields of the value's record type. Hover on an `env.` read or override
SHALL show the declaration with its type and default, and its documentation. Hover on a function,
value or component SHALL list the environment values its body reads directly or through the
functions and values it calls. Go to definition on an `env.` read or override SHALL reach the
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
