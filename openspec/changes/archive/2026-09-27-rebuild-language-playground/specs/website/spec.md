## ADDED Requirements

### Requirement: Documentation code blocks open in the playground
Every `nx` code block on the website that is checked as a complete program, meaning unmarked or
marked `invalid`, SHALL carry an "Open in playground" control. The control SHALL open
`/play#code=<payload>` for exactly that block's text, in the playground's address encoding.
Blocks marked `fragment` or `output` SHALL NOT carry it.

#### Scenario: A complete example
- **WHEN** a visitor presses "Open in playground" on an unmarked `nx` block
- **THEN** the playground SHALL open with the block's text in the source pane, character for
  character, and show its output

#### Scenario: An example of an error
- **WHEN** a visitor presses "Open in playground" on an `nx invalid` block
- **THEN** the playground SHALL open with that text and show the error it produces

#### Scenario: Fragments and output have no control
- **WHEN** a page is built with an `nx fragment` block and an `nx output` block
- **THEN** neither block SHALL carry the control

#### Scenario: The link is built without running the playground
- **WHEN** the website is built
- **THEN** every control's address SHALL be computed at build time, with no script needed on the
  page to follow it
- **AND** the website's tests SHALL encode the shared fixture source to the shared fixture address

### Requirement: Documented output is checked
A code block fenced `nx output` SHALL be read as the evaluated value of the `nx` block immediately
before it. The documentation check SHALL evaluate that preceding block's `root` and fail when its
NX text differs from the output block's text, ignoring trailing whitespace. The preceding block must
be unmarked, and an `nx output` block anywhere else SHALL fail the check. The check evaluates with
the wasm SDK, so a documented program whose calls nest deeper than its limit of 200 fails the check
with the recursion-limit error, as it would in the playground.

#### Scenario: Matching output
- **WHEN** an unmarked block evaluating to `<User id="1" name="Ada" />` is followed by an `nx output`
  block with that text
- **THEN** the check SHALL pass for both

#### Scenario: Stale output
- **WHEN** a language change alters what the preceding block evaluates to
- **THEN** the check SHALL fail naming the page and the output block's line, and showing both texts

#### Scenario: An output block with nothing to evaluate
- **WHEN** an `nx output` block follows a `fragment` block, an `invalid` block, or no `nx` block
- **THEN** the check SHALL fail for it

#### Scenario: The landing page's value is checked
- **WHEN** the check runs over the landing page
- **THEN** the hero's shown value SHALL be an `nx output` block checked against the hero snippet

## MODIFIED Requirements

### Requirement: The site is served at the domain root
The website SHALL serve its landing page at `https://nxlang.org/` and its documentation pages at
paths under the domain root, with no base path. Every path under `/play` SHALL be served by the
playground rather than by the website.

#### Scenario: Landing page at the root
- **WHEN** a visitor opens `https://nxlang.org/`
- **THEN** the landing page SHALL be served over TLS, with no redirect

#### Scenario: A documentation page at its root-relative path
- **WHEN** a visitor opens `https://nxlang.org/language-tour/types/`
- **THEN** the Types page of the language tour SHALL be served

#### Scenario: The playground keeps its prefix
- **WHEN** a visitor opens `https://nxlang.org/play` or any path under `/play/`
- **THEN** the playground SHALL answer, not the website

#### Scenario: Plain HTTP and `www` go to the canonical address
- **WHEN** a visitor opens `http://nxlang.org/<path>` or `https://www.nxlang.org/<path>`
- **THEN** they SHALL be redirected to `https://nxlang.org/<path>`

#### Scenario: An unknown path
- **WHEN** a visitor opens a path outside `/play` that names no page, `/playground` among them
- **THEN** the website SHALL answer with status 404 and its own not-found page, which carries the
  site header and a link to the documentation

### Requirement: Every page carries the site header
Every page of the website SHALL carry one header that names NX, searches the documentation, and
links to the documentation, the playground and the GitHub repository.

#### Scenario: Header links
- **WHEN** a visitor opens any website page, the not-found page among them
- **THEN** the header SHALL link to the documentation, to `/play`, and to
  `https://github.com/nx-lang/nx`

#### Scenario: Search
- **WHEN** a visitor searches from the header for a term that appears on a documentation page
- **THEN** that page SHALL be among the results, with no request leaving the site
