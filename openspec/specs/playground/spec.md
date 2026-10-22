# playground Specification

## Purpose

The public NX Playground at `https://nxlang.org/play`: a browser page for trying the NX language. A
visitor writes NX source on one side and sees the value its `root` evaluates to on the other, shown
by the `value-view` element as highlighted NX text. Short examples grouped by topic get a visitor
started, and the source travels in the page's address so it can be shared. Everything runs in the
browser; the DrawnUI fiddle is where NX draws interfaces.
## Requirements
### Requirement: Compilation and language queries run in the browser behind the same seam
The site SHALL compile and evaluate NX and answer hover, completion, diagnostics and symbol queries
in the visitor's browser, off the main thread, and SHALL keep the choice of where that work happens
confined to the single compile seam and the language service interface, so editing and output do
not change when the implementation does.

#### Scenario: The browser holds the compiler
- **WHEN** the site is loaded and the visitor edits
- **THEN** diagnostics, the evaluated value and language answers SHALL be produced without any
  request leaving the browser

#### Scenario: The seam is uniform
- **WHEN** evaluation is requested
- **THEN** it SHALL be requested through one interface that takes NX source and yields diagnostics
  and either the value as annotated NX text or the reason there is none
- **AND** editing and the output pane SHALL depend on that interface rather than on how it is
  fulfilled

#### Scenario: The editor stays responsive during a compile
- **WHEN** a compile, evaluation or language query is in progress
- **THEN** the source pane SHALL keep accepting input and painting

#### Scenario: The same compiler is tested and shipped
- **WHEN** the example check runs
- **THEN** it SHALL compile and evaluate every example through the same in-browser compiler package
  the site ships

### Requirement: A failed compiler costs one request
A compile, evaluation or language call that traps or overruns its deadline SHALL cost that request
only: the site SHALL report it as a failure, replace the compiler, and answer the next request
normally.

#### Scenario: A trap is reported and recovered from
- **WHEN** the compiler traps while handling a request
- **THEN** that request SHALL be reported as a failure the visitor can read
- **AND** the next compile SHALL succeed without reloading the page

#### Scenario: An overrunning request is cut off
- **WHEN** a request has not answered within the site's deadline
- **THEN** it SHALL be reported as a failure
- **AND** the compiler that was running it SHALL be discarded rather than waited on

#### Scenario: Recursion too deep for the browser is named
- **WHEN** evaluating traps because the browser's stack ran out, as recursion with calls heavier
  than the module's recursion limit allows for can make it
- **THEN** the site SHALL say that the program recursed deeper than the browser can run, replace
  the compiler, and not retry the request

#### Scenario: A compiler that has not loaded yet is waited for
- **WHEN** a request is made while the compiler is still being loaded
- **THEN** the deadline SHALL measure the compiler's own work rather than the wait for the load
- **AND** the load SHALL be waited for rather than discarded and started again

#### Scenario: A load that never finishes is reported
- **WHEN** the compiler has not arrived within the site's own budget for loading it
- **THEN** the site SHALL report a failure the visitor can read rather than wait on it indefinitely

#### Scenario: A recovered failure draws without an edit
- **WHEN** an evaluation fails in a way a replaced compiler can answer, on source the visitor has not
  edited since
- **THEN** the site SHALL evaluate again of its own accord and show the output, rather than leave
  the failure standing

### Requirement: Public deployment is fronted by an edge that serves assets
The playground SHALL be served from Cloudflare's edge as static files, over TLS, with no origin
process behind it. No rate limit SHALL be required, since no request costs more than a file.

#### Scenario: The site is reachable at its public address
- **WHEN** a visitor opens `https://nxlang.org/play`
- **THEN** the playground SHALL be served over TLS
- **AND** `http://nxlang.org/play` SHALL redirect to it

#### Scenario: Assets are served from the edge
- **WHEN** a hashed asset or the NX compiler module is requested
- **THEN** it SHALL be served from Cloudflare's edge, with no request reaching an origin server

### Requirement: Static files declare their cache policy
The site's static files SHALL declare cache headers that let browsers hold content-addressed assets
for a long time and never hold a stale shell, so that a deploy is visible on the next page load.

#### Scenario: Hashed assets are cacheable for a long time
- **WHEN** a build-output asset whose file name carries a content hash is served, the NX compiler
  module included
