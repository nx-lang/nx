## ADDED Requirements

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
match by its inferred type, where a union case's element, such as `<Paint.color … />`, has its
union's type. A value whose type is optional SHALL NOT match. A property typed by an alias of a
token type, such as `ButtonGap = Spacing`, SHALL get only the values declared with that alias or an
alias of it, not those of `Spacing`.

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
declaration of the answer carries, and each is a name the module's top-level scope resolves without
a diagnostic:

- each environment value the module can see, as `env.` reads it, spelled `env.name`; one whose name
  is ambiguous for the module SHALL be left out;
- each parameter of a top-level component or function, and each state field of a component, spelled
  by its name, with the declaration as its scope;
- each top-level parameterless `let` value of the document, spelled by its name;
- each top-level parameterless `let` value that another file of the same library, or another root
  module of the same program, declares without `private`, which the module sees without an import,
  spelled by its name;
- each top-level parameterless `let` value a module the document imports makes visible to it,
  spelled as the document refers to it: through the import's alias when it has one, through the
  qualified alias a selective import gives the name (`import { standardPalette as
  d.standardPalette } from "@nx/design"` gives `d.standardPalette`), and bare otherwise; a selective import SHALL bring only the names it lists;
- a name that is ambiguous for the module, or that a nearer declaration shadows, SHALL be left out
  under that spelling; a top-level name that a parameter or state field of a component or function
  shadows SHALL list that declaration in `self`, so it is not offered inside it;
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

#### Scenario: A peer file's value
- **WHEN** a program's `brand.nx` declares `let brandRed = #e11d48` and its `main.nx`, without
  importing `brand.nx`, writes a `Color` property
- **THEN** the suggestions for that property SHALL include `brandRed`

#### Scenario: A selective import
- **WHEN** a document writes `import { standardPalette as d.standardPalette } from "@nx/design"`
  and a property is typed `Color`
- **THEN** the suggestions SHALL include `d.standardPalette.blue600`, and no other `let` of
  `@nx/design`

#### Scenario: An ambiguous name
- **WHEN** a document imports two modules that each export `let accent: Color` and a property is
  typed `Color`
- **THEN** the suggestions SHALL NOT include a bare `accent`

#### Scenario: A parameter shadows a top-level value
- **WHEN** a document declares `let tint = #e11d48` and `component <Badge tint: Color /> = { … }`
- **THEN** the suggestion `tint` without a scope SHALL list `Badge` in `self`, and the scoped `tint`
  SHALL have `Badge` as its scope

#### Scenario: A component's own property
- **WHEN** `component <Badge tint: Color /> = { <Dot fill={tint} /> }` and `Dot.fill` is typed
  `Color`
- **THEN** the suggestions SHALL include `tint` with `Badge` as its scope

### Requirement: A value is not suggested inside itself
A suggestion without a `scope` SHALL list in `self` each top-level `let`, environment value,
component or function of the document whose source would read it in a cycle: the declaration it
starts from, and each declaration that it reads, directly or through the values, environment
defaults and functions it reads, as hover's list of environment reads follows them. A client SHALL
leave a suggestion out for a property or value written inside a declaration its `self` lists, and
SHALL offer it elsewhere.

#### Scenario: A theme does not suggest itself
- **WHEN** a document declares `let brand: SemanticColors = <SemanticColors accent=#e11d48 … />`
  and asks for its source tree with `suggestions`
- **THEN** the suggestion `brand.accent` SHALL list `brand` in `self`

#### Scenario: A token's default does not read itself
- **WHEN** a client asks with `suggestions` for the source tree of `@nx/design`'s `tokens.nx`
- **THEN** every `env.color.*` suggestion SHALL list the environment value `color` in `self`
- **AND** every `env.text.*` suggestion SHALL list `font` in `self`, since `text`'s default reads
  `env.font`

### Requirement: Suggestions are grouped by where they come from
Each suggestion SHALL belong to one group, in this order:

1. `environment`, labeled `Environment`;
2. `declaration`, the parameters and state of one top-level declaration, labeled with its name;
3. `document`, this document's `let` values, labeled `This document`;
4. `peer`, one per other file of the library or program whose values the module sees without an
   import, in path order, labeled with its path relative to the document, such as `./brand.nx`;
5. `import`, one per imported module in import order, labeled with the import's path as written,
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
discovery recognizes (`color`, with the same swatch text and outside-sRGB mark a source node's color
carries), and SHALL give no preview at anything else. Color discovery SHALL resolve a constructor
through the imports of the module that writes it, as `palette.nx`'s `c.oklch(…)`. A preview SHALL
NOT follow main-module `provide`s, host root values or overrides, and SHALL NOT run NX functions.

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
