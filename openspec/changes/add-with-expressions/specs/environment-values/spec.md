## MODIFIED Requirements

### Requirement: A `provide` block overrides values for an expression
The language SHALL accept `provide overrides { body }` wherever an expression or markup content
is, where `overrides` is one or more overrides, each either in the attribute form (`env.name=value`
or `env.name.field=value`) or a section bundle, `env with bundle`. Its value and type SHALL be its
body's. Attribute-form overrides SHALL be checked as an element's `env.` attributes are, and a
block without an override SHALL be a syntax error. `bundle` SHALL be a name or member access that
refers to a top-level value of type `env.Update`; any other expression SHALL be rejected with
`section-bundle-name`, whose message suggests naming the bundle as a top-level value or writing
single overrides. `env with` SHALL be allowed only as an override of a `provide`. Since `provide` at
the start of an expression always begins a block, a declaration, parameter or binding named
`provide` SHALL be rejected with `reserved-name`; a record field named `provide`, read as
`x.provide`, SHALL keep working.

#### Scenario: A provide in an expression
- **WHEN** a module declares `environment locale: string = "en-US"` and
  `let label() = { provide env.locale="fr-FR" { env.locale } }`
- **THEN** `label()` SHALL be `"fr-FR"`

#### Scenario: A provide around several siblings
- **WHEN** a body renders `{ provide env.colorMode=dark { <Header /> <Hero /> } <Body /> }`, and all
  three bodies read `env.colorMode`, whose default is `light`
- **THEN** `Header` and `Hero` SHALL read `dark` and `Body` SHALL read `light`

#### Scenario: A bundle for one section
- **WHEN** a module declares `environment space: int = 4` and `environment radius: int = 2`,
  `let marketing: env.Update = <env.Update radius=12 />`, the main module provides
  `env.space = 8`, and a body renders `{ provide env with marketing { <Banner /> } <Body /> }`,
  where `Banner` and `Body` read `env.space` and `env.radius`
- **THEN** `Banner` SHALL read `space` `8` and `radius` `12`
- **AND** `Body` SHALL read `space` `8` and `radius` `2`

#### Scenario: An inline section bundle is rejected
- **WHEN** a body contains `provide env with <env.Update radius=12 /> { <Banner /> }`
- **THEN** the module SHALL fail with `section-bundle-name`

#### Scenario: `env with` outside a provide is rejected
- **WHEN** a module contains `let saved = { env with marketing }`
- **THEN** the module SHALL fail with a syntax error at `env`

### Requirement: Only the main module provides values app-wide
A module SHALL be able to contain, at its top level, `provide env.name = value`,
`provide env.name.field = value`, and `provide env with bundle`, where `bundle` is an expression of
type `env.Update`. The main module of a program SHALL be its entry module, which the host chooses:
`chatLink.nx` for a chat link, the only module of a one-module program, and `app.nx` by convention
for an app. A program SHALL apply the top-level provides of its main module alone, in the order
written, and a module that another module of the program imports SHALL NOT contain a top-level
provide: each one SHALL be rejected with `provide-outside-main-module`, whose message says that
app-wide provides belong in the main module. A library module that exports a declaration SHALL
count as imported, since a program that imports the library reaches it. A module that nothing
imports, such as a library's own `app.nx` for previews, SHALL be allowed to contain them, and they
SHALL take effect only when it is the entry module. A top-level `provide bundle` without `env with`
SHALL be a syntax error.

#### Scenario: The main module sets a value app-wide
- **WHEN** `app.nx` imports `design.nx`, which declares `export environment space: int = 4`,
  contains `provide env.space = 8`, and is the entry module, and any body in the program reads
  `env.space` with no override around it
- **THEN** the read SHALL be `8`

#### Scenario: A provide in an imported module is rejected
- **WHEN** `theme.nx` contains `provide env.space = 8` and `app.nx` imports `theme.nx`
- **THEN** the program SHALL fail with `provide-outside-main-module` at the provide in `theme.nx`

#### Scenario: A bundle applies several values
- **WHEN** `brand.nx` exports `let theme: env.Update = <env.Update space=8 radius=12 />` and the
  main module contains `provide env with brand.theme`
- **THEN** reads of `env.space` and `env.radius` with no override around them SHALL be `8` and `12`