- **THEN** the response SHALL declare itself publicly cacheable, immutable, and valid for at least a
  year

#### Scenario: Static files without a hash are revalidated
- **WHEN** a font, image or other static file whose name carries no content hash is served
- **THEN** the response SHALL be cacheable but SHALL require revalidation within a day

#### Scenario: The shell is not cached
- **WHEN** the shell document is served, for any address that resolves to it
- **THEN** the response SHALL require revalidation on every use

### Requirement: Site deploys as static files from main
The site SHALL be built to static files from a clean checkout, the NX compiler module included, and
a push to `main` that touches the site or a package it depends on SHALL build, test and deploy it.
The deployment's configuration, meaning the route it serves, its client routing and its headers,
SHALL be committed in the repository.

#### Scenario: Build produces its own native dependencies
- **WHEN** the site is built from a clean checkout in CI
- **THEN** the build SHALL produce the NX compiler module the client loads
- **AND** it SHALL NOT depend on artifacts built outside that run

#### Scenario: Deploys follow the main branch
- **WHEN** a commit that touches the site or a package it depends on lands on `main`, and its build
  and tests pass
- **THEN** the new build SHALL replace the live one

#### Scenario: A failing build does not deploy
- **WHEN** the build or tests fail
- **THEN** the live site SHALL keep serving the previous build

#### Scenario: The deploy is verified
- **WHEN** a deployment finishes
- **THEN** the workflow SHALL fetch the shell and the compiler module from
  `https://nxlang.org/play`, and fail if either does not answer

### Requirement: Addresses under the playground prefix
Everything the playground serves SHALL live under `/play`. The playground SHALL always open
on source in the editor view: an example chosen by its address, source carried in the address's
fragment, or the default example.

#### Scenario: The prefix opens the default example
- **WHEN** a visitor opens `/play` or `/play/` with no fragment
- **THEN** the editor view SHALL open with the default example loaded and its output shown

#### Scenario: An example's address
- **WHEN** a visitor opens `/play/<id>` for an example's id
- **THEN** the editor view SHALL open with that example loaded and selected in the examples
  drop-down

#### Scenario: An unknown example
- **WHEN** a visitor opens `/play/<id>` and no example has that id
- **THEN** the playground SHALL open the default example and say that the requested example was not
  found, rather than show an error page

#### Scenario: The fragment wins
- **WHEN** a visitor opens an address under the prefix carrying a `#code=` fragment
- **THEN** the source pane SHALL hold the source the fragment encodes, whatever the path names

#### Scenario: Assets live under the prefix
- **WHEN** the shell loads its scripts, styles and the NX compiler module
- **THEN** every one of those requests SHALL be for a path under `/play/`

#### Scenario: No API is served
- **WHEN** a request names a path under `/play/api/`
- **THEN** the playground SHALL answer not found

#### Scenario: A missing asset is not the shell
- **WHEN** a request under `/play/assets/` names a file the build did not produce
- **THEN** the playground SHALL answer not found rather than serving the shell

### Requirement: The playground carries the site header and names itself
The playground SHALL show the same header links as the website (NX, Docs, Playground, GitHub), with
Playground marked current, and SHALL present itself as the NX Playground: a place to try the
language. It SHALL point visitors who want to see NX draw interfaces to the DrawnUI fiddle.

#### Scenario: Header links
- **WHEN** a visitor opens the playground
- **THEN** the header SHALL link to the website's documentation, to `/play`, and to
  `https://github.com/nx-lang/nx`

#### Scenario: Document titles
- **WHEN** the source is exactly an example's, whether its address is the example's or a fragment
- **THEN** the browser tab title SHALL name the NX Playground and the example
- **AND** for any other source, it SHALL name the NX Playground alone

#### Scenario: The fiddle is pointed to
- **WHEN** a visitor opens the playground
- **THEN** its toolbar SHALL link to `https://fiddle.drawnui.net` as where NX draws interfaces

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
- **THEN** the output pane SHALL show the interpreter's recursion limit as a runtime error, with its
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

### Requirement: Source travels in the address
The playground SHALL encode the source pane's text in the address fragment as
`#code=<payload>`. The payload SHALL be the UTF-8 text, compressed with raw DEFLATE, in unpadded
base64url. The playground SHALL decode any address in that form, so that other sites, the website
among them, can build playground links without the playground's code.

