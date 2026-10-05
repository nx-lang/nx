## ADDED Requirements

### Requirement: A host value is a canonical value in the host's own form
What a host passes to an evaluation API, and what an evaluation API returns, SHALL be canonical
values. A canonical value is one of: the empty value; a boolean; a number, which is an integer or
a float of the width the site it reaches declares; a string; a sequence of canonical values; a
record, which holds canonical values under field names and may carry a type name; and a function
value or an action handler, which only a runtime makes and a host passes back. Nothing else is a
canonical value.

Every runtime SHALL define one form for each of these in its host's language, built from that
language's own data, and SHALL take and return values in that form with no encoding step, so that
a host which computed a value passes it as it holds it. Canonical JSON is an encoding of the same
values, for a wire or a store, and SHALL NOT be what a host has to produce. What a host reads
from canonical JSON into a runtime's form SHALL be a value that runtime takes, so a host MAY pass
what it read as it read it, and a value a host built SHALL be taken exactly as the same value
read from JSON is.

A form MAY spell a value in a way canonical JSON does not, where the host's language makes it the
natural spelling, as a JavaScript member set to `undefined` spells a field that is left out and a
Rust 64-bit integer spells an integer JSON needs a record for. Each such spelling SHALL have one
meaning, stated by the runtime that takes it, and one size, stated by *A call's host input has a
size*.

A runtime whose host can hold something that is not a canonical value SHALL refuse it where the
host passes it, with a diagnostic that says where it is, and SHALL NOT convert it: it SHALL NOT
read a date as text or a map as a record. A host's type SHALL join a form only as the spelling of
a kind of value NX has, by a change to that runtime's own requirements. The objects a runtime
made for its own use, which a host passes back as it received them, are the function values and
action handlers above and SHALL be accepted where that runtime accepts them.

#### Scenario: A computed value needs no encoding
- **WHEN** a JavaScript host builds `{ title: "Home", tags: ["a", "b"] }` as an object literal and
  passes it as props, and another passes the result of `JSON.parse` on the text of the same value
- **THEN** the two calls SHALL have the same result, the same input size and the same operation
  count
- **AND** neither host SHALL have had to write the value as JSON

#### Scenario: One value in two hosts' forms is one input
- **WHEN** the record `{ "$type": "Person", "name": "Ada", "tags": ["a"] }` is passed to the
  TypeScript runtime as a plain object and to the Rust runtime as an `NxValue::Record`
- **THEN** the two calls SHALL have the same result, the same input size and the same operation
  count

#### Scenario: What is not a canonical value is refused, not converted
- **WHEN** a JavaScript host passes a `Date` or a `Map` where a value is expected, at a site
  typed `string`, a record or `object`
- **THEN** the call SHALL fail with a diagnostic that says where the value is
- **AND** the runtime SHALL NOT have read the date as a string or the map as a record

## MODIFIED Requirements

### Requirement: A call's host input has a size
The input of one call of an evaluation API SHALL have a size that depends on the values the host
supplies alone, so that every runtime measures the same input alike. The size SHALL be the sum of
the sizes of those values, each measured as the cost model measures a value written for the host:
one for each value, a sequence, the empty value and a `null` included, and one more for every 64
UTF-16 code units, rounded down, of a string, of a record's type name and of each field name. The
values a host supplies are: each positional argument of a function; the arguments of a call by
name, measured together as one record; the function record such a call names; the props of a
component, measured as one record; each content item; a state the host passes in, measured as one
record; each entry of a dispatched batch; and a state patch, measured as one record. A component
instance, and the parent instance a host names when it initializes a child, SHALL NOT be input:
the runtime produced them. A helper that takes no runtime options and evaluates no program code,
as the update helpers do, has no input in this sense. The size SHALL be measured before any value
is checked against a type, and SHALL NOT depend on the types the values reach.

Where a host can spell one input in more than one way, or the forms two runtimes are given differ,
the size SHALL be the same:

- An integer is one value, a 64-bit integer the Rust runtime is given included. The record
  canonical JSON spells an integer outside the safe range with,
  `{ "$type": "nx.int", "value": "<digits>" }`, is a record to every runtime where a host passes
  it, and is measured as one: the record, its string, and the string's length.
- The arguments of a call by name and the props of a component, when the host leaves them out,
  are measured as an empty record: their size is one. A state the host does not pass is not input.
- A member named `$type` that holds a string is the type name of the record or the map it is in,
  and counts its length alone. A `$type` member that holds anything else is a field like any other.
