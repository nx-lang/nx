## MODIFIED Requirements

### Requirement: NX IR encodes environment values behind a required feature
NX IR SHALL encode an environment value as a declaration of kind `environment` holding its type and
its default node, and exported as other declarations are. A read SHALL be an `envRead` node, kind
`30`, holding a reference to the declaration, with any field read after it encoded as ordinary
`member` nodes. A `provide` block, and an element with `env.` attributes, SHALL be an `envProvide`
node, kind `31`, holding a list of overrides and a body; the list SHALL hold either attribute
overrides or a single bundle override, never both. Each override SHALL hold a reference to the
declaration, a list of field names that may be empty, a value node, and, only when the value is a
formula, a fourth cell holding the formula flag; an override that is not a formula SHALL have no
fourth cell, so its encoding is unchanged. A value is a formula when its static read set, following
`env.` reads, the functions and values they call and the defaults of the values they read, is not
empty. A bundle override SHALL hold no reference and an empty list, and its node SHALL be of type
`env.Update`. An `env.Update` bundle SHALL be an `envUpdate` node, kind `32`, holding an optional
`base` node of type `env.Update` and a list of overrides of the same shape as `envProvide`'s; its
overrides SHALL be the base's followed by its own. A module with top-level provides SHALL hold one
declaration of kind `provides`, named `$provides`, listing them in source order, each holding an
optional reference to the declaration, a list of field names and a value node: an override names its
declaration, and a bundle names none and its node is of `env.Update` type. A module that declares an
environment value, contains an `envRead`, `envProvide` or `envUpdate` node or has top-level provides
SHALL list the required feature `environment-v1`, and a module that does none of these SHALL NOT. A
module that contains a formula override, a bundle override in an `envProvide`, or an `envUpdate`
with a `base` SHALL list `environment-v2` as well, and a module that does none of these SHALL NOT.
The `explain` text SHALL read the nodes as `env.` reads, `provide` blocks and `with` steps. Adding
the declaration kind, the node kinds and the features SHALL NOT change the IR schema version.

#### Scenario: A read and a provide are their own nodes
- **WHEN** NX source contains `environment locale: string = "en-US"` and
  `let label() = { provide env.locale="fr-FR" { env.locale } }`, and NX IR is emitted
- **THEN** the module SHALL declare `locale` as an `environment` declaration with the default
  `"en-US"`
- **AND** the body of `label` SHALL be an `envProvide` node with one override of `locale` with an
  empty path, whose body is an `envRead` of `locale`
- **AND** the module SHALL list `environment-v1`

#### Scenario: An element's `env.` attributes are a provide around it
- **WHEN** NX source renders `<Section env.colorMode=dark title="x" />`
- **THEN** the IR SHALL be an `envProvide` node overriding `colorMode` whose body builds `Section`
  with `title` alone

#### Scenario: A program without environment values needs no feature
- **WHEN** NX source declares no environment value and contains no `env.` read or `provide`
- **THEN** the emitted module SHALL NOT list `environment-v1` and SHALL be the IR emitted before
  this change

#### Scenario: A formula override is flagged
- **WHEN** NX source renders `<Stack env.gap={env.space * 2} env.colorMode=dark />`
- **THEN** the `envProvide` override of `gap` SHALL have the formula flag set and the override of
  `colorMode` SHALL NOT
- **AND** the module SHALL list `environment-v1` and `environment-v2`

#### Scenario: A bundle in a block and a `with` step
- **WHEN** NX source contains `let dense: env.Update = { brand with { space=2 } }` and renders
  `{ provide env with dense { <List /> } }`
- **THEN** `dense` SHALL be an `envUpdate` node whose `base` reads `brand`
- **AND** the `envProvide` SHALL hold one bundle override whose node reads `dense`

#### Scenario: A module that uses only plain overrides stays at v1
- **WHEN** NX source uses only `env.` attributes and `provide` overrides whose values read no
  environment value, and no bundle in a block
- **THEN** the module SHALL list `environment-v1` and not `environment-v2`, and its IR SHALL be
  what was emitted before this change

### Requirement: A component descriptor carries the overrides around it in `$env`
A runtime that evaluates an `envProvide` node SHALL evaluate the override values without the formula
flag once, in order, in the environment outside the node, and SHALL capture each formula override;
it SHALL then evaluate the body with the overrides in effect, and add the overrides to every
component descriptor its result reaches. Capturing a formula SHALL record the module, the override's
value node, the module's fingerprint, and the values of the locals the node reads, in canonical
form. A bundle override's node SHALL be evaluated where it is written, to an `env.Update` value
whose entries are held as `$env` entries are. The walk SHALL go through sequences, records, and the
fields and content of elements that are not component descriptors, and SHALL stop at each component
descriptor, which SHALL receive the overrides in front of any it already holds. A component
descriptor here SHALL be a record whose `$type` names a component declared with a body; the record
of an external component is the host's to interpret, so the walk SHALL go through its fields and
content as through any other element, and it SHALL never get `$env`. A component descriptor SHALL
carry its overrides in the reserved field `$env`, as a list of entries outermost first, each
holding `value` (the qualified name of the environment value: its module identity and name) and
`path` (a list of field names), and either `to` (the value in canonical form) or `formula`
(`module`, `node`, `fingerprint` and `locals`), with `fingerprint` written in the lossless form the
program fingerprints use. A bundle override SHALL be an entry holding only
`bundle`: the bundle's own entries, in order. An `env.Update` value SHALL be
`{ "$type": "env.Update", "$entries": [ … ] }` with entries of the same shape. A descriptor without
overrides SHALL have no `$env` field, and an element that is not a component descriptor SHALL never
get one. A formula whose module is not in the program, whose fingerprint differs from the module's,
or whose node is not a formula-flagged override value SHALL be refused with `nx-ir-boundary-type`
when it is evaluated, at a read that uses it; entering a descriptor SHALL check only the shape of
its `$env`, so a stale formula for a value the body never reads is not refused.

