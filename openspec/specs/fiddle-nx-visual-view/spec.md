# fiddle-nx-visual-view Specification

## Purpose

Defines the fiddle's Visual and Source switch for NX: a reading of the snippet in the NX viewer beside
the editor, how it is computed and drawn, how it shares a selection and folds with the editor, and the
language-agnostic contract through which a fiddle language offers a visual view.

## Requirements

### Requirement: A language can offer a visual view of its source
The fiddle engine SHALL let a language declare that it offers a visual view of its source, through
the language contract and its registration script, with no language named in the editor page. While a
language that offers one is selected, the editor pane SHALL show a Visual and Source switch above the
editor. While a language that does not is selected, the editor pane SHALL show the editor alone, laid
out as it is without this change. `nx` and `nx-rust` SHALL offer a visual view, and C# and TSX SHALL
NOT.

#### Scenario: NX shows the switch
- **WHEN** a visitor selects NX, on the React engine or the Rust engine
- **THEN** the editor pane SHALL show a switch offering Visual and Source, Visual first

#### Scenario: C# and TSX show the editor alone
- **WHEN** a visitor selects C# or TSX
- **THEN** the editor pane SHALL show no Visual and Source switch, and the editor SHALL fill the pane
  as it does without this change

#### Scenario: The engine switch keeps the view
- **WHEN** Visual shows for NX on the React engine and the visitor switches the engine to Rust
- **THEN** Visual SHALL still show, with the same folds and selection

### Requirement: Visual is the default and reads the snippet
Visual SHALL be the view shown when NX is first selected in a page. Visual SHALL show the editor's
NX source in the NX viewer, from the source-tree query of the language service the page already
runs for diagnostics, hover and completion, asked with visualizers, against the same catalog and
implicit imports. The tree SHALL be asked for at once when Visual first shows and after the
visitor pauses typing thereafter. While a newer tree is computed, the reading SHALL be marked stale.
Loading a preset or a share, or switching language to NX, SHALL show its reading. Nothing SHALL be
asked while Source shows.

#### Scenario: A preset reads
- **WHEN** a visitor opens the fiddle and selects NX with the starter preset
- **THEN** the editor pane SHALL show the preset's reading in Visual, and the canvas SHALL draw the
  preset as it does today

#### Scenario: A share reads
- **WHEN** a visitor opens the editor link of an NX share
- **THEN** the editor pane SHALL show the share's reading in Visual

#### Scenario: Source asks nothing
- **WHEN** Source shows and the visitor types
- **THEN** no source-tree query SHALL be made until Visual is shown again

#### Scenario: The query fails
- **WHEN** the source-tree query fails
- **THEN** Visual SHALL say that the reading is unavailable and why, and Source, the canvas and the
  diagnostics SHALL keep working

### Requirement: The reading takes the editor's colors and the fiddle's faces
The reading SHALL be colored with the theme the NX editor is colored with, SHALL follow the fiddle's
dark color scheme whatever the system prefers, SHALL set code in the editor's monospace face and
prose in the fiddle page's own face, and SHALL draw icons from Lucide. The icons SHALL be loaded apart
from the NX runtime the first time Visual shows, and a page that never shows Visual SHALL NOT fetch
them. Without the icons, the reading SHALL be drawn unmarked.

#### Scenario: Icons load on first use
- **WHEN** a visitor uses only C# and TSX, or plays an NX share in the `/p/` player
- **THEN** the page SHALL NOT request the icons
- **AND** the first time Visual shows, the page SHALL request them once, apart from the NX runtime

#### Scenario: Dark on a light system
- **WHEN** the visitor's system prefers a light scheme
- **THEN** the reading SHALL be drawn in its dark colors, matching the editor

### Requirement: The switch offers folding, and folds are kept
While Visual shows, the switch SHALL also offer Collapse all and Expand all. What the visitor folds
SHALL be kept while they switch between Visual and Source and between engines, and forgotten when
another preset or share is loaded or another language is selected.

#### Scenario: Folds survive a trip to Source
- **WHEN** the visitor folds a group in Visual, switches to Source, edits a line outside the group and
  switches back
- **THEN** the group SHALL still be folded

#### Scenario: A new preset starts from the defaults
- **WHEN** the visitor folds a group and then loads another preset
- **THEN** the new preset's reading SHALL be folded as the viewer folds by default

### Requirement: The selection and the place cross between Visual and Source
The editor and the reading SHALL each keep their scroll position while the other shows. When the
visitor switches, the view switched to SHALL show the same part of the snippet: the node selected,
or holding what is selected, drawn as far below the top as it was in the view left, when it was in
view there; otherwise the first node starting in that view, as far below the top. Switching to Visual
SHALL select the smallest node holding the editor's selection, less the whitespace at its ends, or the
node at the cursor when nothing is selected. Switching to Source SHALL select the text of the node
selected in Visual, with the cursor at its start, and focus the editor. A view the visitor left as the
switch to it placed it SHALL leave the other view where it was, so a round trip with nothing moved
comes back as it was. Positions SHALL be converted between the viewer's byte offsets and the editor's
positions, so text after a character outside the basic multilingual plane is placed right.

#### Scenario: From Visual to Source
- **WHEN** a visitor selects a `SkiaLabel` in Visual and switches to Source
- **THEN** the label's text SHALL be selected, the editor's cursor SHALL be at the `<` that starts
  it, and its line SHALL be as far below the editor's top as the label was below the reading's

