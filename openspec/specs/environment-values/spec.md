# environment-values Specification

## Purpose
Define environment values: values with a type and a default that any body reads with `env.name`,
overridden for part of the output with `env.` attributes and `provide` blocks, set app-wide by the
main module's top-level provides, and given root values by the host.
## Requirements
### Requirement: An environment value is declared with a type and a default
A module SHALL be able to declare an environment value at its top level with
`environment name: Type = default`, optionally preceded by `private` or `export`. `environment` SHALL
be a keyword only as the first token of a module member after an optional visibility modifier, so
a record field, property or local named `environment` keeps its meaning. The type and the default
SHALL both be required: a declaration without a default SHALL be rejected with
`environment-default-required`, and one without a type SHALL be a syntax error. The default SHALL
be checked against the type as a `let` value's initializer is. Visibility SHALL work as it does for
a `let` value: a declaration without a modifier is internal to its library. Two environment
values of one module with the same name SHALL be rejected with `duplicate-environment`, and a
value named `Update` SHALL be rejected with `reserved-name`, since `env.Update` names the bundle
type.

#### Scenario: A declaration with a type and a default
- **WHEN** a module contains `export type ColorMode = light | dark` and
  `export environment colorMode: ColorMode = light`
- **THEN** the module SHALL compile, and `colorMode` SHALL be an exported environment value of type
  `ColorMode`

#### Scenario: A declaration without a default is rejected
- **WHEN** a module contains `environment locale: string`
- **THEN** the module SHALL fail with `environment-default-required` at the declaration

#### Scenario: A default of the wrong type is rejected
- **WHEN** a module contains `environment scale: float = "large"`
- **THEN** the module SHALL fail with the type mismatch a `let` initializer of the wrong type gets

#### Scenario: `environment` stays a name elsewhere
- **WHEN** a module contains `type Deployment = { environment:string }`
- **THEN** the module SHALL compile, and `environment` SHALL be a field of `Deployment`

### Requirement: `env.` reads an environment value
An expression `env.name` SHALL read the environment value `name`, and a member access after it,
such as `env.color.accent`, SHALL read a field of the value as on any record. The read's type
SHALL be the value's declared type. `name` SHALL resolve among the environment values the module
can see: those it declares, the internal ones of its library, and the exported ones of every
module it imports, under any alias. A name that resolves to none SHALL be rejected with
`unknown-environment-value`, and a name that resolves to more than one SHALL be rejected with
`ambiguous-environment-value`, naming the modules that declare it. `env` SHALL be reserved: a
declaration, parameter, `for` item, pattern binding or property named `env` SHALL be rejected with
`reserved-name`, and `env` without a member SHALL be rejected as an expression. A record field
named `env`, read as `x.env`, SHALL keep working.

#### Scenario: A read gets the default
- **WHEN** a module declares `environment locale: string = "en-US"` and
  `let greeting() = { env.locale }`, and a host calls `greeting` with no environment values
- **THEN** the result SHALL be `"en-US"`

#### Scenario: A read of an imported value
- **WHEN** module `design.nx` declares `export environment space: int = 4`, and a module that
  contains `import "./design.nx" as design` reads `env.space`
- **THEN** the read SHALL resolve to `design.nx`'s `space`, without the alias

#### Scenario: An unknown and an ambiguous read
- **WHEN** a module reads `env.missing`, and another module imports two modules that both export an
  environment value `colorMode` and reads `env.colorMode`
- **THEN** the first SHALL fail with `unknown-environment-value`
- **AND** the second SHALL fail with `ambiguous-environment-value` naming both modules

#### Scenario: `env` can't be bound
- **WHEN** a module contains `let describe(env: string) = { env }`
- **THEN** the module SHALL fail with `reserved-name` at the parameter

### Requirement: An `env.` attribute overrides a value for an element and everything in it
An element SHALL accept attributes of the form `env.name=value` and `env.name.field=value`, where
`name` resolves as an `env.` read does, on a component, an external component, and an intrinsic or
host element alike. Such an attribute SHALL NOT be a property: it SHALL NOT be checked against
the element's props, SHALL NOT satisfy a required prop, and SHALL NOT appear in the element's
fields. The value SHALL be checked against the value's type, or against the type of the field the
path names. Each step of a path SHALL name a field of a record type, or the attribute SHALL be
rejected with `environment-path`. The same path written twice on one element SHALL be rejected as
a duplicate property is. An element with `env.` attributes SHALL mean what a `provide` block with
the same overrides, in the same order, around the element without them means.

