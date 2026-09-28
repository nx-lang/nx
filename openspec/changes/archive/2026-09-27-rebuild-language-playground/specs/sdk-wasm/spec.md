## ADDED Requirements

### Requirement: Program artifacts evaluate root to annotated NX text
A program artifact built by the wasm SDK SHALL evaluate its entry module's `root` and return an
`NxValueText`: the value as NX text, identical to what `nxlang run` prints for the same source, and
a list of nodes annotating that text. The one difference from `nxlang run` SHALL be how deep calls
may nest: at most 200, where the command line allows 1,000, because a browser runs the module on a
native stack the module cannot size. A program that nests deeper fails with the interpreter's
recursion-limit error. Each node SHALL give the UTF-16 range of one value, property or
sequence in the text, its enclosing node, its role, its type spelled in NX, and, when the entry
module declares it, the span of its declaration. An artifact that cannot be evaluated SHALL fail
with an `NxEvaluationError` carrying diagnostics, and the artifact SHALL remain usable afterwards.

#### Scenario: A record evaluates to its NX spelling
- **WHEN** a host builds `type User = { id:string name:string }` followed by
  `<User id="1" name="Ada" />` and calls `evaluateNx()` on the artifact
- **THEN** the result's `text` SHALL be `<User id="1" name="Ada" />`

#### Scenario: Nodes annotate the text
- **WHEN** a host evaluates that same source
- **THEN** the result SHALL hold a `record` node of type `User` covering the whole text, whose
  declaration is the span of `type User`
- **AND** it SHALL hold, as that node's children, `property` nodes named `id` and `name` of type
  `string`, each covering its `name="…"` text and declared at its property

#### Scenario: A sequence carries its count
- **WHEN** `root` returns a sequence of three records of type `User`
- **THEN** the result SHALL hold a `sequence` node of type `User*` covering the whole text with a
  count of 3, and one `record` node per item as its children

#### Scenario: A property's type is its declared type
- **WHEN** a record's type declares `subtitle?:string` and the value omits it or supplies it
- **THEN** any `property` node for `subtitle` SHALL have the type `string` and be marked optional

#### Scenario: Offsets count UTF-16 code units
- **WHEN** a string before a node holds a character outside the Basic Multilingual Plane
- **THEN** the node's offsets SHALL count that character as two units, so that they index the
  JavaScript string `text` directly

#### Scenario: No root
- **WHEN** a host calls `evaluateNx()` on an artifact whose entry module has no `root`
- **THEN** it SHALL throw an `NxEvaluationError` whose diagnostic says there is no `root`

#### Scenario: A runtime error carries its span
- **WHEN** evaluating `root` fails at run time
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` whose diagnostic carries the message and
  the span in the entry module

#### Scenario: Recursion past the module's limit but within the command line's
- **WHEN** `root` calls a function that recurses 250 deep and then returns
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` saying the recursion depth of 200 was
  exceeded, where `nxlang run` prints the value

#### Scenario: Runaway recursion is a diagnostic, not a trap
- **WHEN** `root` recurses without end through a plain function
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` for the interpreter's recursion limit
- **AND** the host SHALL remain usable

#### Scenario: A value with no NX spelling
- **WHEN** `root` evaluates to a value holding an action handler, or a sequence nested directly in a
  sequence
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` saying which part has no spelling
- **AND** SHALL NOT return partial text

#### Scenario: Parity with the command line
- **WHEN** the wasm SDK's parity tests run
- **THEN** `evaluateNx()` SHALL return, for every source in their corpus that has a `root`, a `text`
  equal to what `nxlang run` prints for it (none of them nests calls past the module's limit)

#### Scenario: The ABI version moves with the new export
- **WHEN** the wrapper for this version loads a module built before this export existed
- **THEN** loading SHALL fail with the version mismatch error, naming both versions
