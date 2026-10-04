## ADDED Requirements

### Requirement: NX IR evaluation cost is counted in operations
The cost of evaluating NX IR SHALL be counted in operations, and the count SHALL be a function of
the linked images and the values the host supplies alone: two runtimes that evaluate the same
images with the same input and produce the same result SHALL count the same number of operations,
in the same order. The format's documentation SHALL state the rules, which are:

- Evaluating a node costs one operation, charged before any of the node's children is evaluated.
  A node is charged every time it is evaluated: a loop body once per iteration, a parameter's
  default each time a call leaves the parameter out, a field's default each time a construction
  leaves the field out, a state field's default each time it is evaluated, a value's initializer
  each time a `reference` node evaluates it, and an action handler's body each time a dispatch
  runs it.
- Testing one pattern of an `ifIs` arm costs one operation for the pattern's node, whether the
  runtime evaluates that node or, for a pattern that names a union case with fields, reads the
  case it names. A pattern that is not tested because an earlier pattern of its arm matched costs
  nothing.
- The callee of a `call` or `namedCall` costs one operation for its node, whether the runtime
  evaluates that node or, for a `reference` to a function, resolves the function in place.
- Placing one item in a sequence a node builds costs one operation, charged as the item is placed,
  after the child that produced it has been evaluated. The sequences a node builds are the value of
  an `array` node, the value of a `for` or `forRange` node, and the content list of a `record`,
  `unionCase`, `element` or `component` node. A child whose value is itself a sequence places each
  of its items; a child whose value is empty places none. A `for` whose iterable is one item
  rather than a sequence yields its body's value as it stands, builds no sequence and so places
  nothing. A call that binds a sequence to a content parameter builds the list anew, an item that
  is itself a sequence contributing its items, before the list is checked against the parameter's
  type: each item of the argument costs one, or, when it is itself a sequence of several items,
  one for each item it contributes. An item that is empty contributes nothing and costs one.
- Producing the string of a `binary` node with the `concat` operator costs one operation for every
  64 UTF-16 code units of that string, rounded down, charged after both operands have been
  evaluated and before the string is produced.
- Checking a value against a declared type costs one operation for each value checked against a
  type that is not a sequence type, charged before that value is checked. A sequence type costs
  nothing itself: each item is checked against the item type, and no items cost nothing. A
  sequence of one item that reaches a type that is not a sequence type is read as its item, and
  costs the one check of that item. A record, or a union case with fields, costs one and then the
  check of each field value against the field's type, a default the check fills in included,
  after the nodes of the default have been charged as any nodes are; a field that is left out and
  has no default is not checked. A value at the type `object` is one value whatever it holds. The
  checks are the ones evaluation makes: an argument or a default against its parameter, a
  function's result against its declared result type, a value declaration's value against its
  declared type, each field of a `record`, `unionCase` or `component` construction against the
  field's type, and every value the host supplies to a typed site.
- Comparing two values costs one operation for the pair, charged before they are compared, for
  the `eq` and `ne` operators, for a pattern matched by equality, and for each field the `diff`
  intrinsic compares, a field one record does not hold comparing as the empty value. A pattern is
  matched by equality unless the pattern or the scrutinee is the empty value, or both are records
  and the pattern names a type; those are decided by emptiness or by the type name and compare
  nothing. An integer outside JavaScript's safe range, in a runtime that holds one, is an integer
  for this rule as any other is: a pattern that is one is matched by equality, and the comparison
  is paid for. Beyond the pair:
  - two sequences of one length compare their items pair by pair, in order, up to and including
    the first pair that differs; the pairs after it are not compared and cost nothing;
  - a sequence of one item and a value that is not a sequence compare the item with the value;
  - two records cost one more for every 64 UTF-16 code units of the shorter of their type names
    when both have one, and two records of different types compare nothing further. Two records
    of one type are lined up by field name: every field name either of them holds costs one more
    for every 64 code units, for each record that holds it; a name both hold compares the pair of
    its values; and a name only one holds costs one operation. A record holds a field when it
    stores a value for it: an optional field that was left out or is empty is not held, and a
    field an update record clears is;
  - two action handlers made by one node that captured as many slots compare their captured
    values slot by slot, in order, up to and including the first slot that differs. A handler
    captures the slots of its frame as they stood when it was made, and a slot that was not bound
    then compares only with another that was not;
  - two strings cost one more for every 64 UTF-16 code units of the shorter, rounded down.

  The items of a sequence and the slots of a frame are in one order in every runtime, so a
  comparison that stops at the first difference among them costs the same everywhere. The fields
  of a record are not: every pair of field values two records line up is compared, and every name
  counted, whether or not an earlier one already differed, so that the cost of a comparison
  depends on the two values and not on the order a runtime holds a record's fields in. The result of a comparison SHALL NOT depend on whether a
  budget is set, nor on whether the two values are held as one.
