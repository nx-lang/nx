## ADDED Requirements

### Requirement: An example counts with a range rather than by hand
Where an example needs a collection of a given size whose items follow from their position, its
source SHALL derive that collection from a range — a `for` over `a..b` or `a..=b` — rather than
from a written-out seed list, a repeated element, or a helper that concatenates shifted copies of
one. No example's source SHALL say that NX has no range, and no example SHALL carry scaffolding
whose only purpose is to reach a count. This SHALL hold for a collection of any size an example
draws, including the hundred thousand cells the Cells example binds.

#### Scenario: A list example's collection is one loop
- **WHEN** the Cells or Uneven Cells example is opened
- **THEN** its collection SHALL be built by one `for` over a range whose bound is the item count
  the DrawnUI original demonstrates
- **AND** the source SHALL declare no seed list, shift function, or multiplying helper

#### Scenario: No example claims the language cannot count
- **WHEN** the example set's sources are inspected
- **THEN** none SHALL state that NX has no range, and none SHALL excuse code by that absence

#### Scenario: A gap that remains is still stated in place
- **WHEN** an example cycles a palette or otherwise needs to read a list by position, which NX has
  no operator for
- **THEN** its source SHALL say so where the value is chosen, and SHALL NOT attribute the
  workaround to the absence of a range

#### Scenario: The rewritten examples still compile and draw
- **WHEN** the example check runs over the set
- **THEN** every example SHALL compile with no diagnostics and evaluate, as before the rewrite