#### Scenario: Share
- **WHEN** a visitor presses Share
- **THEN** the playground SHALL copy to the clipboard an absolute `https://nxlang.org/play#code=…`
  address for the current source, and confirm that it did

#### Scenario: A shared address opens the same source
- **WHEN** another visitor opens that address
- **THEN** their source pane SHALL hold exactly the shared text, and the output pane SHALL show its
  value

#### Scenario: Edits keep the address current
- **WHEN** a visitor edits the source and pauses
- **THEN** the address SHALL be replaced with one encoding the new source, without adding a history
  entry
- **AND** reloading the page SHALL restore that source

#### Scenario: An undecodable fragment
- **WHEN** the `#code=` payload is not valid base64url, does not inflate, or is not UTF-8
- **THEN** the playground SHALL open the default example and say that the link could not be read

#### Scenario: The encoding is fixed by a shared fixture
- **WHEN** the playground's tests run
- **THEN** they SHALL decode a committed fixture address to its committed source, the same fixture
  the website's encoder is tested against

### Requirement: Examples get a visitor started
The playground SHALL offer between 10 and 20 short examples, grouped by topic, that together cover
the language's main features, and MAY offer, under the topic "Larger examples", a few longer programs
of the kind a real app holds, such as a survey, a settings screen or an orders dashboard, for judging
how the Visual view reads a whole program, and, under the topic "Test cases", one example holding
one or two of every case the Visual view reads and edits, each with a comment saying what to try. The
larger examples and the test cases SHALL NOT count toward the 10 to 20 and need not fit without
scrolling. They SHALL be offered from a single drop-down in the toolbar and SHALL
NOT be featured anywhere else on the page, because their purpose is to show what NX to write rather
than to be browsed. Each short example SHALL be small enough to read without scrolling on a laptop,
and each SHALL link to the documentation page for its topic.

#### Scenario: Topics
- **WHEN** a visitor opens the examples drop-down
- **THEN** it SHALL offer examples, grouped under their topic, for at least: basic values, records
  and defaults, unions, occurrences (`?`, `+`, `*`), sequences, `if`, `for` with a range, functions
  and function values, components, and text with interpolation

#### Scenario: The examples stay in the background
- **WHEN** a visitor opens the playground
- **THEN** the examples SHALL be reachable only through the toolbar's drop-down, which SHALL name
  the current example, or say the source is edited once it no longer matches one

#### Scenario: Choosing an example
- **WHEN** a visitor chooses an example from the drop-down
- **THEN** the source pane SHALL load it, the output pane SHALL show its value, and the address SHALL
  become `/play/<id>` with no fragment

#### Scenario: Leaving edits behind
- **WHEN** a visitor has edited the source and chooses another example
- **THEN** the edited source SHALL remain reachable through the browser's Back button, since its
  address held it

#### Scenario: From an example to its docs
- **WHEN** an example is loaded
- **THEN** the toolbar SHALL show a small link to the documentation page for its topic

#### Scenario: Larger examples
- **WHEN** a visitor opens the examples drop-down
- **THEN** the larger examples SHALL be grouped under "Larger examples", and the playground's tests
  SHALL count toward the 10 to 20 only the examples under neither "Larger examples" nor "Test cases"

#### Scenario: Visual test cases
- **WHEN** a visitor opens the "Visual test cases" example and switches to Visual
- **THEN** it SHALL hold, with a comment for each saying what to try, a one-line element, a property
  sheet, an element as a property's value, a union case as an element, a group with no title to fold,
  an element whose visualizer merges with its base's and puts its id last, an icon Visual cannot draw,
  a function used as an element, text content around a braced value and over two lines, a doc on a
  type and on a property, and a value of every kind Visual edits: a string with escapes, `uitext`, an
  id formatted by a value visualizer, an `int`, an alias of one, a `float64`, an untyped number, a
  boolean written bare and in braces, a case written bare and qualified, a color formatted by a value
  visualizer and by a property visualizer, a value after an emoji and a sequence of numbers, together
  with a string Visual opens in Source, an expression, a `for` and an `if`, and properties that may
  and may not be left out

### Requirement: Every example is checked against its expected output
Every example SHALL have its expected output committed beside it. The playground's tests SHALL
compile and evaluate every example through the same compiler module the site ships, and SHALL fail
when an example reports any diagnostic or its output differs from the committed text.