- Writing a value for the host costs one operation for each value written, charged before it is
  written, because every value takes a place in what is written: a sequence is one, the empty
  value included, and then its items; a record is one, and one more for every 64 UTF-16 code
  units of its type name and of each field name, and then its field values; a function value and
  an action handler are one each; and a string is one and one more for every 64 UTF-16 code
  units, rounded down. Only a value the runtime itself made is a function value or an action
  handler: a record a host passes that has the type name or the internal marking of one is a
  record, entered and paid for like any other. So is the record canonical JSON spells an integer
  outside JavaScript's safe range with: at `object`, where no type says what it is, no runtime
  takes it for a number. A `null` inside a value the host passed at the type `object` is the empty
  value: one value where it is written, and one item where a list that holds it is bound to a
  content parameter. Every value an evaluation returns is written: a function's
  result, a descriptor, rendered output, each effect, and component state. A value held once and
  reached from several places is written once for each place.

A node that is not evaluated SHALL cost nothing: the branch a conditional does not take, the right
operand of an `and`, `or` or `coalesce` that the left operand decides, and the body of an action
handler when the handler is created. Work a runtime does that is none of the above — preparing and
linking images, reading the host's input before it is checked, an update intrinsic's own work
apart from the comparisons of `diff`, collecting a dispatch's effects, and assigning handler
tokens — SHALL cost nothing, and SHALL NOT
grow with the size of a value, as it is held or as it unfolds when walked as a tree: a runtime
that walks a value for a purpose of its own SHALL visit a value reached from several places once,
and SHALL NOT walk again what an evaluation did not change.

A runtime MAY offer the host a budget of operations. A runtime that does SHALL charge by these
rules and SHALL fail an evaluation at the first charge that would take the count above the budget,
without performing the operation charged. The failure SHALL name the declaration the charge
belongs to: for a node, an item placed, a `concat` or a comparison, the declaration the node
belongs to, with the node's span when the image carries its debug section; for a check against a
type, the declaration whose parameter, result, field or value the check is for, which for a field
of an update record is the update record; for an item bound to a content parameter, the function
called; and for a value written for the host, the function or component the call evaluated. The
last three carry no span.

#### Scenario: A straight-line function pays for its arguments, its nodes and its result
- **WHEN** an image of `let add(a:int, b:int) = { a + b }` is evaluated at entrypoint `add` with `1` and `2`
- **THEN** the evaluation SHALL cost six operations: two for the arguments checked against `int`,
  three for the `binary` node and its two `slot` operands, and one for the number written

#### Scenario: A loop pays for its body and for its items
- **WHEN** an image of `let root() = { for i in 0..4 { i * i } }` is evaluated at entrypoint `root`
- **THEN** the evaluation SHALL cost one operation for the `forRange` node, seven for evaluating
  its iterable once (the `record` node of the `Range`, its three literals and the three fields
  checked), three for each of the four evaluations of the body, four for the four items placed in
  the result, and five for the list and its four numbers written: 29 in all

#### Scenario: An untaken branch costs nothing
- **WHEN** an image of `let pick(flag:boolean) = { if flag { 1 } else { for i in 0..1000 { i } } }` is evaluated with `true`
- **THEN** the evaluation SHALL cost eight operations: the argument checked, the `if` node, the
  `slot` node, the `array` node the then-branch `{ 1 }` is emitted as, its literal, the one item
  placed, and the one-item list and its number written

#### Scenario: A call pays for its callee however it is resolved
- **WHEN** an image of `let one() = { 1 }` and `let root() = { one() }` is evaluated at entrypoint `root`
- **THEN** the evaluation SHALL cost four operations: the `call` node, its `reference` callee, the
  literal that is the body of `one`, and the number written

#### Scenario: A spliced list pays for every item it places
- **WHEN** a function takes `xs:int+` and its body is the list `{ xs xs }`
- **AND** it is evaluated with a list of 500 integers
- **THEN** the evaluation SHALL cost 500 operations for the items of the argument checked, one for
  the `array` node, two for the two `slot` nodes, one thousand for the items placed, and 1,001
  for the list and its items written: 2,504 in all

