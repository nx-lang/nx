## ADDED Requirements

### Requirement: Examples format numbers and booleans into their readouts
Where a DrawnUI original formats a number or a boolean into text a visitor reads, such as a tap
count, a selected index, a slider's value, a speed or an `IsOpen` flag, and the value is one NX can
hold, the example SHALL hold it in component state and build the text with the language's implicit
conversion, rather than drawing a fixed string in its place. A value is one NX can hold when it is
the example's own state or arrives in the payload of an action the catalog declares. Where the
original numbers a run of items from an index, the example SHALL generate the run from a loop's
index rather than spelling each item out.

#### Scenario: A counter follows its taps
- **WHEN** a visitor taps the counting button on the Transforms or the Accessibility example
- **THEN** the button's text SHALL read `Tapped 1×`, then `Tapped 2×` on the next tap

#### Scenario: A readout follows the control it reports
- **WHEN** a visitor swipes a carousel on the Carousel & Drawer example
- **THEN** that carousel's `SelectedIndex=` readout SHALL show the index the carousel reports
- **AND** a whole index SHALL be shown without a fraction

#### Scenario: A boolean is shown as the language prints it
- **WHEN** a visitor opens the drawer on the Carousel & Drawer example
- **THEN** its readout SHALL read `IsOpen: true`

#### Scenario: Numbered rows come from a loop
- **WHEN** the SkiaScroll example's source is read
- **THEN** its numbered rows SHALL be produced by a loop that joins a prefix to the loop's index
- **AND** the drawing SHALL show the same rows the original shows

#### Scenario: A value NX cannot reach keeps its note
- **WHEN** the original reads the value from a control or an engine object, such as a Lottie's frame
  count or the accessibility manager's node count
- **THEN** the example SHALL leave that part of the readout out
- **AND** its source SHALL say so at that point, naming the code-behind capability

## MODIFIED Requirements

### Requirement: Examples declare how completely they cover the original
Each example SHALL declare its coverage as one of three states, so that a gap in NX's expressiveness
is never mistaken for a broken example, and so that an example whose drawing is correct is not
presented as if it were faulty:

- **complete** — nothing the DrawnUI original does is missing;
- **static** — the drawing is correct and complete, but some of the motion or interaction the
  original has is absent;
- **reduced** — the example is scaled down because NX cannot express the mechanism the original
  demonstrates.

#### Scenario: A complete example carries no coverage note
- **WHEN** an example's NX covers everything the DrawnUI original does
- **THEN** it SHALL NOT carry a coverage note or badge

#### Scenario: A static example is distinguished from a reduced one
- **WHEN** an example draws the original correctly but omits some of its motion or interaction
- **THEN** it SHALL declare itself static rather than reduced

#### Scenario: A static example that responds is not described as inert
- **WHEN** a static example's coverage note is shown
- **THEN** the note SHALL say that some of the original's motion or interaction is absent
- **AND** it SHALL NOT say that nothing in the example responds

#### Scenario: A reduced example states what the original demonstrates
- **WHEN** an example is scaled down from the original
- **THEN** it SHALL declare itself reduced
- **AND** what the original demonstrates SHALL be stated where the visitor can read it

#### Scenario: Coverage marking is proportionate
- **WHEN** a static or reduced example is presented
- **THEN** it SHALL NOT be presented as an error or failure
- **AND** its wording SHALL be addressed to a visitor, not to a maintainer surveying gaps

#### Scenario: Omitted examples are accounted for
- **WHEN** a DrawnUI demo page has no NX example at all
- **THEN** the site's documentation SHALL name it and say why
- **AND** the gallery SHALL NOT show an entry for it