#### Scenario: An example drifts
- **WHEN** a language change alters what an example evaluates to
- **THEN** the playground's tests SHALL fail naming the example and showing both texts

#### Scenario: Every example links to a page that exists
- **WHEN** the playground's tests run
- **THEN** each example's docs link SHALL name a page the website builds

### Requirement: Source pane offers NX language features
The source pane SHALL highlight NX with the grammar the repository publishes for editors, through the
shared Monaco integration, and SHALL offer hover and completion answered by the NX language service
over the visitor's source and the prelude.

#### Scenario: NX is syntax highlighted by the shared grammar
- **WHEN** the source pane contains NX
- **THEN** it SHALL be highlighted by the repository's published grammar through the shared Monaco
  integration, with no grammar, tokenizer or theme of the playground's own

#### Scenario: Hover on the visitor's own declarations
- **WHEN** a visitor hovers a name declared in their own source
- **THEN** the source pane SHALL show what the language service reports for it

#### Scenario: Completions
- **WHEN** a visitor requests completions inside the opening tag of an element-style function
- **THEN** the source pane SHALL offer its properties not yet supplied

#### Scenario: Language features degrade without breaking editing
- **WHEN** the compiler that answers language queries cannot be loaded or fails
- **THEN** hover and completion SHALL silently offer nothing
- **AND** the source pane SHALL remain editable and evaluation SHALL continue to work

### Requirement: Diagnostics are reported against the visitor's source
The playground SHALL report every diagnostic from compiling the visitor's source, positioned at the
line and column the visitor sees, including the warnings of source that compiles, alongside its
value or its runtime error.

#### Scenario: Diagnostics are shown in the source pane
- **WHEN** compilation reports a diagnostic with a source position
- **THEN** the source pane SHALL mark the reported span, and the message SHALL be readable by the
  visitor

#### Scenario: A position with no width is still a position
- **WHEN** a diagnostic names a point rather than a range, as a missing token's insertion point does
- **THEN** the source pane SHALL mark it visibly

#### Scenario: Compilation errors do not break the session
- **WHEN** compilation fails for any reason, including a compiler that crashed or did not load
- **THEN** the playground SHALL report the failure and remain editable

#### Scenario: A warning in source that compiles is shown
- **WHEN** the visitor's source compiles and evaluates, and compiling reported a warning
- **THEN** the source pane SHALL mark the warning's span as a warning while the output pane shows
  the value

### Requirement: The layout works on a phone
The playground SHALL be usable at a viewport 375 pixels wide: the panes SHALL stack with the source
above the output, the toolbar and its examples drop-down SHALL stay usable, and the page SHALL not
scroll horizontally.

#### Scenario: Narrow viewport
- **WHEN** a visitor opens the playground at 375 × 800
- **THEN** the source and output panes SHALL both be visible by vertical scrolling, and the page's
  width SHALL not exceed the viewport

### Requirement: Site documents how to run and deploy it
The playground SHALL carry documentation covering its prerequisites, the wasm toolchain among them,
how to run it locally under `/play`, how to add an example and its expected output, and where
its deployment is documented.

#### Scenario: Adding an example is documented
- **WHEN** a contributor reads the playground's README
- **THEN** it SHALL say where an example's source, expected output, topic and docs link go, and which
  command checks them

### Requirement: Source and output use readable themes
The playground SHALL color its source and output with GitHub's current light and dark themes,
`github-light-default` and `github-dark-default`, whose comment colors meet a 4.5:1 contrast ratio
against their backgrounds, and its panes SHALL use those themes' background and foreground colors.

#### Scenario: Comments are readable in the dark theme
- **WHEN** the playground is shown in its dark theme
- **THEN** a comment in the source SHALL be drawn with a contrast ratio of at least 4.5:1 against
  the editor background

### Requirement: Completion documentation is visible from the first completion
The source pane SHALL open the completion list's details, with the item's documentation, the first
time a completion list appears whose focused item has a detail or documentation to show. After
that the details SHALL be the visitor's to open and close, and SHALL stay as the visitor left them.

#### Scenario: The first completion shows its documentation
- **WHEN** the visitor requests completions for the first time at a property slot whose property is
  documented
- **THEN** the completion list SHALL show that documentation beside it without a further gesture