#### Scenario: A long concatenation pays for its length
- **WHEN** a function concatenates two strings of 1,000 UTF-16 code units each
- **THEN** the `concat` SHALL cost one operation for its node and 31 for the 2,000 code units of
  its result, besides the cost of its operands
- **AND** writing the result for the host SHALL cost 32: one for the string and 31 for its length

#### Scenario: A host value is checked and written value by value
- **WHEN** a host passes a list of 10,000 integers to a function `let same(xs:int+) = { xs }`
- **THEN** the evaluation SHALL cost 20,002 operations: 10,000 for the items checked against
  `int`, one for the `slot` node, and 10,001 for the list and its items written

#### Scenario: A check against a type visits every value of a record
- **WHEN** an image of `type Point = { x:int y:int }` and `let sum(p:Point): int = { p.x + p.y }` evaluates `sum(<Point x={1} y={2} />)`
- **THEN** building the record SHALL cost two operations for its two fields checked, besides its nodes
- **AND** passing it to `sum` SHALL cost three: one for the record and one for each field
- **AND** the result of `sum` SHALL cost one, checked against `int`

#### Scenario: A comparison costs every pair it lines up
- **WHEN** an image evaluates `<Point x={1} y={2} /> == <Point x={3} y={2} />`
- **THEN** the comparison SHALL cost three operations: one for the two records and one for each
  pair of fields, the second although the first already differs

#### Scenario: Two lists are compared up to the first pair that differs
- **WHEN** two lists of 100,000 integers that differ in their first item are compared
- **THEN** the comparison SHALL cost two operations: the pair of lists and the first pair of items
- **AND** two equal lists of three integers SHALL cost four, and two lists of different lengths one

#### Scenario: Records inside a list are compared to the end
- **WHEN** a list whose first item is `{ "a": 1, "b": 2 }` is compared with one whose first item is `{ "a": 9, "b": 9 }`
- **THEN** the comparison SHALL cost four operations: the pair of lists, the pair of records, and both pairs of fields, and no later item SHALL be compared

#### Scenario: A comparison of strings costs their length
- **WHEN** an image compares two strings of 1,000 UTF-16 code units each
- **THEN** the comparison SHALL cost 16 operations: one for the pair and 15 for the code units
- **AND** comparing one of them with a string of 63 code units SHALL cost one

#### Scenario: A value held twice is written twice
- **WHEN** a function takes `p:Point` and returns the list `{ p p }`
- **THEN** writing its result for the host SHALL cost seven operations: one for the list and three for each of the two records

#### Scenario: A list bound to a content parameter is placed again at every call
- **WHEN** a function `let ignore(content c:object): int = { 1 }` is called with a list of 100 integers the host passed at `object`
- **THEN** the call SHALL cost 100 operations for the items bound, besides its nodes and the two checks of the list and the result
- **AND** 16,000 such calls with a list of 20,000 SHALL be refused by a budget of one hundred thousand, naming `ignore` and carrying no span

#### Scenario: A one-item sequence at a single-value type is one check
- **WHEN** a function `let f(x:int) = { x }` is called with the one-item list `{ 5 }`
- **THEN** checking the argument SHALL cost one operation, not two

#### Scenario: A field left out with no default is not checked
- **WHEN** an image of `type Opt = { a:int = 1 b?:int c?:string }` evaluates `<Opt c="q" />`
- **THEN** building the record SHALL cost two operations for fields checked, the default of `a` and `c`, and none for `b`

#### Scenario: A pattern decided by emptiness or by type compares nothing
- **WHEN** an `ifIs` tests the empty pattern against the number 4, or a pattern that names a union case with fields against a record
- **THEN** testing the pattern SHALL cost one operation for its node and none for a comparison

#### Scenario: Two records compare the fields they hold
- **WHEN** an image evaluates `<Opt c="q" /> == <Opt c="q" />`
- **THEN** the comparison SHALL cost three operations: the pair of records and the pairs of `a` and of `c`, and none for `b`, which neither holds

#### Scenario: A name only one record holds costs one
- **WHEN** a host passes `{ "a": 1, "b": 2 }` and `{ "a": 1, "c": 2 }` at `object` and they are compared
- **THEN** the comparison SHALL cost four operations: the pair of records, the pair of values under `a`, and one for each of `b` and `c`
- **AND** comparing `{ "a": 1 }` with `{ "a": 1, "b": 2 }`, in either order, SHALL cost three

