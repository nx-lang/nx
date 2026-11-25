## ADDED Requirements

### Requirement: The source tree answers value suggestions when asked
A `sourceTree` request SHALL accept an optional `suggestions` boolean, defaulting to false. When it is
false the answer SHALL carry no suggestions and no suggestion keys, and SHALL be unchanged from an
answer without this feature. When it is true, the answer SHALL carry a `suggestions` list, computed
from the same analysis as the rest of the answer, and each property of a declaration entry, and
each value declaration entry, whose type has a suggestion key SHALL carry it as `suggestionKey`. The
list SHALL hold only entries whose key some `suggestionKey` of the answer carries. Each entry SHALL
carry:

- `name`: the value as the document would write it, such as `env.space.md`, `brandRed` or
  `d.standardPalette.blue600`;
- `key`: its suggestion key;
- `group`: which group it belongs to and the group's label, as "Suggestions are grouped by where
  they come from" says;
- `scope`: for a parameter or state field, the key of the top-level declaration whose body may use
  it; absent for every other entry;
- `type`: its type spelled in NX;
- `declaration`: the index in the declaration table of the declaration it starts from, which the
  table SHALL then list;
- `doc`: the doc comment of the field or declaration it names, when it has one;
- `preview`: its value at the defaults, when "A suggestion previews its default value statically"
  finds one.

The entries SHALL be listed by group in group order, and within a group in the order that
requirement gives. The answer SHALL round-trip through JSON without loss, and the wasm snapshot and
`@nx-lang/language-core`'s `sourceTree` SHALL give the same answer the Rust language service gives.

#### Scenario: Asking for suggestions
- **WHEN** a client asks with `suggestions` for the source tree of a document that imports
  `@nx/design` and constructs `<Card padding=8 />`, where `Card.padding` is typed `Spacing`
- **THEN** `Card`'s `padding` property SHALL carry a suggestion key
- **AND** the suggestions with that key SHALL include `env.space.md` with the type `Spacing`

#### Scenario: Without the option nothing changes
- **WHEN** a client asks for the source tree of the same document without `suggestions`
- **THEN** the answer SHALL carry no `suggestions` and no `suggestionKey`

#### Scenario: Parity between Rust and TypeScript
- **WHEN** the wasm SDK's language-service tests compute the source tree of each document in their
  corpus with `suggestions` through the TypeScript service and through the Rust service
- **THEN** the two answers SHALL be equal after JSON round-trip

### Requirement: A suggestion key comes from the declared type
The language service SHALL give a type a suggestion key from its declared type, with a trailing `?`
removed, by these rules:

- an alias whose chain of aliases ends in a primitive type, constrained or not, SHALL have the
  alias itself as its key;
- a record, union or action type, named directly or through a chain of aliases, SHALL have that
  type as its key;
- a primitive type (`int`, `float32`, `float64`, `string`, `boolean`, `uitext` and the rest),
  written bare or with an inline constraint, an element, function, sequence or object type, and an
  `env.Update` type SHALL have no key.

A key SHALL identify a declaration by its module identity and name, so two declarations with the
same name in different modules SHALL have different keys.

#### Scenario: Spacing is not radius
- **WHEN** a property is typed `Spacing` and another `Radius`, both `float64 where finite && >= 0.0`
  in `@nx/design`
- **THEN** their suggestion keys SHALL differ

#### Scenario: An alias of a record type
- **WHEN** a property is typed by `@nx/design`'s `Color`, an alias of `@nx/color`'s `Color`, and
  another by `@nx/color`'s `Color` through an import alias
- **THEN** both SHALL have `@nx/color`'s `Color` as their key

#### Scenario: A bare number
- **WHEN** a property is typed `float64 where finite && > 0.0`
- **THEN** it SHALL have no suggestion key

#### Scenario: A user type with the same name
- **WHEN** a document declares `type Spacing = int` and a property is typed by it
- **THEN** its key SHALL differ from the key of `@nx/design`'s `Spacing`

### Requirement: A named value matches a key by its type
A named value SHALL match a suggestion key when its type has that key, or, for a key that is an
alias of a primitive, when its declared type is an alias whose chain of aliases passes through that
alias, or, for a key that is a record type, when its type is a record the type checker accepts
where that record is expected through inheritance. A named value without a declared type SHALL
match by its inferred type. A value whose type is optional SHALL NOT match.

#### Scenario: A user alias of a token type
- **WHEN** a document declares `type ButtonGap = Spacing` and `let gap: ButtonGap = 6`
- **THEN** `gap` SHALL be suggested for a property typed `Spacing`
- **AND** a `let tight: Spacing = 2` SHALL NOT be suggested for a property typed `ButtonGap`

#### Scenario: An untyped color
- **WHEN** a document declares `let brandRed = #e11d48`
- **THEN** `brandRed` SHALL be suggested for a property typed `Color`

#### Scenario: An untyped number
- **WHEN** a document declares `let gap = 12`
- **THEN** `gap` SHALL NOT be suggested for a property typed `Spacing`

### Requirement: Suggestions are the named values the document can reach
The suggestions SHALL be these named values, where each matches a key some property or value
declaration of the answer carries:

- each environment value the module can see, as `env.` reads it, spelled `env.name`; one whose name
  is ambiguous for the module SHALL be left out;