#### Scenario: The old bundle form is a syntax error
- **WHEN** the main module contains `provide brand.theme`
- **THEN** the module SHALL fail with a syntax error at the provide

### Requirement: Values take precedence from the default up to the nearest override
A read SHALL find the value of an environment value at its place from the first source that sets it,
in this order: the nearest local override around the place (an `env.` attribute, an attribute-form
override of a `provide` block, or a section bundle, nearest first); then the host's root value; then
the main module's provides, a later one before an earlier one; then the declaration's default. A
source that sets only a field path, and a section bundle that doesn't set the value at all, SHALL
take its base from the sources after it. The overrides of one block SHALL apply in the order
written, each on the result of the one before.

#### Scenario: A local override beats the host's root value
- **WHEN** the host sets `colorMode` to `dark`, the main module provides `env.colorMode = light`,
  and a body renders `<Card env.colorMode=light />`, whose body reads `env.colorMode`
- **THEN** `Card` SHALL read `light`
- **AND** a body with no override around it SHALL read `dark`

#### Scenario: The main module's provide beats the default
- **WHEN** a value's default is `4`, the main module provides `env.space = 8`, and the host sets no
  root value for it
- **THEN** a read with no override around it SHALL be `8`

#### Scenario: A single override after a bundle in one block wins
- **WHEN** `marketing` sets `radius=12` and a body renders
  `{ provide env with marketing env.radius=0 { <Banner /> } }`, where `Banner` reads `env.radius`
- **THEN** `Banner` SHALL read `0`

#### Scenario: A section bundle patches the app theme
- **WHEN** a module declares `environment accent: string = "blue"` and
  `environment radius: int = 2`, the main module contains `provide env with brand`, where `brand` is
  `<env.Update accent="green" radius=8 />`, `marketing` is `<env.Update accent="pink" />`, and a body
  renders `{ provide env with marketing { <Banner /> } }`, where `Banner` reads `env.accent` and
  `env.radius`
- **THEN** `Banner` SHALL read `"pink"` and `8`

### Requirement: Defaults and main-module provides are evaluated where they are read
These SHALL be evaluated each time a read uses them, in the environment of that read: a
declaration's default, the value of a main-module provide, and each override of a section bundle, so
that an override of a value one of them reads changes what it gives. Inside the evaluation of a main-module provide or a section
bundle's override of a value, a read of that same value SHALL see the sources after it. The value of
an `env.` attribute or of a `provide` block's attribute-form override SHALL be evaluated once, where
it is written, in the environment outside the overrides of that element or block. The element's
other attributes and content, and the block's body, SHALL be evaluated inside them. A component
descriptor's `$env` SHALL carry a section bundle as a reference to the top-level value that names
it, which a runtime evaluates at a read.

#### Scenario: An override cascades through a default
- **WHEN** `color`'s default is `if env.colorMode is { light => lightColors dark => darkColors }`
  and a body renders `<Footer env.colorMode=dark />`, whose body reads `env.color`
- **THEN** `Footer` SHALL read `darkColors`

#### Scenario: An override's value reads the value outside it
- **WHEN** `env.space` is `4` at a place, and a body there renders `<Stack env.space={env.space * 2} />`,
  whose body reads `env.space`
- **THEN** `Stack` SHALL read `8`

#### Scenario: A main-module provide reads the host's value
- **WHEN** the main module provides `env.color = if env.colorMode is { light => a dark => b }` and
  the host sets `colorMode` to `dark`
- **THEN** a read of `env.color` with no override around it SHALL be `b`

#### Scenario: A section theme follows a mode set inside the section
- **WHEN** `marketing` is `<env.Update color={ if env.colorMode is { light => a dark => b } } />`,
  the host sets no `colorMode`, whose default is `light`, and a body renders
  `{ provide env with marketing env.colorMode=dark { <Banner /> } }`, where `Banner` reads
  `env.color`
- **THEN** `Banner` SHALL read `b`

#### Scenario: A descriptor carries a section bundle by reference
- **WHEN** a body renders `{ provide env with marketing { <Banner /> } }` and the result is
  rendered output
- **THEN** the `Banner` descriptor's `$env` SHALL hold a reference to `marketing`'s declaration and
  no evaluated `color` value
