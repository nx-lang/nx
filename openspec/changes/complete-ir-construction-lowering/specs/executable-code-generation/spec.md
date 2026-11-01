## ADDED Requirements

### Requirement: Executable code supports conditional property fragments
Executable code generation SHALL support simple conditional, condition-list and match-style
property fragments. Only the selected branch SHALL contribute properties or evaluate its property
expressions. Match-arm narrowing, defaults, authored evaluation order and duplicate-property
validation SHALL be preserved in Rust IR, TypeScript IR and generated JavaScript execution.

#### Scenario: A conditional supplies its active label
- **WHEN** `enabled` is true in `<button if enabled { label="Save" } else { label="Disabled" } />`
- **THEN** execution SHALL produce a button whose label is `Save`
- **AND** false SHALL select `Disabled`

#### Scenario: A condition list chooses its first matching branch
- **WHEN** two condition-list arms are true
- **THEN** only the first arm SHALL contribute properties
- **AND** an inactive branch that would fail SHALL not be evaluated

#### Scenario: A match fragment reads the narrowed payload
- **WHEN** a fragment matches a payload case and binds a property from its payload
- **THEN** execution SHALL produce the property's payload value

#### Scenario: An omitted property retains its default
- **WHEN** the selected path contributes no binding for a defaulted property
- **THEN** construction SHALL apply the declared default
- **AND** an explicitly empty binding SHALL keep its distinct source semantics

#### Scenario: Duplicate active properties are rejected
- **WHEN** two properties can bind the same name on an active path
- **THEN** compilation SHALL reject the duplicate rather than apply last-wins behavior

### Requirement: Executable code supports positional record constructors
Executable code generation SHALL construct a record from positional arguments using declared field
order and existing type validation and default rules, including inherited and imported fields.
It SHALL NOT attempt to invoke the record declaration as a function value.

#### Scenario: Positional values bind in field order
- **WHEN** `User` declares `name:string` followed by `age:int = 30`
- **THEN** `User("Bob", 42).name` SHALL evaluate to `Bob`
- **AND** `User("Bob", 42).age` SHALL evaluate to `42`

#### Scenario: Omitted arguments use field defaults
- **WHEN** the same record is constructed as `User("Bob")`
- **THEN** its age SHALL be `30`

#### Scenario: Imported and inherited constructors preserve order
- **WHEN** a record with inherited fields is imported under an alias
- **THEN** positional construction SHALL bind the same fields and defaults as its equivalent markup