#### Scenario: Closed details stay closed
- **WHEN** the visitor closes the details and requests completions again
- **THEN** the details SHALL stay closed

### Requirement: The source pane reads as well as edits
The playground's source pane, titled "NX", SHALL offer a Visual and Source switch in its title,
Visual first and by default. Source SHALL show the editor, and Visual SHALL show the visitor's
source in the viewer's `<nx-viewer>` element, from the `sourceTree` query the playground's worker
answers, refreshed after the visitor pauses typing as evaluation is and marked stale while a newer
tree is computed. Visual SHALL be colored by the editor's themes, Dark+ and Light+, set in Geist for
prose and Geist Mono for code, and draw its icons from Lucide, loaded apart from the rest of the
playground when Visual first needs them. While Visual shows, the title SHALL also offer Collapse all
and Expand all, and what the reader folds SHALL be kept while they switch between Source and Visual
and forgotten when they choose an example. Switching to Visual SHALL select the node at the editor's
cursor, and switching to Source SHALL put the cursor at the start of the selected node and reveal
it. When the query fails, Visual SHALL say that the reading is unavailable, and Source SHALL keep
working.

#### Scenario: Reading an example
- **WHEN** a visitor opens the default example
- **THEN** the source pane SHALL show the example's reading in Visual, and the output pane SHALL be
  unchanged

#### Scenario: The selection crosses over
- **WHEN** a visitor selects an element in Visual and switches to Source
- **THEN** the editor's cursor SHALL be at the start of that element in the source

#### Scenario: Read on a phone
- **WHEN** a visitor opens the playground at a viewport 375 pixels wide
- **THEN** the reading SHALL fit the pane's width without the page scrolling horizontally

### Requirement: The output pane previews a component
The playground's output pane SHALL offer an Output and Preview switch in its title, Output by
default. Preview SHALL run one component of the visitor's source in a preview session of
`@nx-lang/previewer`, inside the playground's worker, and show the session's current output
through `<nx-value>`, spelled by the artifact's `formatValue`. When `root` returns an element of a
component the source declares, Preview SHALL run that component with that element's props.
Otherwise it SHALL run the first non-external component the source declares, with no props given.
A picker SHALL list the non-external components the source declares, in source order, and choosing
one SHALL start a new session for it. Output SHALL keep working as before, and switching between
the two SHALL keep the session.

#### Scenario: A component element as root
- **WHEN** the source declares `component <Counter start:int = 0 />` with a `count` state that
  starts at `start`, ends with `<Counter start=3 />`, and the visitor switches to Preview
- **THEN** the pane SHALL show what `Counter` renders with `start` 3, and its state SHALL hold a
  `count` of 3

#### Scenario: Another component
- **WHEN** the source declares `Counter` and `Stepper` and the visitor picks `Stepper`
- **THEN** a new session SHALL start for `Stepper` with no props given, and the pane SHALL show what
  it renders

#### Scenario: Props the component requires
- **WHEN** the component Preview runs requires a prop that nothing gives
- **THEN** the pane SHALL show the runtime's message, and say that ending the program with the
  component's element, with its props, previews it with them

#### Scenario: No component
- **WHEN** the source declares no component, or only external ones
- **THEN** Preview SHALL say that there is no component to preview, and how to declare one

#### Scenario: Source that does not compile
- **WHEN** the visitor switches to Preview while the source does not compile
- **THEN** Preview SHALL say that the program does not compile yet, as Output does

#### Scenario: Preview on a phone
- **WHEN** a visitor switches to Preview at a viewport 375 pixels wide
- **THEN** the preview, its timeline and its state SHALL fit the pane's width without the page
  scrolling horizontally

### Requirement: Handlers in the preview are buttons
In Preview, every `ActionHandler` record in the output SHALL carry a button named for the action it
handles, through `<nx-value>`'s `actions`. Pressing it SHALL dispatch the action to that handler by
its pointer, and the output, state and timeline SHALL show the new tick. An action with no fields
SHALL dispatch at once. An action with fields SHALL open a form with one input per field, built from
the action's input schema (`typeSchema` with `direction: "input"`): a text field for a string, a
number field for an integer or a `float32` or `float64`, a checkbox for a `boolean`, a list of cases
for a constant union, and a JSON field for any other type, each starting from the field's default
when it has one. A dispatch that fails SHALL add no tick and SHALL show the failure's message beside
the form or button.