- each parameter of a top-level component or function, and each state field of a component, spelled
  by its name, with the declaration as its scope;
- each top-level parameterless `let` value of the document, spelled by its name;
- each exported top-level parameterless `let` value of each module the document imports, and each
  internal one when the imported module is of the document's own library, spelled as the document
  refers to it: through the import's alias when it has one, and bare otherwise;
- for each of the above whose type is a record, each of its fields, spelled with a `.` and the field
  name after the value's name, and recursively each field of a field whose type is a record, at most
  four fields deep. A field that is optional, a sequence, or of a union type SHALL be suggested only
  when it matches, and SHALL NOT be followed.

A field SHALL be suggested whether or not the value it belongs to matches, and a value SHALL be
suggested whether or not its fields do. Functions, element functions, components, `for` bindings,
local `let`s, and values of modules the document does not import SHALL NOT be suggested.

#### Scenario: Tokens and their fields
- **WHEN** a document imports `@nx/design` and a property is typed `Color`
- **THEN** the suggestions with its key SHALL include `env.color.accent`, `env.color.text.muted` and
  `standardPalette.blue600`
- **AND** SHALL NOT include `env.color` or `env.color.text`

#### Scenario: An intermediate record
- **WHEN** a property is typed `TextColors`
- **THEN** the suggestions with its key SHALL include `env.color.text`

#### Scenario: An import alias
- **WHEN** a document writes `import "@nx/design" as d` and a property is typed `Color`
- **THEN** the suggestions SHALL include `d.standardPalette.blue600` and `env.color.accent`

#### Scenario: A component's own property
- **WHEN** `component <Badge tint: Color /> = { <Dot fill={tint} /> }` and `Dot.fill` is typed
  `Color`
- **THEN** the suggestions SHALL include `tint` with `Badge` as its scope

### Requirement: A value is not suggested inside itself
A suggestion SHALL NOT be offered for a value written inside the declaration it starts from: a
suggestion without a `scope` whose `declaration` is the top-level `let`, environment value, component or function
whose source holds the property or value declaration SHALL be marked `self` with that declaration's
key, so that a client leaves it out there and offers it elsewhere.

#### Scenario: A theme does not suggest itself
- **WHEN** a document declares `let brand: SemanticColors = <SemanticColors accent=#e11d48 … />`
- **THEN** the suggestion `brand.accent` SHALL be marked `self` with the key of `brand`

#### Scenario: A token's default does not read itself
- **WHEN** `@nx/design`'s `environment color` default is shown
- **THEN** every `env.color.*` suggestion SHALL be marked `self` with the key of `color`

### Requirement: Suggestions are grouped by where they come from
Each suggestion SHALL belong to one group, in this order:

1. `environment`, labeled `Environment`;
2. `declaration`, the parameters and state of one top-level declaration, labeled with its name;
3. `document`, this document's `let` values, labeled `This document`;
4. `import`, one per imported module in import order, labeled with the import's path as written,
   such as `@nx/design` or `./brand.nx`.

Within a group, values SHALL follow declaration order, environment values in the order their
modules are imported and then declared, and each value's fields SHALL follow it in its record's
field order, depth first.

#### Scenario: Scales read small to large
- **WHEN** a property is typed `Spacing` in a document that imports `@nx/design`
- **THEN** the environment suggestions SHALL read `env.space.none`, `env.space.xxs`, `env.space.xs`,
  `env.space.sm`, `env.space.md`, `env.space.lg`, `env.space.xl`, `env.space.xxl` and
  `env.space.xxxl`, in that order

#### Scenario: Environment first
- **WHEN** a document declares `let brandRed = #e11d48` and imports `@nx/design`
- **THEN** every `environment` suggestion SHALL precede `brandRed`, which SHALL precede
  `standardPalette.blue600`

### Requirement: A suggestion previews its default value statically
A suggestion SHALL carry a `preview` when its value at the defaults can be found without running the
program, by following, at most 16 steps:

- a name to its declaration's value, or an environment value to its default;
- a reference to another top-level `let` or an `env.` read to that value;
- a field to the attribute of the record or element construction that sets it, or to the field's
  default when the construction leaves it out;
- an `if` or `is` on an `env.` read whose own default is a bare case, to the arm for that case.

It SHALL stop with a preview at a string, number or boolean literal (`string`, `number` with the
literal's text, `boolean`), at a bare case (`case` with its name), or at a color the color-tooling
discovery recognizes (`color` with its `toCss` text, its hex when it is inside sRGB, and whether it
is outside sRGB), and SHALL give no preview at anything else. A preview SHALL NOT follow main-module
`provide`s, host root values or overrides, and SHALL NOT run NX functions.

#### Scenario: A token's color
- **WHEN** `env.color.accent` is suggested in a document that imports `@nx/design`
- **THEN** its preview SHALL be the color `standardPalette.blue600` gives, following the `light`
  arm because `colorMode` defaults to `light`

#### Scenario: A spacing step
- **WHEN** `env.space.md` is suggested
- **THEN** its preview SHALL be the number `@nx/design` gives `md` as its default

#### Scenario: A computed value
- **WHEN** a document declares `let wide: Spacing = gap * 2`
- **THEN** the suggestion `wide` SHALL carry no preview