#### Scenario: An override on a component use
- **WHEN** `Footer` renders `<Section env.colorMode=dark><Links /></Section>`, and `Links`'s body
  reads `env.colorMode`
- **THEN** `Links` SHALL read `dark`

#### Scenario: An override on an intrinsic element
- **WHEN** a body renders `<div env.locale="fr-FR"><DateLabel /></div>`, and `DateLabel`'s body
  reads `env.locale`
- **THEN** `DateLabel` SHALL read `"fr-FR"`
- **AND** the `div` in the rendered output SHALL hold no field for the override

#### Scenario: A path that is not a record field is rejected
- **WHEN** a module declares `environment locale: string = "en-US"` and an element carries
  `env.locale.region="FR"`
- **THEN** the module SHALL fail with `environment-path`

#### Scenario: An override does not satisfy a prop
- **WHEN** `component <Card title:string /> = { <Text value={title} /> }` and a body renders
  `<Card env.colorMode=dark />`
- **THEN** the module SHALL fail with the error for the missing `title`
- **AND** the descriptor of a `Card` use that also sets `title` SHALL hold `title` and no field
  named `env` or `colorMode`

### Requirement: A `provide` block overrides values for an expression
The language SHALL accept `provide overrides { body }` wherever an expression or markup content is,
where `overrides` is either one or more overrides in the attribute form (`env.name=value` or
`env.name.field=value`), or one bundle, `env with bundle`, but not both. `bundle` SHALL be an
operand of the kind a `with` takes on its right (a name, member access, call, element or braced
expression) or a field list, which is an inline bundle (`env with { colorMode=dark }` means `env
with <env.Update colorMode=dark />`), optionally followed by more `with` steps (`env with brand with
{ radius=4 }`); after `with`, a brace whose first entry is `name=` SHALL be a field list, and the
body SHALL follow the last override or `with` step. A block that mixes the two forms SHALL be a
syntax error whose message suggests one `env with` chain that keeps the written order:
attribute-form overrides written after the bundle become a `with { … }` step after it, and ones
written before it become a field list before it (`provide env.radius=0 env with marketing` suggests
`provide env with { radius=0 } with marketing`). Its value and type SHALL be its body's.
Attribute-form overrides SHALL be checked as an element's `env.` attributes are, `bundle` SHALL be
of type `env.Update`, and a block without an override SHALL be a syntax error. `env with` anywhere
else SHALL be rejected with `reserved-name`, whose message says that `env with` is written only
after `provide`. Since `provide` at the start of an expression always begins a block, a declaration,
parameter or binding named `provide` SHALL be rejected with `reserved-name`; a record field named
`provide`, read as `x.provide`, SHALL keep working.

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

#### Scenario: An inline bundle means the same as a named one
- **WHEN** a body renders `{ provide env with <env.Update radius=12 /> { <Banner /> } }`
- **THEN** `Banner` SHALL read `radius` `12`, as it does with the bundle named `marketing` above

#### Scenario: A bundle with a `with` chain
- **WHEN** a body renders
  `{ provide env with marketing with { radius=4 colorMode=dark } { <Banner /> } }`
- **THEN** `Banner` SHALL read `radius` `4` and `colorMode` `dark`, and `marketing`'s other values

#### Scenario: Mixing a bundle and attribute-form overrides is rejected
- **WHEN** a body renders `{ provide env with marketing env.colorMode=dark { <Banner /> } }`
- **THEN** the module SHALL fail with a syntax error whose message suggests
  `provide env with marketing with { colorMode=dark }`

#### Scenario: The suggestion keeps the order
- **WHEN** a body renders `{ provide env.radius=0 env with marketing { <Banner /> } }`
- **THEN** the syntax error's message SHALL suggest `provide env with { radius=0 } with marketing`

#### Scenario: A field list after `provide env with`
- **WHEN** a body renders `{ provide env with { colorMode=dark } { <Banner /> } }`
- **THEN** `Banner` SHALL read `colorMode` `dark`, as with `provide env.colorMode=dark`

