## MODIFIED Requirements

### Requirement: The output pane shows the value as NX text
The playground SHALL evaluate the `root` of the visitor's source in the browser and show what it
returns in the output pane through the `<nx-value>` element of `value-view`. The text SHALL be
identical to what `nxlang run` prints for the same source, for any program whose calls nest no
deeper than the wasm SDK's limit of 200; one that nests deeper shows the recursion-limit error
instead, as the sdk-wasm capability specifies. The pane SHALL update after the visitor pauses
typing, not on every keystroke.

#### Scenario: A record
- **WHEN** the source is `type User = { id:string name:string }` followed by
  `<User id="1" name="Ada" />`
- **THEN** the output pane SHALL show `<User id="1" name="Ada" />`

#### Scenario: A sequence
- **WHEN** `root` evaluates to a sequence of three integers
- **THEN** the output pane SHALL show the three values, one per line, as `nxlang run` does

#### Scenario: The empty value
- **WHEN** `root` evaluates to the empty value
- **THEN** the output pane SHALL show `{}`

#### Scenario: A constant union case
- **WHEN** `root` evaluates to the case `active` of the union `Status`
- **THEN** the output pane SHALL show `Status.active`

#### Scenario: No root
- **WHEN** the source compiles but declares no `root` and ends in no element
- **THEN** the output pane SHALL say that there is no `root` to evaluate, and how to add one

#### Scenario: A runtime error
- **WHEN** evaluating `root` fails at run time
- **THEN** the output pane SHALL show the error's message, and the source pane SHALL mark its span
  when it has one

#### Scenario: A value with no NX spelling
- **WHEN** `root` evaluates to a value NX text cannot spell, such as one holding an action handler
- **THEN** the output pane SHALL say which part of the value has no spelling, rather than show
  partial text

#### Scenario: A very large value
- **WHEN** the NX text of `root`'s value is longer than 100,000 characters
- **THEN** the output pane SHALL show the first 100,000 characters followed by a notice that the rest
  was cut, and the page SHALL stay responsive

#### Scenario: Runaway recursion is a runtime error
- **WHEN** `root` recurses without end, as `let f(n:int): int = { f(n + 1) }` does from `f(0)`
- **THEN** the output pane SHALL show the runtime's recursion limit as a runtime error, with its
  span marked in the source pane, and the compiler SHALL go on answering without being replaced

#### Scenario: Last good output survives a broken edit
- **WHEN** an edit makes the source fail to compile
- **THEN** the output pane SHALL keep the last value it showed, marked as out of date
- **AND** the failure SHALL be reported as diagnostics in the source pane

#### Scenario: Updates are not issued per keystroke
- **WHEN** a visitor types continuously
- **THEN** the playground SHALL coalesce the edits and evaluate once the visitor pauses

#### Scenario: Hover in the output explains with the source's own hover
- **WHEN** the source declares `type Task = { title:string done:boolean = false }`, `root`
  returns a `Task`, and the visitor hovers `Task` in the output pane
- **THEN** the output pane SHALL show the same hover the source pane shows for the declaration of
  `Task`

#### Scenario: Hover in stale output falls back
- **WHEN** the output is marked out of date and the visitor hovers a node in it
- **THEN** the output pane SHALL show the element's default hover for that node, since the source's
  spans no longer match the value

#### Scenario: From the output to the declaration
- **WHEN** the visitor clicks a type name, property or case in the output whose declaration is in the
  source
- **THEN** the source pane SHALL select that declaration and scroll it into view

#### Scenario: A long value folds
- **WHEN** `root` returns a sequence of records that each span several lines
- **THEN** the output pane SHALL offer to fold each record, and each nested record, property and
  sequence that starts its line, as `value-view` specifies; the value as a whole does not fold