#### Scenario: From Source to Visual
- **WHEN** the editor's cursor is inside a `SkiaButton`'s attributes and the visitor switches to Visual
- **THEN** the unit holding that attribute SHALL be selected in the reading

#### Scenario: Whole lines selected in Source
- **WHEN** the visitor selects a `SkiaLabel`'s whole line in Source, from its start through its
  newline, and switches to Visual
- **THEN** that label SHALL be selected in the reading

#### Scenario: After an emoji
- **WHEN** a line holds `<SkiaLabel Text="😀 hi" Tag="x" />` and the visitor selects it in Visual and
  switches to Source
- **THEN** the cursor SHALL be at the line's `<`

#### Scenario: Scrolled in Visual
- **WHEN** the visitor scrolls the reading of a long preset, with nothing selected, and switches to
  Source
- **THEN** the line where the reading's first node in view starts SHALL be as far below the editor's
  top as that node is below the reading's

#### Scenario: Scrolled in Source
- **WHEN** the visitor scrolls the editor and switches to Visual
- **THEN** the first node starting in the editor's view SHALL be drawn as far below the reading's top

#### Scenario: A round trip
- **WHEN** the visitor switches from Visual to Source and back, moving nothing in Source
- **THEN** the reading SHALL be scrolled as it was, with the same selection

### Requirement: Visual edits the snippet through the editor
While the editor accepts typing, the reading SHALL be editable: each edit made in Visual SHALL be
answered by the page's language service and applied to the editor's text as one step of the editor's
undo history, in the order the edits were made, each against the text the one before left. An edit the
service refuses or answers with no change SHALL leave the text as it was and the value reading as
before. After an edit, the reading SHALL be asked for at once rather than after the typing pause.
An edit in Visual SHALL reach diagnostics, run gating, HotReload and sharing as typing the same change
does. A value the reader is still changing, as with a stepper, the color picker or a pause in
typing, SHALL reach the editor's text, the diagnostics and HotReload at once, and the whole change
SHALL be one step of the undo history once committed; a change cancelled SHALL leave the text and the
history as they were. While the editor is read-only, as for a share opened to view, the reading SHALL
edit nothing.
Showing either view SHALL NOT run, re-run or stop the snippet.

#### Scenario: Editing a number re-runs as typing does
- **WHEN** HotReload is on and the visitor changes `FontSize=26` to `32` in Visual
- **THEN** the editor's text SHALL read `FontSize=32` and the snippet SHALL run once, as after typing it

#### Scenario: The canvas follows a stepper
- **WHEN** HotReload is on and the visitor opens the field of `FontSize=26` in Visual and clicks its up
  stepper
- **THEN** the editor's text SHALL read `FontSize=27` and the snippet SHALL run, before the field closes

#### Scenario: A color picked live, one step
- **WHEN** the visitor drags in the color picker of `TextColor` and then closes the popover
- **THEN** the canvas SHALL follow the drag, and one Undo SHALL take the text back to the color before
  the drag

#### Scenario: One undo step
- **WHEN** the visitor sets a color in Visual and then presses Ctrl+Z in the reading
- **THEN** the editor's text SHALL be what it was before the color was set, and the reading SHALL show
  the old color

#### Scenario: Undo selects what it changed
- **WHEN** the visitor steps `Spacing` in Visual, scrolls down and selects something else, and presses
  Undo
- **THEN** the reading SHALL select the `Spacing` row and scroll it into view; when a step takes a
  property out, as an Undo of one added, it SHALL select the element that held it, and when it puts
  one back, that property

#### Scenario: A read-only share
- **WHEN** a visitor opens someone else's share, read-only, and clicks a value in Visual
- **THEN** no field SHALL open and the text SHALL be unchanged

#### Scenario: Switching does not run
- **WHEN** a visitor switches between Visual and Source with no edit in between
- **THEN** the snippet SHALL NOT be run again

#### Scenario: Sharing from Visual
- **WHEN** a visitor edits in Visual and shares while Visual shows
- **THEN** the share SHALL store the editor's source, with the edit, and its compiled program, as it
  does from Source

### Requirement: Visual offers Undo and Redo
While Visual shows for a language whose view reports an undo history, the bar SHALL show Undo and
Redo beside Collapse all and Expand all, each enabled when the editor's history has a step that way, or,
for Undo, when a field in the reading holds an uncommitted change or an edit is still being made.
Pressing one SHALL take that step in the editor's history once the edits made before it are applied.
A language whose view reports no history SHALL show neither.

#### Scenario: Undo from the bar
- **WHEN** the visitor deletes an item in Visual and presses Undo on the bar
- **THEN** the item SHALL be back in the editor's text and the reading

#### Scenario: Undo with a field open
- **WHEN** the visitor types a new value into a field in Visual and presses Undo without committing it
- **THEN** the text SHALL be unchanged and the field's old value SHALL read again

### Requirement: Visual opens a value in Source
The page SHALL switch to Source with a value's text selected and the cursor at its start when the
reading asks to show that value in the source, from Show in Source on a row's or an item's menu or
Enter on a value it cannot edit.

#### Scenario: An expression opens in Source
- **WHEN** the visitor presses Enter on `Text={titleOf(taps)}`'s value in Visual
- **THEN** Source SHALL show with the expression `titleOf(taps)` selected