#### Scenario: `env with` outside a provide is rejected
- **WHEN** a module contains `let saved = { env with marketing }`
- **THEN** the module SHALL fail with `reserved-name` at `env`, whose message says `env with` is
  written only after `provide`

### Requirement: An override with a field path updates one field and keeps the rest
An override whose path names a field SHALL produce the value that the same place would see without
the override, with that field replaced, as applying an update record replaces it. An override
without a path SHALL replace the whole value. Overrides of one value at one element, block or
module SHALL apply in the order written, each on the result of the one before.

#### Scenario: A field override keeps the other fields
- **WHEN** `env.color` is `<SemanticColors accent=#2563eb surface=#ffffff />` at a place, and a
  body there renders `<Panel env.color.accent=#f472b6 />`, whose body reads `env.color`
- **THEN** `Panel` SHALL read `accent` equal to `#f472b6` and `surface` equal to `#ffffff`

#### Scenario: A later override applies on an earlier one
- **WHEN** an element carries `env.color=partnerColors env.color.accent=#000000`
- **THEN** the value inside SHALL be `partnerColors` with `accent` equal to `#000000`

### Requirement: Only the main module provides values app-wide
A module SHALL be able to contain, at its top level, `provide env.name = value`, `provide
env.name.field = value`, and `provide env with bundle`, where `bundle` is the same operand as in a
`provide` block (a name, member access, call, element or braced expression), optionally followed by
more `with` steps, of type `env.Update`. The main module of a program SHALL be its entry module,
which the host chooses: `chatLink.nx` for a chat link, the only module of a one-module program, and
`app.nx` by convention for an app. A program SHALL apply the top-level provides of its main module
alone, in the order written, and a module that another module of the program imports SHALL NOT
contain a top-level provide: each one SHALL be rejected with `provide-outside-main-module`, whose
message says that app-wide provides belong in the main module. A library module that exports a
declaration SHALL count as imported, since a program that imports the library reaches it. A module
that nothing imports, such as a library's own `app.nx` for previews, SHALL be allowed to contain
them, and they SHALL take effect only when it is the entry module. A top-level `provide bundle`
without `env with` SHALL be a syntax error whose message suggests `provide env with bundle`.

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
- **THEN** the module SHALL fail with a syntax error at the provide that suggests
  `provide env with brand.theme`

### Requirement: `env.Update` is the type of a bundle of overrides
`env.Update` SHALL be an update record type whose fields are the environment values visible where
it is written, each optional, with a record-typed value taking that record's update type, so that
a bundle can update fields of it. `<env.Update …/>` SHALL construct one, and a field SHALL name an
environment value as an `env.` read does. A bundle SHALL identify each value by its declaration,
not by its name, so that a main module that applies a bundle built in another module applies it to
the values that module meant, whether or not the main module can see them. Two `env.Update` values
built in different modules SHALL both be accepted by `provide`. A bundle SHALL hold its overrides as
an environment override holds its value: a value that reads no environment value is computed where
the bundle is built, and any other is kept as a formula with the locals it captured, computed where
a value is read. So a bundle SHALL mean the same wherever it is applied, whether it is named, written
inline, or passed through a prop, a local or a `for` item.

#### Scenario: A bundle field updates part of a record
- **WHEN** a bundle is `<env.Update color=<SemanticColors.Update accent=#f472b6 /> />` and the main
  module provides it
- **THEN** `env.color` with no override around it SHALL be the default with `accent` replaced

#### Scenario: A bundle passed as a prop still follows the mode
- **WHEN** `component <Banner theme: env.Update /> = { provide env with theme { <Hero /> } }`, a body
  renders `<Banner theme=marketing env.colorMode=dark />`, `marketing` is
  `<env.Update color={ if env.colorMode is { light => a dark => b } } />`, and `Hero` reads
  `env.color`
- **THEN** `Hero` SHALL read `b`

### Requirement: Values take precedence from the default up to the nearest override
A read SHALL find the value of an environment value at its place from the first source that sets it,
in this order: the nearest local override around the place (an `env.` attribute, an attribute-form
override of a `provide` block, or a bundle in one, nearest first); then the host's root value; then
the main module's provides, a later one before an earlier one; then the declaration's default. A
source that sets only a field path, and a bundle that doesn't set the value at all, SHALL
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