#### Scenario: A wide record is not compared for the price of one operation
- **WHEN** a host passes an object of 8,000 fields at `object` and it is compared with `{ "x": 1 }`, in either order
- **THEN** the comparison SHALL cost more than 8,000 operations
- **AND** 16,000 such comparisons SHALL be refused by a budget of one hundred thousand

#### Scenario: Records of different types compare nothing further
- **WHEN** a record of type `A` with two fields is compared with a record of type `B`
- **THEN** the comparison SHALL cost one operation

#### Scenario: Every item bound to a content parameter costs one
- **WHEN** a host passes the list `[null]` at `object` and it is bound to a content parameter
- **THEN** binding it SHALL cost one operation, `[1, null]` two, `[[null, null], 1]` three, and `[[], [], [1, 2, 3]]` five

#### Scenario: Two handlers compare what they captured
- **WHEN** two action handlers made by one node in a frame with two bound slots are compared
- **THEN** the comparison SHALL cost three operations when they captured equal values: the pair of handlers and each pair of captured values
- **AND** SHALL cost two when the values in their first slot differ, whether or not those in the second differ too

#### Scenario: A comparison gives one result with or without a budget
- **WHEN** a record that holds a NaN is compared with itself
- **THEN** the result SHALL be `false` under any budget and under none, in every runtime

#### Scenario: A name the host supplied costs its length
- **WHEN** a host passes, at `object`, an object whose one key is 1,048,576 UTF-16 code units long, and a function returns it 20 times in a list
- **THEN** writing the result SHALL cost one operation for the list and 20 times 16,386: one for each object, 16,384 for its key, and one for its value

#### Scenario: Text a host puts in a form the runtime gives a meaning to costs its length
- **WHEN** a host passes, at `object`, `{ "$type": "nx.int", "value": <a string of 1,048,576 UTF-16 code units> }`, or an object with the type name or the internal marking of an action handler or a function value that holds such a string, and a function returns it 20 times in a list
- **THEN** writing the result SHALL cost more than 20 times 16,384 operations in every runtime
- **AND** comparing the value with itself SHALL cost at least 16,384

#### Scenario: The JSON form of a wide integer is a record at object
- **WHEN** a host passes `{ "$type": "nx.int", "value": "1152921504606846976" }` at `object`
- **THEN** in every runtime it SHALL be written back as that record, for two operations: the record and its string
- **AND** a record of that type name that holds 8,000 further fields SHALL be paid for by its fields where it is written, and compared with a value of another type for one pair
- **AND** in a runtime that holds 64-bit integers, it SHALL NOT be equal to the program's integer 1152921504606846976 and SHALL NOT match it as a pattern, the comparison costing one pair, while that integer passed in the runtime's own form SHALL be equal to it

#### Scenario: A wide integer pattern is matched by equality
- **WHEN** a runtime that holds 64-bit integers matches a value of 1152921504606846977 against the pattern 1152921504606846976
- **THEN** the pattern SHALL NOT match
- **AND** testing it SHALL cost the pattern and one pair compared, as a pattern of 1 against a value of 2 does

#### Scenario: An empty value written is one value
- **WHEN** a host passes `{ "a": null, "b": [null, null], "c": [] }` to `let passObject(o:object): object = { o }`
- **THEN** the evaluation SHALL cost nine operations: the argument and the result checked, the `slot` node, and six values written, which are the record, the empty value under `a`, the list under `b` and its two empty items, and the empty list under `c`

#### Scenario: A value made of empty values costs its length each time it is written
- **WHEN** a host passes, at `object`, an object holding a list of 20,000 `null`s, and a function returns it 500 times in a list
- **THEN** each copy written SHALL cost 20,002 operations, and the evaluation SHALL be refused by a budget of one hundred thousand

#### Scenario: A value held and never walked costs nothing more
- **WHEN** a function builds an element that holds one value in two properties, forty levels deep, and returns a number
- **THEN** the evaluation SHALL cost what building it costs, a few hundred operations
- **AND** the same value compared with another like it, or returned to the host, SHALL be refused
  by a budget of one hundred thousand, since walked as a tree it is 2^40 values

