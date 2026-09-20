## ADDED Requirements

### Requirement: The range operators are scoped as operators
The TextMate grammar and the tree-sitter highlight queries SHALL scope `..` and `..=` as operators
wherever a value expression admits them, including a `for` header. Neither SHALL be scoped as two
member-access dots or as a dot followed by an assignment, and a number directly before or after the
operator SHALL keep its numeric scope: `1..5` is two integer literals around an operator, not a
real literal.

#### Scenario: A range in a `for` header
- **WHEN** the grammar tokenizes `for i in 0..count { i }`
- **THEN** `..` SHALL be scoped as an operator, `0` as a numeric literal and `count` as a variable

#### Scenario: The inclusive operator is one token
- **WHEN** the grammar tokenizes `let r = 1..=5`
- **THEN** `..=` SHALL be scoped as a single operator and `1` and `5` as numeric literals

#### Scenario: Member access beside a range
- **WHEN** the grammar tokenizes `page.first..page.last`
- **THEN** the dots in `page.first` and `page.last` SHALL keep their member-access scope and `..` SHALL be scoped as an operator
