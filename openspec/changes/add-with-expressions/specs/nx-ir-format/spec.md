## MODIFIED Requirements

### Requirement: A component descriptor carries the overrides around it in `$env`
A runtime that evaluates an `envProvide` node SHALL evaluate the body with the overrides in effect,
then add the overrides to every component descriptor its result reaches. An override whose value
reads no environment value SHALL be evaluated once, in order, in the environment outside the node,
and carried as its value. Any other override, and a bundle override, SHALL be carried as a
**formula**: the override's value node together with the values of the locals it reads, captured
where it is written, which a runtime evaluates at a read. The walk SHALL go through sequences,
records, and the fields and content of elements that are not component descriptors, and SHALL stop
at each component descriptor, which SHALL receive the overrides in front of any it already holds.
A component descriptor here SHALL be a record whose `$type` names a component declared with a body;
the record of an external component is the host's to interpret, so the walk SHALL go through its
fields and content as through any other element, and it SHALL never get `$env`. A component
descriptor SHALL carry its overrides in the reserved field `$env`, as a list of entries outermost
first, each holding `value` (the qualified name of the environment value: its module identity and
name) and `path` (a list of field names), with either `to` (the value in canonical form) or
`formula` (the module identity and node of the value, and the captured locals in canonical form); a
bundle entry SHALL hold `bundle` with a formula in place of `value`, `path` and `to`. A descriptor
without overrides SHALL have no `$env` field, and an element that is not a component descriptor
SHALL never get one. Formulas are behind the required feature `environment-v2`.

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

### Requirement: Environment lookups follow precedence and evaluate defaults at the read
A runtime SHALL find the value of an environment value at a read from, in order: the overrides of
the `envProvide` nodes being evaluated around the read, nearest first; the overrides in `$env` of
the current component's descriptor, last entry first, and then those of each component above it;
the host's root value; the entry module's provides, last first; and the declaration's default.
An override with a non-empty path SHALL apply its value at that path to the value the later sources
give, and a bundle SHALL contribute the overrides it holds for the value, in its order. A default,
an entry-module provide and a formula SHALL be evaluated at each read that uses it, in the
environment of the read, and while any of them is evaluated for a value, a read of that value SHALL
see only the sources after it. The provides of a module that is not the entry module SHALL be
ignored.

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
- an `envProvide` node costs one operation, and its override values evaluated where written and its
  body are charged as any other children; capturing a formula costs one operation;
- adding the overrides of an `envProvide` to one component descriptor costs one operation;
- finding a value through the environment costs nothing beyond the nodes it evaluates.

The conformance corpus SHALL hold cases for reads, defaults, field paths, `provide` around
expressions and markup, entry-module provides and bundles, bundles in `provide` blocks, formulas,
and root values, with their results and operation counts.

#### Scenario: Both runtimes count a read of a default alike
- **WHEN** both runtimes evaluate a corpus case that reads an environment value whose default is
  `1 + 2`
- **THEN** both SHALL count one operation for the `envRead` node and three for the default

#### Scenario: A formula is charged at each read
- **WHEN** both runtimes evaluate a corpus case in which a component reads `env.gap` twice and its
  descriptor carries the formula `env.space * 2`
- **THEN** both SHALL charge the formula's nodes twice
