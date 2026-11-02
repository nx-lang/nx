## ADDED Requirements

### Requirement: Named record construction requires element syntax
Shared source analysis SHALL reject parenthesized construction of named record declarations with
`record-construction-requires-element`, before code generation or execution. The diagnostic SHALL
retain caller file identity and label the offending callee/call. It SHALL explain that records require named element construction and include a short illustrative
example such as `<User ... />`. This diagnostic SHALL NOT require field enumeration, positional
argument mapping, source-expression reconstruction or a generated rewrite/code action.
Semantic callee resolution,
not spelling or result type, SHALL determine whether this rule applies. Existing function calls,
factory functions, first-class-function and shadowing rules SHALL remain unchanged.

#### Scenario: Positional record call gives element guidance
- **WHEN** `User` declares `name:string` and `age:int = 30` and source calls `User("Bob", 42)`
- **THEN** analysis SHALL report `record-construction-requires-element` at that call
- **AND** its message SHALL direct the caller to named element syntax, such as `<User ... />`
- **AND** executable generation and CLI execution SHALL stop on the static error

#### Scenario: Named construction retains defaults
- **WHEN** `User` is constructed as `<User name="Bob" />`
- **THEN** its age SHALL remain `30` under the existing default rules
- **AND** rejecting `User("Bob")` SHALL NOT require generating that replacement expression

#### Scenario: Zero-argument record construction is rejected
- **WHEN** a record with no required fields is called with parentheses and no arguments
- **THEN** analysis SHALL reject it and direct the caller to element construction

#### Scenario: Semantic aliases and inheritance do not bypass rejection
- **WHEN** a parenthesized target resolves to a named record through an import, qualified name or type alias
- **THEN** the same diagnostic SHALL be reported, including for inherited or generic records

#### Scenario: A factory returning a record remains a function
- **WHEN** an ordinary function returns a named record using element construction
- **THEN** its parentheses invocation SHALL remain accepted under existing function rules
- **AND** its record result type alone SHALL NOT trigger this diagnostic

#### Scenario: Tooling shares the source diagnostic
- **WHEN** checker-backed tooling analyzes an offending record call
- **THEN** its diagnostics SHALL contain the same code and original source location
- **AND** the error SHALL NOT be deferred into an attempted runtime function invocation

#### Scenario: A callable binding is resolved before applying the record rule
- **WHEN** existing name-resolution rules select an ordinary function or function-typed binding whose spelling also occurs as a record name
- **THEN** the compiler SHALL apply the existing invocation rules for that binding
- **AND** it SHALL NOT report this record-construction diagnostic solely because the spelling matches a type

#### Scenario: Record type references remain valid outside constructor calls
- **WHEN** a record name is used as a type annotation or as a permitted compile-time type argument
- **THEN** this restriction SHALL NOT reject the type reference

#### Scenario: Argument shape does not require specialized guidance
- **WHEN** a resolved record call supplies an empty, partial or invalid positional argument list
- **THEN** analysis SHALL report the same element-syntax requirement without reconstructing a replacement
- **AND** it SHALL NOT cascade into a missing-function diagnostic for the record declaration