### Requirement: The conformance corpus records operation counts
Every program of the conformance corpus SHALL record, beside its expected results, the operation
count of each entrypoint it names and, for each lifecycle it names, the operation count of
initialization and of each dispatched batch, where a batch's count covers every handler the batch
runs and the render that follows. The regeneration command that writes the expected results SHALL
write the counts in the same run. Every runtime that offers an operation budget SHALL check, for
each recorded count, that the evaluation succeeds with its recorded result under a budget equal to
the count and fails with the resource-limit diagnostic under a budget one less. The corpus SHALL
hold at least one entrypoint for each charging rule: an `ifIs` whose arm tests more than one
pattern, a call whose callee is a function `reference` and one through a function value, a
sequence built by splicing a list, a `for`, a `forRange`, a content list, a `concat` whose
result is at least 64 UTF-16 code units long, a record checked against its type as an argument, a
comparison of two records that differ in their first field, a comparison of two lists that differ
in their first item, a comparison of two handlers of one node whose captured values differ in the
first slot and in a later one, a comparison of two strings of at least 64 UTF-16 code units, a
result that holds one record in two places, a list bound to a content parameter, and a comparison
of a constructed record with one `apply` returned from a constructed update record. For each of those entrypoints the corpus SHALL
also record a budget below its count together with the declaration and the span of the diagnostic
that budget produces, and every runtime that offers an operation budget SHALL check that it fails
there.

#### Scenario: A recorded count is exact
- **WHEN** a runtime evaluates a corpus entrypoint under a budget equal to its recorded count
- **THEN** the evaluation SHALL yield the recorded result
- **AND** the same evaluation under a budget one less SHALL fail with `nx-ir-resource-limit`

#### Scenario: A lifecycle step has a count of its own
- **WHEN** a runtime dispatches a batch of a corpus lifecycle under a budget equal to the count
  recorded for that batch
- **THEN** the dispatch SHALL yield the recorded rendered output and effects
- **AND** the same dispatch under a budget one less SHALL fail with `nx-ir-resource-limit`

#### Scenario: A smaller budget stops at the recorded node
- **WHEN** a runtime evaluates a corpus entrypoint, from the image that carries its debug section,
  under a budget the corpus records a failure for
- **THEN** the diagnostic SHALL name the recorded declaration and carry the recorded span

#### Scenario: A change in a runtime's counting is caught
- **WHEN** a runtime is changed so that it charges a node twice, or does not charge an item it
  places in a sequence
- **THEN** that runtime's corpus test SHALL fail naming the program and the entrypoint

#### Scenario: Counts are regenerated with the results
- **WHEN** the corpus is regenerated after an intended change to the emitter
- **THEN** the operation counts SHALL be rewritten by the same command
- **AND** a count that changed SHALL appear in the diff as a changed number beside its entrypoint

## MODIFIED Requirements

### Requirement: NX IR preserves canonical NX value encoding rules
NX IR SHALL preserve enough type and value metadata for runtimes to produce canonical raw NX values.
A `+` or `*` value SHALL evaluate as an array, including the empty array for an empty `*` value; a
`?` value that holds an item SHALL evaluate as that item, and an empty `?` value SHALL be an
omitted key where it is a record field, as `occurrence-types` defines the canonical encoding.
Constant union cases SHALL evaluate as authored case strings, records and payload union cases SHALL
evaluate as object/map payloads with `$type` discriminators when their type requires one, and
numeric values that cannot safely round-trip through JavaScript numbers SHALL use a lossless tagged
representation. A runtime that cannot hold such a number SHALL refuse it explicitly where it reads
it, and SHALL NOT take a value a host passes for one because of its shape.

#### Scenario: Enum output remains a bare string
- **WHEN** NX source evaluates `Theme.dark` where `Theme` is a constant union
- **AND** the value is produced through an NX IR runtime
- **THEN** the canonical output value SHALL be the bare string `"dark"`
- **AND** the output SHALL NOT wrap the value in an object

#### Scenario: Union case output includes discriminator
- **WHEN** NX source evaluates `LoadState.failed { message: "offline" }`
- **AND** the value is produced through an NX IR runtime
- **THEN** the canonical output value SHALL include `$type` with value `LoadState.failed`
- **AND** it SHALL include the declared `message` field

#### Scenario: An empty optional field is an omitted key and an empty sequence is an empty array
- **WHEN** NX source contains `type Book = { title:string author?:string tags?:string+ } let b() = <Book title="B" />` and `let t(b:Book): string* = { b.tags }`
- **AND** the values of `b()` and `t(b())` are produced through an NX IR runtime
- **THEN** the canonical output of `b()` SHALL contain `title` and neither an `author` nor a `tags` key, since an empty optional field is not stored whatever its occurrence
- **AND** the canonical output of `t(b())` SHALL be the empty array

#### Scenario: Large integer literal is lossless
- **WHEN** NX source contains an integer literal that cannot be represented exactly as a JavaScript
  number
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL encode that literal with enough information for a runtime to preserve the
  exact integer value, or to refuse the integer explicitly when it cannot hold it