#### Scenario: A `with` step after a bundle wins
- **WHEN** `marketing` sets `radius=12` and a body renders
  `{ provide env with marketing with { radius=0 } { <Banner /> } }`, where `Banner` reads
  `env.radius`
- **THEN** `Banner` SHALL read `0`

#### Scenario: A section bundle patches the app theme
- **WHEN** a module declares `environment accent: string = "blue"` and
  `environment radius: int = 2`, the main module contains `provide env with brand`, where `brand` is
  `<env.Update accent="green" radius=8 />`, `marketing` is `<env.Update accent="pink" />`, and a body
  renders `{ provide env with marketing { <Banner /> } }`, where `Banner` reads `env.accent` and
  `env.radius`
- **THEN** `Banner` SHALL read `"pink"` and `8`

### Requirement: A component's body sees the environment where its use is placed
A component's body SHALL run in the environment of the place where its use ends up in the
rendered output: the overrides around the use in the body that renders it, inside those around
that body's own use, and so on up to the root. Overrides written around a use where it is built
SHALL apply as well, and they SHALL be nearer than those around the place where another body puts
it. An expression SHALL see the overrides written around it in its body. A function, a `let` value
and a default SHALL have no environment of their own: each SHALL read in the environment of the
call or read that evaluates it.

#### Scenario: Content a caller builds sees the override where it is placed
- **WHEN** `Page` renders `<Card><Label /></Card>`, `Card` renders its content inside
  `<div env.colorMode=dark>`, and `Label`'s body reads `env.colorMode`, whose default is `light`
- **THEN** `Label` SHALL read `dark`

#### Scenario: An override written on the use wins over one where it is placed
- **WHEN** `Page` renders `<Card><Label env.colorMode=light /></Card>` and `Card` renders its
  content inside `<div env.colorMode=dark>`
- **THEN** `Label` SHALL read `light`

#### Scenario: A function reads where it is called
- **WHEN** a module declares `let label() = { env.locale }` and a body renders
  `<Text value={provide env.locale="fr-FR" { label() }} />`
- **THEN** the `Text`'s `value` SHALL be `"fr-FR"`

### Requirement: A default that depends on itself is rejected
A declaration's default whose evaluation can read the value itself SHALL be rejected with
`environment-cycle` at the declaration, naming the values on the path. The check SHALL follow
`env.` reads, the functions and values those call, and the defaults of the values they read, and
SHALL NOT follow component uses. An override, a bundle entry or a main-module provide of a value
`V` whose value can read a value `W` whose default can read `V`, as the same check follows them,
SHALL be rejected with `environment-cycle` at the override, naming the path: under it, a read of `W`
would evaluate `W`'s default, then the override, then `W` again with no source left. An override
placed by another body can still close such a path at run time, so a read that reaches a value while
its default is being evaluated, with no source after the default, SHALL fail with the runtime error
`nx-ir-environment-cycle` naming the values on the path. While an override is evaluated for a value,
a read of that value sees only the sources after it, so an override of a value by itself cannot loop.

#### Scenario: Two defaults that read each other
- **WHEN** `environment a: int = env.b + 1` and `environment b: int = env.a + 1`
- **THEN** both SHALL fail with `environment-cycle` naming `a` and `b`

#### Scenario: A cycle through a function
- **WHEN** `environment a: int = twice()` and `let twice() = { env.a * 2 }`
- **THEN** `a` SHALL fail with `environment-cycle` naming `twice`

#### Scenario: An override that reads a value whose default reads it
- **WHEN** `color`'s default reads `env.colorMode` and a body renders
  `<A env.colorMode={ if env.color == x { dark } else { light } } />`
- **THEN** the module SHALL fail with `environment-cycle` at the override, naming `colorMode` and
  `color`

#### Scenario: An override of a value by itself is fine
- **WHEN** a body renders `<Stack env.space={env.space * 2} />`
- **THEN** the module SHALL be accepted