#### Scenario: An override reaches a descriptor through a host element
- **WHEN** a body evaluates to `<div env.colorMode=dark><Card /></div>`, where `Card` is an authored
  component
- **THEN** the `Card` descriptor in the result SHALL hold `$env` with one entry naming `colorMode`,
  an empty `path` and `to` equal to `dark`
- **AND** the `div` SHALL hold no `$env`

#### Scenario: An override on an external component reaches the components inside it
- **WHEN** a body evaluates to `<Stack env.colorMode=dark><Card /></Stack>`, where `Stack` is an
  external component and `Card` an authored one
- **THEN** the `Card` descriptor SHALL hold `$env` with the `dark` entry
- **AND** the `Stack` SHALL hold no `$env`

#### Scenario: The overrides where a use is built are innermost
- **WHEN** `<Label env.colorMode=light />` is built in one body and placed by another inside
  `<div env.colorMode=dark>`
- **THEN** the `Label` descriptor SHALL hold `$env` with the `dark` entry first and the `light`
  entry last

#### Scenario: An override that reads the environment is carried as a formula
- **WHEN** a body inside a `for item in items` evaluates to
  `<Card env.gap={env.space * item.scale} />`
- **THEN** the `Card` descriptor's `$env` entry for `gap` SHALL hold a `formula` naming the value's
  node and capturing `item`, and no `to`

#### Scenario: A formula from another build is refused
- **WHEN** a host stores a descriptor whose `$env` holds a formula, the module is rebuilt with a new
  fingerprint, and the host enters the stored descriptor
- **THEN** entering SHALL succeed
- **AND** a read in the body that uses the formula SHALL fail with `nx-ir-boundary-type`

### Requirement: Environment lookups follow precedence and evaluate defaults at the read
A runtime SHALL find the value of an environment value at a read from, in order: the overrides of
the `envProvide` nodes being evaluated around the read, nearest first; the overrides in `$env` of
the current component's descriptor, last entry first, and then those of each component above it;
the host's root value; the entry module's provides, last first; and the declaration's default.
An override with a non-empty path SHALL apply its value at that path to the value the later sources
give. A bundle SHALL contribute its entries for the value being read, last first, and no other entry
of it SHALL be evaluated for that read. A default, an entry-module provide and a formula SHALL be
evaluated at each read that uses it, in the environment of the read, with a formula's captured
locals bound, and while any of them is evaluated for a value, a read of that value at any depth
SHALL see only the sources after it. A read that reaches a value while its default is being
evaluated, with no source left after the default, SHALL fail with `nx-ir-environment-cycle` naming
the values on the path. The provides of a module that is not the entry module SHALL be ignored.

#### Scenario: A default is evaluated in the reader's environment
- **WHEN** `color`'s default reads `env.colorMode`, and `env.color` is read under an `envProvide`
  overriding `colorMode` to `dark`
- **THEN** the default SHALL be evaluated with `colorMode` equal to `dark`

#### Scenario: A formula is evaluated in the reader's environment
- **WHEN** a descriptor carries a `gap` formula `env.space * 2`, and `env.gap` is read under an
  `envProvide` overriding `space` to `1` inside that component
- **THEN** the read SHALL be `2`

### Requirement: Environment values have a defined cost
The format's documentation SHALL state the cost of the environment feature, and the runtimes SHALL
count it alike:

- an `envRead` node costs one operation, and a default, entry-module provide or formula evaluated
  for it is charged node by node each time it is evaluated;
- an `envProvide` node costs one operation, and its override values evaluated where written, its
  bundle nodes and its body are charged as any other children; capturing a formula costs one
  operation;
- an `envUpdate` node costs one operation, and its `base`, its values evaluated where written and
  each formula it captures are charged as in an `envProvide`;
- adding the overrides of an `envProvide` to one component descriptor costs one operation;
- finding a value through the environment, bundle entries included, costs nothing beyond the nodes
  it evaluates; a bundle's entries for other values are not evaluated for that read.

The conformance corpus SHALL hold cases for reads, defaults, field paths, `provide` around
expressions and markup, entry-module provides and bundles, bundles in `provide` blocks, `with`
steps, formulas, a bundle entry that reads another value the bundle sets, and root values, with
their results and operation counts.

#### Scenario: Both runtimes count a read of a default alike
- **WHEN** both runtimes evaluate a corpus case that reads an environment value whose default is
  `1 + 2`
- **THEN** both SHALL count one operation for the `envRead` node and three for the default

#### Scenario: A formula is charged at each read
- **WHEN** both runtimes evaluate a corpus case in which a component reads `env.gap` twice and its
  descriptor carries the formula `env.space * 2`
- **THEN** both SHALL charge the formula's nodes twice