#### Scenario: Pressing a button
- **WHEN** a visitor previews the counter, whose `Button` binds `onTapped=<State.Update count={count +
  1} />`, and presses "Tapped"
- **THEN** the output SHALL show the count as 4, and the timeline SHALL gain a step

#### Scenario: An action with fields
- **WHEN** the output holds a handler for `action Answered = { questionId:string value:int }` and
  the visitor presses its button
- **THEN** a form SHALL open with a text field for `questionId` and a number field for `value`, and
  submitting it SHALL dispatch `Answered` with those values

#### Scenario: A failed dispatch
- **WHEN** a handler's body fails at run time
- **THEN** the pane SHALL show the runtime's message, the timeline SHALL be unchanged, and the
  source pane SHALL mark the failure's span when it has one

### Requirement: The preview keeps a timeline
Preview SHALL show a timeline strip of the ticks on the session's current path, from the first
render to the current tick, each step named by what made it: the start, the action dispatched, new
props, or an edit. Choosing a step SHALL make it current without running anything. A step that has
more than one child SHALL show how many branches leave it and let the visitor choose which to
follow. Back SHALL go to the current step's parent, and Forward SHALL go to the child the visitor
last left, or to the newest child when they have left none. Dispatching from a step that is not the
last SHALL add a branch, keeping the steps that followed it.

#### Scenario: Going back
- **WHEN** a visitor has pressed "Tapped" twice and chooses the first step
- **THEN** the output SHALL show the count as 3 again and the state SHALL hold 3, and nothing SHALL
  have run

#### Scenario: A branch
- **WHEN** the visitor then presses a different button from that first step
- **THEN** the first step SHALL show two branches, and the visitor SHALL be able to return to the
  run with two taps

### Requirement: The preview shows state and what was emitted
Preview SHALL show the current tick's state through `<nx-value>`, one `name=value` line per state
field as the artifact's `formatValue` spells a component's state, and say so when the component
keeps no state. It SHALL list the actions the current tick emitted to its host, the effects no
handler of the component consumed, each spelled as NX text.

#### Scenario: State
- **WHEN** the counter's current tick holds a `count` of 4
- **THEN** the state panel SHALL show `count=4`

#### Scenario: Emitted actions
- **WHEN** a handler returns an action the component emits to its host
- **THEN** the panel SHALL list that action under what the tick emitted

### Requirement: Edits hot reload the preview
While Preview shows a session, each edit that compiles SHALL reload the session with the new
program, after the visitor pauses typing as evaluation does. When the new program accepts the
current state, the output SHALL update in place and the timeline SHALL gain an edit step. When it
does not, the run SHALL be replayed under the new program, and the pane SHALL say so, and where the
replay stopped when it did not complete. When the props no longer render, the pane SHALL keep the
last output, marked out of date, and say why. When an edit changes the props of the element `root`
returns, the session SHALL render the current state with the new props. An edit that does not
compile SHALL leave the session as it was, marked out of date.

#### Scenario: A label fixed
- **WHEN** a visitor previewing the counter changes its label text from `"Count: "` to `"Total: "`
- **THEN** the output SHALL read `Total: 3`, with the state unchanged

#### Scenario: A state field renamed
- **WHEN** the visitor renames the `count` state field to `total`
- **THEN** the pane SHALL say that the run was replayed, and the timeline SHALL show the replayed
  steps

#### Scenario: New props
- **WHEN** the visitor changes the final element to `<Counter start=10 />`
- **THEN** the timeline SHALL gain a props step, and the state SHALL be kept

### Requirement: The preview leads to the source that built it
In Preview, clicking a record's type name SHALL select, in the source pane, the element expression
that built that record, from the session's value origins, and scroll it into view. A record with no
origin SHALL lead to its declaration, as in Output. Hovering SHALL explain a node as it does in
Output.

#### Scenario: From a rendered label to its expression
- **WHEN** a visitor previewing the counter clicks `Label` in the output
- **THEN** the source pane SHALL select `<Label text={"Count: " + count} />`

### Requirement: The preview runs within limits
Every call the preview session makes SHALL run under the runtime's limits, with a call depth of at
most 200 as evaluation allows, so that a handler that runs without end stops with the runtime's
resource-limit message and the page stays responsive. The output shown SHALL be cut at 100,000
characters as Output's is.