### Requirement: The host sets root values by name and they are checked
A host SHALL be able to give an evaluation root values for environment values, naming each one by
its simple name when exactly one environment value in the program has that name, and by its
module identity and name otherwise. A root value SHALL be checked against the declared type at the
boundary, as an entrypoint argument is. A name that matches no environment value, or a simple name
that matches more than one, SHALL be rejected with `nx-ir-environment-name`, and the evaluation
SHALL NOT run. NX code SHALL have no way to read the host's operating-system environment.

#### Scenario: An unknown root value is rejected
- **WHEN** a host evaluates a program with the root value `colourMode` and the program declares no
  environment value of that name
- **THEN** the evaluation SHALL fail with `nx-ir-environment-name` naming `colourMode`

#### Scenario: A root value of the wrong type is rejected
- **WHEN** a host sets `colorMode`, of type `light | dark`, to `"dim"`
- **THEN** the evaluation SHALL fail with the boundary diagnostic for a value of the wrong type

### Requirement: Every environment value is computed where it is read
An environment value SHALL be computed at each read, in the environment of that read, from every
source that sets it: the declaration's default, the main module's provides, and every local
override, whether an `env.` attribute, an attribute-form override of a `provide` block, or a bundle.
Each override's value SHALL therefore act as a formula: a change, nearer the reader, of a value it
reads SHALL change what it gives. A read of a value `V` SHALL evaluate only the entries that set `V`
of each bundle on its way, not the bundle's other entries. While any source of `V` is evaluated, at
any depth of the reads and calls it makes, a read of `V` SHALL see only the sources after that
source, so an override can read the value it replaces; reads of other values SHALL see every source,
including other entries of the same bundle. An override's value SHALL see the names in scope where
it is written, with the values they had there. The element's other attributes and content, and a
`provide` block's body, SHALL be evaluated inside the overrides. Whether a value reads the
environment SHALL be decided from its static read set, so a value that reads one only on some
branches is a formula. A runtime MAY compute once, where it is written, a value whose static read
set is empty, since it gives the same value at every read.

#### Scenario: An override cascades through a default
- **WHEN** `color`'s default is `if env.colorMode is { light => lightColors dark => darkColors }`
  and a body renders `<Footer env.colorMode=dark />`, whose body reads `env.color`
- **THEN** `Footer` SHALL read `darkColors`

#### Scenario: An override's value reads the value outside it
- **WHEN** `env.space` is `4` at a place, and a body there renders `<Stack env.space={env.space * 2}
  />`, whose body reads `env.space`
- **THEN** `Stack` SHALL read `8`

#### Scenario: An override follows a value overridden nearer the reader
- **WHEN** `env.space` is `4` at a place, and a body there renders
  `<Section env.gap={env.space * 2}><Inner env.space=1 /></Section>`, where `Inner`'s body reads
  `env.gap`
- **THEN** `Inner` SHALL read `2`
- **AND** a read of `env.gap` in `Section`'s own body SHALL be `8`

#### Scenario: An override sees the locals where it is written
- **WHEN** `env.space` is `4`, items have `scale` `1` and `3`, and a body renders
  `for item in items { <Row env.gap={env.space * item.scale}><Cell env.space=2 /></Row> }`, where
  `Cell` reads `env.gap`
- **THEN** the two `Cell`s SHALL read `2` and `6`

#### Scenario: One bundle entry reads another value the bundle sets
- **WHEN** `dense` is `<env.Update space=2 gap={env.space * 3} />` and a body renders
  `{ provide env with dense { <List /> } }`, where `List` reads `env.gap`
- **THEN** `List` SHALL read `6`

#### Scenario: A main-module provide reads the host's value
- **WHEN** the main module provides `env.color = if env.colorMode is { light => a dark => b }` and
  the host sets `colorMode` to `dark`
- **THEN** a read of `env.color` with no override around it SHALL be `b`

#### Scenario: A section theme follows a mode set inside the section
- **WHEN** `marketing` is `<env.Update color={ if env.colorMode is { light => a dark => b } } />`,
  the host sets no `colorMode`, whose default is `light`, and a body renders
  `{ provide env with marketing { <Section env.colorMode=dark><Banner /></Section> } }`, where
  `Banner` reads `env.color`
- **THEN** `Banner` SHALL read `b`
- **AND** with `provide env with marketing with { colorMode=dark }` in place of the inner
  attribute, `Banner` SHALL read `b` as well

