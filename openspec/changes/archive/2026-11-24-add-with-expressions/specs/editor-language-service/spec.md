## MODIFIED Requirements

### Requirement: Language service completes and describes environment values
The language service SHALL offer, after `env.` in an expression and in an element's attribute
position, the environment values the module can see, each with its type and its documentation, and
after `env.name.` the fields of the value's record type. Hover on an `env.` read or override SHALL
show the declaration with its type and default, and its documentation. Hover on a function, value or
component SHALL list the environment values its body reads directly or through the functions and
values it calls. Hover on a top-level `provide` or on a `provide` block's `env with` SHALL list the
environment values and field paths it sets, and for a bundle SHALL list what the bundle sets,
following `with` steps, so the line can be read without opening the bundle. Go to definition is not
part of this requirement: the language service has no definition query yet, and environment values
get one when it does.

#### Scenario: Completion after `env.`
- **WHEN** a module that imports `design.nx`, which exports `environment space: int = 4`, has the
  cursor after `env.` inside an element's attributes
- **THEN** the completions SHALL include `space` with the type `int`

#### Scenario: Hover lists what a component reads
- **WHEN** `component <Badge /> = { <Text value={label()} /> }`, `let label() = { env.locale }`,
  and the cursor is on `Badge`
- **THEN** the hover SHALL say that `Badge` reads `env.locale`

#### Scenario: Hover on a bundle provide lists what it sets
- **WHEN** `brand.nx` exports `let theme: env.Update = <env.Update radius=12
  color=<SemanticColors.Update accent=#e11d48 /> />`, the main module contains `provide env with
  brand.theme`, and the cursor is on that line
- **THEN** the hover SHALL say that it sets `env.radius` and `env.color.accent`

## ADDED Requirements

### Requirement: Language service completes and explains `with`
The language service SHALL offer, after `with {` and between entries of a field list, the fields of
the left side's type with their types and documentation, and after `name.` in an entry the fields of
that field's record type; for an `env.Update` left side, and after `provide env with … with {`, it
SHALL offer the environment values the module can see. Hover on `with` SHALL show the type being
patched.

#### Scenario: Field completion in a field list
- **WHEN** the cursor is after `{ <User name="Ada" /> with { ` in a module declaring
  `type User = { name: string email?: string }`
- **THEN** completion SHALL offer `name` and `email` with their types

#### Scenario: Hover on `with`
- **WHEN** the cursor is on `with` in `{ <User name="Ada" /> with { name="Bo" } }`
- **THEN** the hover SHALL show `User`