- An element of the list of positional arguments that is undefined, or a hole in that list, is
  the empty value: its size is one. A parameter the list does not reach is not input.
- A member of a plain object that is undefined, as a JavaScript host can write one, is not a
  member: neither its name nor a value is counted, so an object measures the same with the member
  set to undefined as with the member left out.
- A value that is not a canonical value is one value and is not read further: in JavaScript,
  anything but `null`, a boolean, a number, a string, an array or a plain object, and an object
  the runtime can tell it made for its own use. An object that only carries the marking of one,
  as a value read from JSON can, is a plain object and is measured as one. A runtime that can be
  given a value that is not canonical refuses it when it reads the input, as *A host value is a
  canonical value in the host's own form* requires, but for an object it made for its own use;
  the measure counts one for it first, since the size is taken before any value is read.

A runtime MAY offer the host a limit on the input size. A runtime that does SHALL refuse a call
whose input is larger than the limit before it evaluates anything, and SHALL stop measuring as
soon as the size passes the limit. For input that is plain data, the work of measuring SHALL then
be bounded by the limit and, at most once in a call, by the number of members of the one record at
which the limit is passed, whose names a runtime may have to list before it reads any of them. A
member that is undefined is listed and not counted, so the work is bounded by the number of those
as well; a value read from JSON has none. The
text a runtime reads in order to refuse a call SHALL be bounded by a fixed multiple of the limit:
a string, a type name or a field name far longer than the limit allows SHALL be refused from its
length, without its contents being read. The limit SHALL be separate from the operation budget: measuring
the input SHALL charge no operation, and a limit SHALL NOT change what an evaluation costs.

#### Scenario: A list is its items and itself
- **WHEN** a host passes a list of 1,000 integers as the one argument of a function
- **THEN** the input size of the call SHALL be 1,001

#### Scenario: Props are measured as one record
- **WHEN** a host initializes a component with the props `{ "title": "Home", "count": 3 }`
- **THEN** the input size of the call SHALL be three: the record and its two values

#### Scenario: Props left out are an empty record
- **WHEN** one host initializes a component with no props and another with the props `{}`
- **THEN** the input size of each call SHALL be one

#### Scenario: Text costs its length
- **WHEN** a host passes a string of 6,400 UTF-16 code units
- **THEN** its size SHALL be 101: one for the value and 100 for its length

#### Scenario: A name costs its length
- **WHEN** a host passes an object whose one field name is 1,048,576 UTF-16 code units long and whose value is a number
- **THEN** its size SHALL be 16,386: the object, 16,384 for the name, and the number

#### Scenario: The JSON form of a wide integer is measured as the record it is
- **WHEN** a host passes `{ "$type": "nx.int", "value": "9007199254740993" }`
- **THEN** its size SHALL be two in every runtime: the record and its string
- **AND** a list of two of them SHALL have the size five
- **AND** the same integer passed to the Rust runtime as a 64-bit integer SHALL have the size one

#### Scenario: Text in the form of a wide integer is counted
- **WHEN** a host passes `{ "$type": "nx.int", "value": <a string of 1,048,576 UTF-16 code units> }`
- **THEN** its size SHALL be 16,386 in every runtime

#### Scenario: An object that only looks like the runtime's own is measured
- **WHEN** a JavaScript host passes an object read from JSON that carries the marking the runtime gives an action handler and holds a string of 1,048,576 code units
- **THEN** its size SHALL be more than 16,384: it is a plain object, and what it holds is measured

#### Scenario: A type name among props is a type name
- **WHEN** a host passes the props `{ "$type": "Card", "title": "Home" }`
- **THEN** the input size SHALL be two: the record and the title

#### Scenario: An instance is not input
- **WHEN** a host dispatches a batch of one entry against an instance whose state holds 50,000 values
- **THEN** the input size of the call SHALL be the size of the one entry

#### Scenario: A limit costs no operations
- **WHEN** an evaluation costs `n` operations with no input limit
- **THEN** it SHALL cost `n` operations under any input limit its input fits

#### Scenario: A long string is refused without being read
- **WHEN** a host passes a string of 64 million code units under an input limit of 1,000
- **THEN** the call SHALL be refused for its input
- **AND** the runtime SHALL NOT have read the string's contents to refuse it

#### Scenario: An undefined member is not counted
- **WHEN** a JavaScript host measures `{ a: 1, b: undefined }` and `{ a: 1 }`
- **THEN** the two sizes SHALL be equal
- **AND** an object of one member and a thousand members that are undefined SHALL be within a
  limit that the object of the one member is within