#### Scenario: A runaway handler
- **WHEN** a handler calls a function that recurses without end and the visitor presses its button
- **THEN** the pane SHALL show the runtime's resource-limit message, the timeline SHALL be
  unchanged, and the editor SHALL keep responding

### Requirement: An example previews at once
The playground's examples SHALL include "A counter to click", under the Components topic: a
`Counter` component with a `count` state and buttons that add one and reset, whose program ends
with a `Counter` element, so that switching to Preview runs it with no further steps.

#### Scenario: Opening the example
- **WHEN** a visitor opens "A counter to click" and switches to Preview
- **THEN** the pane SHALL show the counter with buttons for adding and resetting, and pressing them
  SHALL change the count

### Requirement: The Visual view uses the program's visualizers
The playground's Visual view SHALL ask for the source tree with `visualizers`, naming no host visualizer declaration
modules, so that visualizer declarations in the visitor's source and in the standard libraries it imports shape the
reading. Visualizer diagnostics SHALL be reported against the visitor's source like any other diagnostic.

#### Scenario: A visualizer declaration in the visitor's source
- **WHEN** a visitor's source declares `type Choice = { value:string label:string }`, a visualizer declaration
  `visualizer Choice = <ElementVisualizer title=label id=value />`, and a value
  `let pick = <Choice value="a" label="Apples" />`, and the visitor switches to Visual
- **THEN** the reading of `pick` SHALL show `Apples` as its title and `a` as its id, without quotes

#### Scenario: A misspelled visualizer declaration
- **WHEN** the visitor writes `visualizer Choice = <ElementVisualizer title=lable />`
- **THEN** the source pane SHALL show a diagnostic on `lable`

### Requirement: Visual edits the visitor's source through the editor
The playground SHALL set the viewer's `editable` in Visual. For each `nx-edit`, it SHALL ask its
worker's language service for the `sourceEdit` of the visitor's source and apply the answer's edits
to the editor's model as one undoable step, so that Source shows them and undo and redo take them back
and forth in either view; with Visual focused, Ctrl or Cmd with Z SHALL undo and Ctrl+Y or Ctrl or Cmd
with Shift+Z SHALL redo. The NX pane's title bar SHALL show Undo and Redo buttons in both views, each
disabled while the editor's history holds nothing to take back or forth, with tooltips naming their
shortcuts on the visitor's platform; in Visual, Undo SHALL also be enabled while a field holds a
change not yet committed, as `nx-draft` says, or an edit Visual fired is not yet in the history. An undo or redo, from a button or a shortcut, SHALL take its step
after every edit Visual fired before it, so that pressing Undo with a field open, which commits the
field, takes back the field's edit. On a narrow screen, the title bar's Collapse all and Expand all
SHALL show as icons, so the view switch stays in view. After an edit from Visual the playground SHALL ask for the source tree at once,
without waiting for a pause, and the preview and output SHALL follow as they do after an edit in
Source. Edits SHALL be applied in the order the viewer fires them, each against the text the one before
left. A refused edit SHALL leave the source as it was. For `nx-open`, the playground SHALL switch to
Source with the cursor at the start of the node and reveal it.

#### Scenario: An edit in Visual shows in Source
- **WHEN** a visitor on the visualizer declarations example changes the `Latte` drink's name to
  `Flat white` in Visual and switches to Source
- **THEN** the source SHALL read `name="Flat white"` where it read `name="Latte"`

#### Scenario: Undo from Visual
- **WHEN** the visitor then switches back to Visual and presses Ctrl+Z
- **THEN** the source SHALL read `name="Latte"` again, and Visual SHALL read `Latte`

#### Scenario: Undo from the title bar
- **WHEN** the visitor makes an edit in Visual and presses the title bar's Undo button
- **THEN** the source SHALL read as before the edit, and Redo SHALL be enabled

#### Scenario: Undo before the first edit
- **WHEN** a visitor opens an example, types in a field in Visual and, before committing it, presses
  the title bar's Undo button
- **THEN** the source SHALL read as it did, and Visual SHALL read the value as before

#### Scenario: Opening an expression in Source
- **WHEN** a visitor double-clicks a braced expression in Visual
- **THEN** Source SHALL show, with the cursor at the start of that expression

