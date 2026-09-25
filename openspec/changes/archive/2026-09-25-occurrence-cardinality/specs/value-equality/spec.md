## MODIFIED Requirements

### Requirement: Equality compares records and lists structurally
The equality operators `==` and `!=` SHALL compare scalars by value, a constant union case by its
union and case name, a sequence to a sequence of the same length whose items are equal in order, and
a record to a record of the same declared type whose every field is equal, recursively. Two records
of different declared types SHALL NOT be equal. Every value SHALL compare as a sequence: an
exactly-one value is a sequence of one item, and the empty value `{}` is the sequence of no items,
so `{}` is equal only to `{}`, and a value that holds an item is never equal to `{}`. The same
equality SHALL be used wherever the language compares values, including `diff` over update records.

#### Scenario: Records with equal fields are equal
- **WHEN** a file contains `type Address = { city:string } let rome = <Address city="Rome" /> let again = <Address city="Rome" /> let paris = <Address city="Paris" /> let same = {rome == again} let other = {rome == paris}`
- **THEN** evaluating `same` SHALL produce `true` and `other` SHALL produce `false`

#### Scenario: Lists compare element-wise in order
- **WHEN** a file contains `let ones = { 1 2 } let again = { 1 2 } let twos = { 2 1 } let same = {ones == again} let other = {ones == twos}`
- **THEN** evaluating `same` SHALL produce `true` and `other` SHALL produce `false`

#### Scenario: An item compares as a sequence of one
- **WHEN** a file contains `let one:int? = 1 let ones:int* = { 1 } let two:int+ = { 1 2 } let same = {one == ones} let other = {one == two}`
- **THEN** evaluating `same` SHALL produce `true` and `other` SHALL produce `false`

#### Scenario: The empty value is equal only to itself
- **WHEN** a file contains `let none:int? = {} let one:int? = 1 let a = {none == {}} let b = {one == {}} let c = {one != {}}`
- **THEN** evaluating `a` SHALL produce `true`, `b` SHALL produce `false` and `c` SHALL produce `true`
