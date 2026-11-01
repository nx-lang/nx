## ADDED Requirements

### Requirement: Named record construction requires element syntax
Shared source analysis SHALL reject parenthesized construction of named record declarations with
`record-construction-requires-element`, before code generation or execution. The diagnostic SHALL
retain caller file identity and label the offending callee/call. It SHALL explain named element
construction and show a suitable example or rewrite when safely derivable from resolved metadata.
It SHALL NOT fabricate field mappings or silently discard arguments. Semantic callee resolution,
not spelling or result type, SHALL determine whether this rule applies. Existing function calls,
factory functions, first-class-function and shadowing rules SHALL remain unchanged.

#### Scenario: Positional record call gives element guidance
- **WHEN** `User` declares `name:string` and `age:int = 30` and source calls `User("Bob", 42)`
- **THEN** analysis SHALL report `record-construction-requires-element` at that call
- **AND** its guidance SHALL show `<User name="Bob" age=42 />`
- **AND** executable generation and CLI execution SHALL stop on the static error

#### Scenario: Omitted default stays omitted in guidance
- **WHEN** the same record is called as `User("Bob")`
- **THEN** analysis SHALL reject the call and suggest `<User name="Bob" />`
- **AND** equivalent element construction SHALL continue to supply age `30`

#### Scenario: Zero-argument record construction is rejected
- **WHEN** a record with no required fields is called with parentheses and no arguments
- **THEN** analysis SHALL reject it and direct the caller to element construction

#### Scenario: Semantic aliases and inheritance do not bypass rejection
- **WHEN** a parenthesized target resolves to a named record through an import, qualified name or type alias
- **THEN** the same diagnostic SHALL be reported, including for inherited or generic records
- **AND** any detailed field guidance SHALL use resolved metadata or fall back to accurate generic guidance

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

#### Scenario: Malformed record calls receive safe guidance
- **WHEN** a resolved record call has an argument list that cannot be safely rewritten as named construction
- **THEN** the diagnostic SHALL still explain the element-syntax requirement
- **AND** it SHALL NOT present a complete rewrite that drops arguments or misbinds fields
- **AND** it SHALL NOT cascade into a missing-function diagnostic for the record declaration
