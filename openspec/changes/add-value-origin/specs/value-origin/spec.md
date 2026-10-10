## Purpose

Defines the origin of an evaluated record: the place in the program's source of the element
expression that constructed it. Origins let a tool go from a program's output back to its source,
which is what links a preview to the NX viewer.

## ADDED Requirements

### Requirement: A record's origin is the element expression that constructed it
The origin of a record value SHALL be the identity of the module and the source span of the element
expression whose evaluation constructed the record. A record SHALL keep its origin when it is
bound to a name, passed as a prop or an argument, stored in or read from component state, placed in
a sequence, or returned from a function or a component body. A record built by an update record's
application, a `merge` or an `apply` SHALL take the origin of the expression that applied it. A
value that is not a record SHALL have no origin.

#### Scenario: A question shown through a step
- **WHEN** a module declares `let roleQuestion = <SingleChoice id="role" … />` and a component
  renders `<Step question={roleQuestion} />`
- **THEN** the `SingleChoice` record in the rendered output SHALL have as its origin the span of the
  `<SingleChoice … />` element in `let roleQuestion`

#### Scenario: A record built in a loop
- **WHEN** a component renders `for item in items { <Row label={item.name} /> }` over three items
- **THEN** each of the three `Row` records SHALL have as its origin the span of the one `<Row … />`
  element in the loop body

#### Scenario: A library record
- **WHEN** a standard library function constructs a record that a program returns
- **THEN** the record's origin SHALL name the library module's identity and its span in that module

### Requirement: Origins are matched to source by module and byte offsets
An origin's span SHALL give UTF-8 byte offsets into the source of the module it names, the offsets
the source tree's nodes carry, so that the source tree node of an origin is the element node of
that module whose byte range equals the origin's.

#### Scenario: From preview to source tree
- **WHEN** a tool holds the source tree of the module an origin names
- **THEN** exactly one element node of that tree SHALL have the origin's start and end byte offsets
