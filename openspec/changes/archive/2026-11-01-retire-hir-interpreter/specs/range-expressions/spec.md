## MODIFIED Requirements

### Requirement: `for` iterates a range of an integer type
A `for`, in both its value and its element form, SHALL accept as its iterable a list or a
`<Range T=X/>` of the prelude's `Range` where `X` is `int`, `int32` or `int64`. Over a range the
item SHALL have type `X` and SHALL take each integer from `start` upward, in order, stopping before
`end` for a half-open range and after `end` for a closed one; a range whose start is not below its
end — or, when closed, is above its end — SHALL yield no items. The optional index variable SHALL be
an `int` counting from zero. The result SHALL be the list of body values, as for a list. A range of
any other type SHALL be rejected as an iterable with a diagnostic that names the type and says only
integer ranges iterate. Evaluation SHALL NOT build the list of integers before running the body.

#### Scenario: A half-open range counts up to its end
- **WHEN** a file contains `let squares = { for i in 0..4 { i * i } }`
- **THEN** `squares` SHALL evaluate to the list `0 1 4 9`

#### Scenario: A closed range includes its end
- **WHEN** a file contains `let pages = { for page in 1..=3 { page } }`
- **THEN** `pages` SHALL evaluate to the list `1 2 3`

#### Scenario: The index counts from zero
- **WHEN** a file contains `let pairs = { for value, index in 5..8 { value * 10 + index } }`
- **THEN** `pairs` SHALL evaluate to the list `50 61 72`

#### Scenario: Empty and reversed ranges yield nothing
- **WHEN** a file contains `let count = 0` and `let a = { for i in 0..count { i } }` and `let b = { for i in 5..2 { i } }` and `let c = { for i in 1..=0 { i } }`
- **THEN** `a`, `b` and `c` SHALL each evaluate to the empty list

#### Scenario: A one-element closed range
- **WHEN** a file contains `let one = { for i in 3..=3 { i } }`
- **THEN** `one` SHALL evaluate to the list `3`

#### Scenario: A range value from anywhere iterates
- **WHEN** a file contains `let <Stars count:int /> = { for i in 0..count { <Star /> } }` and `let span:<Range T=int/> = <Range T=int start={2} end={4} endInclusive={true} />` and `let items = { for i in span { i } }`
- **THEN** `<Stars count={5} />` SHALL render five `Star` elements
- **AND** `items` SHALL evaluate to the list `2 3 4`

#### Scenario: The item has the range's integer type
- **WHEN** a file contains `let lo:int32 = 1` and `let hi:int32 = 3` and `let xs:int32* = { for i in lo..hi { i } }`
- **THEN** analysis SHALL accept the binding

#### Scenario: A non-integer range does not iterate
- **WHEN** a file contains `let bad = { for x in 0..2.5 { x } }`
- **THEN** analysis SHALL reject the iterable with a diagnostic naming `<Range T=float64/>` and saying only integer ranges iterate

#### Scenario: Evaluation agrees across backends
- **WHEN** a program builds ranges with both operators, compares two for equality, iterates one with an index, and iterates an empty one
- **THEN** the Rust IR runtime, the generated executable code and the TypeScript IR runtime SHALL produce the same values
