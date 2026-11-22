## ADDED Requirements

### Requirement: NX IR encodes environment values behind a required feature
NX IR SHALL encode an environment value as a declaration of kind `environment` holding its type and
its default node, and exported as other declarations are. A read SHALL be an `envRead` node, kind
`30`, holding a reference to the declaration, with any field read after it encoded as ordinary
`member` nodes. A `provide` block, and an element with `env.` attributes, SHALL be an `envProvide`
node, kind `31`, holding a list of overrides and a body, where each override holds a reference to
the declaration, a list of field names that may be empty, and a value node. An `env.Update` bundle
SHALL be an `envUpdate` node, kind `32`, holding a list of overrides of the same shape. A module
with top-level provides SHALL hold one declaration of kind `provides`, named `$provides`, listing
them in source order, each holding an optional reference to the declaration, a list of field names
and a value node: an override names its declaration, and a bundle names none and its node is of
`env.Update` type. A module that declares an environment value, contains an `envRead`,
`envProvide` or `envUpdate` node or has top-level provides SHALL list the required feature
`environment-v1`, and a module that does none of these SHALL NOT. The `explain` text SHALL
read the nodes as `env.` reads and `provide` blocks. Adding the declaration kind, the node kinds and
the feature SHALL NOT change the IR schema version.

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

### Requirement: A component descriptor carries the overrides around it in `$env`
A runtime that evaluates an `envProvide` node SHALL evaluate the override values in order, in the
environment outside the node, then evaluate the body with the overrides in effect, then add the
overrides to every component descriptor its result reaches. The walk SHALL go through sequences,
records, and the fields and content of elements that are not component descriptors, and SHALL stop
at each component descriptor, which SHALL receive the overrides in front of any it already holds.
A component descriptor SHALL carry its overrides in the reserved field `$env`, as a list of entries
outermost first, each holding `value` (the qualified name of the environment value: its module
identity and name), `path` (a list of field names) and `to` (the value in canonical form). A
descriptor without overrides SHALL have no `$env` field, and an element that is not a component
descriptor SHALL never get one.

#### Scenario: An override reaches a descriptor through a host element
- **WHEN** a body evaluates to `<div env.colorMode=dark><Card /></div>`, where `Card` is an authored
  component
- **THEN** the `Card` descriptor in the result SHALL hold `$env` with one entry naming `colorMode`,
  an empty `path` and `to` equal to `dark`
- **AND** the `div` SHALL hold no `$env`

#### Scenario: The overrides where a use is built are innermost
- **WHEN** `<Label env.colorMode=light />` is built in one body and placed by another inside
  `<div env.colorMode=dark>`
- **THEN** the `Label` descriptor SHALL hold `$env` with the `dark` entry first and the `light`
  entry last

### Requirement: Environment lookups follow precedence and evaluate defaults at the read
A runtime SHALL find the value of an environment value at a read from, in order: the overrides of
the `envProvide` nodes being evaluated around the read, nearest first; the overrides in `$env` of
the current component's descriptor, last entry first, and then those of each component above it;
the host's root value; the entry module's provides, last first; and the declaration's default.
An override with a non-empty path SHALL apply its value at that path to the value the later sources
give. A default and an entry-module provide SHALL be evaluated at each read that uses it, in the
environment of the read, and while an entry-module provide of a value is evaluated, a read of that
value SHALL see only the sources after it. The provides of a module that is not the entry module
SHALL be ignored.

#### Scenario: A default is evaluated in the reader's environment
- **WHEN** `color`'s default reads `env.colorMode`, and `env.color` is read under an `envProvide`
  overriding `colorMode` to `dark`
- **THEN** the default SHALL be evaluated with `colorMode` equal to `dark`

### Requirement: Environment values have a defined cost
The format's documentation SHALL state the cost of the environment feature, and the runtimes SHALL
count it alike:

- an `envRead` node costs one operation, and a default or entry-module provide evaluated for it is
  charged node by node each time it is evaluated;
- an `envProvide` node costs one operation, and its override values and its body are charged as any
  other children;
- adding the overrides of an `envProvide` to one component descriptor costs one operation;
- finding a value through the environment costs nothing beyond the nodes it evaluates.

The conformance corpus SHALL hold cases for reads, defaults, field paths, `provide` around
expressions and markup, entry-module provides and bundles, and root values, with their results and
operation counts.

#### Scenario: Both runtimes count a read of a default alike
- **WHEN** both runtimes evaluate a corpus case that reads an environment value whose default is
  `1 + 2`
- **THEN** both SHALL count one operation for the `envRead` node and three for the default
