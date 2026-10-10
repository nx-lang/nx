## Purpose

Defines the origin of an evaluated record: the place in the program's source of the element
expression that constructed it, and how an IR runtime reports it. Origins let a tool go from a
program's output back to its source, which is what links a preview to the NX viewer.

## ADDED Requirements

### Requirement: A record's origin is the element expression that constructed it
The origin of a record value SHALL be the identity of the module and the byte span, from the
image's debug section, of the IR node of the element expression whose evaluation constructed the
record. A record SHALL keep its origin when it is bound to a name, passed as a prop or an argument,
stored in or read from component state, placed in a sequence, or returned from a function or a
component body. A record built by an update record's application or by `apply` SHALL take the
origin of the expression that applied it. A record built by a node whose image has no debug section
SHALL have no origin. A value that is not a record SHALL have no origin. Origins SHALL NOT take
part in value equality.

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
- **WHEN** a library function constructs a record that a program returns, and the library's image
  carries its debug section
- **THEN** the record's origin SHALL name the library module's identity and its span in that module

### Requirement: An IR runtime reports origins to a host that asks
An IR runtime's options SHALL accept an origins report, as they accept a usage report. A call of
`evaluateFunction`, `initializeComponent`, `evaluateComponent` or `dispatchComponentActions` given
one SHALL clear it when the call begins and, when the call returns, fill it with one entry for each
record in the value the call returned that has an origin: the function's value, or the rendered
output. Each entry SHALL name the record by its JSON pointer (RFC 6901) within that value, as the
host receives the value, and give the origin's module identity and start and end byte offsets.
Entries SHALL be in the order of a depth-first walk of the value, a record before its fields and
fields in the order the value lists them. A call that fails SHALL leave the report empty. A call
given no report SHALL collect nothing, and its result SHALL be exactly what it is without this
requirement.

#### Scenario: Rendered output with origins
- **WHEN** a host initializes a component whose body renders `<Screen><Label text="Hi" /></Screen>`
  from an image built with its debug section, passing an origins report
- **THEN** the report SHALL hold an entry for `""` with the span of the `<Screen>` element and one
  for the `Label` record's pointer with the span of the `<Label … />` element

#### Scenario: An image without its debug section
- **WHEN** a host makes the same call with an image built without its debug section
- **THEN** the call SHALL succeed and the report SHALL be empty

#### Scenario: No report, no change
- **WHEN** a host makes the same call without a report
- **THEN** its result SHALL equal the result of the same call before origins existed

### Requirement: The IR runtimes report the same origins
The TypeScript and Rust IR runtimes SHALL fill the origins report with equal entries for the same
call on the same images.

#### Scenario: Corpus parity
- **WHEN** the runtime conformance corpus runs on both runtimes with a report and images built with
  their debug sections
- **THEN** every case's report SHALL be equal between the two runtimes

### Requirement: Origins are matched to source by module and byte offsets
An origin's span SHALL give UTF-8 byte offsets into the source of the module it names, the offsets
the source tree's nodes carry, so that the source tree node of an origin is the element node of
that module whose byte range equals the origin's.

#### Scenario: From preview to source tree
- **WHEN** a tool holds the source tree of the module an origin names
- **THEN** exactly one element node of that tree SHALL have the origin's start and end byte offsets
