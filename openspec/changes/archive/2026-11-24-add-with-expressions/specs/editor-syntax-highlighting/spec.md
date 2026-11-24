## MODIFIED Requirements

### Requirement: Environment declarations, reads and provides are scoped
The grammar SHALL scope `environment` as a declaration keyword when it is the first token of a
module member after an optional visibility modifier, followed by a name and `:`, and SHALL scope the
name as a variable declaration. It SHALL scope `provide` as a keyword at the start of a module
member and at the start of an expression or content item followed by `env`. It SHALL scope `env` as
a built-in namespace where it is followed by `.`, in expressions and in attribute names.
`environment` and `provide` in any other position SHALL keep the scope the same position gives any
other name.

#### Scenario: A declaration and a read
- **WHEN** a file contains `export environment space: int = 4` and `let gap() = { env.space * 2 }`
- **THEN** `environment` SHALL be scoped `keyword.declaration.environment.nx`
- **AND** both `env` tokens SHALL be scoped `support.variable.env.nx`

#### Scenario: A provide block and an attribute
- **WHEN** a file contains `{ provide env.colorMode=dark { <Header /> } }` and
  `<Section env.colorMode=dark />`
- **THEN** `provide` SHALL be scoped `keyword.control.provide.nx`
- **AND** `env` in the attribute SHALL be scoped `support.variable.env.nx`

#### Scenario: `provide` as a field name
- **WHEN** a file contains `type Plan = { provide:string }`
- **THEN** `provide` SHALL be scoped as a property name and not as a keyword

## ADDED Requirements

### Requirement: `with` is scoped as a keyword after an operand
The grammar SHALL scope `with` as an operator keyword where it follows an operand in an expression,
including after `provide env`. `with` as a property name, a field-list entry name, an attribute name,
after `.` in a member access, and in markup text SHALL keep the scope that position gives any other
name or text.

#### Scenario: `with` in an expression and in a provide
- **WHEN** a file contains `let v = { saved with { name="Bo" } }` and
  `{ provide env with brand { <Header /> } }`
- **THEN** both `with` tokens SHALL be scoped `keyword.operator.with.nx`

#### Scenario: `with` as a name and as text
- **WHEN** a file contains `type Opts = { with: int }`, `let n = { opts.with }` and
  `<p>Pay with card</p>`
- **THEN** none of those `with` tokens SHALL be scoped as a keyword
