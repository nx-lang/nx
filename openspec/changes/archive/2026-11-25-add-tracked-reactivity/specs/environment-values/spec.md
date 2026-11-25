## MODIFIED Requirements

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
set is empty, since it gives the same value at every read. A runtime MAY also, in place of computing
a default, an entry-module provide or an override's formula again, reuse a result it computed
earlier, as the reactivity capability defines; the read SHALL then give the value computing it
there would give.

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
