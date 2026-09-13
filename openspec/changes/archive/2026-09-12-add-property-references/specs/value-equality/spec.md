## Purpose

Defines the language's equality operators, `==` and `!=`, over every kind of value.

## ADDED Requirements

### Requirement: Equality compares records and lists structurally
The equality operators `==` and `!=` SHALL compare scalars by value, `null` as equal only to
`null`, a constant union case by its union and case name, a list to a list of the same length whose
elements are equal in order, and a record to a record of the same declared type whose every field
is equal, recursively. Two records of different declared types SHALL NOT be equal. The same
equality SHALL be used wherever the language compares values, including `diff` over update records.

#### Scenario: Records with equal fields are equal
- **WHEN** a file contains `type Address = { city:string } let rome = <Address city="Rome" /> let again = <Address city="Rome" /> let paris = <Address city="Paris" /> let same = {rome == again} let other = {rome == paris}`
- **THEN** evaluating `same` SHALL produce `true` and `other` SHALL produce `false`

#### Scenario: Lists compare element-wise in order
- **WHEN** a file contains `let ones = { 1 2 } let again = { 1 2 } let twos = { 2 1 } let same = {ones == again} let other = {ones == twos}`
- **THEN** evaluating `same` SHALL produce `true` and `other` SHALL produce `false`
