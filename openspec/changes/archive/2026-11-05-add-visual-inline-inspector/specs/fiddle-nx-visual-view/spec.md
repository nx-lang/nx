## ADDED Requirements

### Requirement: NX Visual reads elements as rows with inline inspectors
The playground SHALL show NX Visual, on both engines, with the viewer's element rows and inline
inspectors, from the source tree and visualizers it already asks for. Inspectors SHALL edit through
the existing source-edit and undo paths and SHALL NOT need a running preview. A selection the host
sets, such as one carried from Source, SHALL open the inspector that holds it.

#### Scenario: A preset opens
- **WHEN** a visitor loads an NX preset
- **THEN** Visual SHALL show its rows with the default folds, no pins, and an inspector only for the
  selected element, if any

#### Scenario: No preview running
- **WHEN** the snippet is not running
- **THEN** selecting, pinning and editing in Visual SHALL still work

### Requirement: Pins and closed inspectors follow the document
The playground SHALL keep the viewer's `detailsState` with the folds and the selection across
Visual/Source and engine switches for the same document, restore it before the selection on
returning to Visual, and apply the state the viewer reports after source changes. Loading another
preset or share, or choosing another language, SHALL clear it. It SHALL NOT be saved in source,
shares or browser storage, nor enter undo history.

#### Scenario: A round trip through Source
- **WHEN** a visitor pins Star, switches to Source and back without editing
- **THEN** Star SHALL still be pinned and the selection and place restored

#### Scenario: A new document
- **WHEN** a visitor loads another preset after pinning Star
- **THEN** the new document SHALL start with no pins

#### Scenario: Undo is for edits only
- **WHEN** a visitor edits a property, pins its element, and presses Undo
- **THEN** Undo SHALL revert the edit and leave the pin

## MODIFIED Requirements

### Requirement: Visual edits the snippet through the editor
While the editor accepts typing, the reading SHALL be editable: each edit made in Visual SHALL be
answered by the page's language service and applied to the editor's text as one step of the editor's
undo history, in the order the edits were made, each against the text the one before left. An edit the
service refuses or answers with no change SHALL leave the text as it was and the value reading as
before. After an edit, the reading SHALL be asked for at once rather than after the typing pause.
An edit in Visual SHALL reach diagnostics, run gating, HotReload and sharing as typing the same change
does. A value the reader is still changing, as with scrubbing, the color picker or a pause in
typing, SHALL reach the editor's text, the diagnostics and HotReload at once, and the whole change
SHALL be one step of the undo history once committed; a change cancelled SHALL leave the text and the
history as they were. While the editor is read-only, as for a share opened to view, the reading SHALL
edit nothing.
Showing either view SHALL NOT run, re-run or stop the snippet.

#### Scenario: Editing a number re-runs as typing does
- **WHEN** HotReload is on and the visitor changes `FontSize=26` to `32` in Visual
- **THEN** the editor's text SHALL read `FontSize=32` and the snippet SHALL run once, as after typing it

#### Scenario: The canvas follows scrubbing
- **WHEN** HotReload is on and the visitor drags the label of `FontSize=26` in Visual to the right
- **THEN** the editor's text SHALL follow the drag and the snippet SHALL run before the drag ends, and
  one Undo SHALL take the text back to `FontSize=26`

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

### Requirement: The switch offers folding, and folds are kept
While Visual shows, the switch SHALL also offer Collapse outline and Expand outline, which change
folds only, not open inspectors or pins. What the visitor folds
SHALL be kept while they switch between Visual and Source and between engines, and forgotten when
another preset or share is loaded or another language is selected.

#### Scenario: Folds survive a trip to Source
- **WHEN** the visitor folds a row's children in Visual, switches to Source, edits a line outside that
  element and switches back
- **THEN** the row SHALL still be folded

#### Scenario: A new preset starts from the defaults
- **WHEN** the visitor folds a row and then loads another preset
- **THEN** the new preset's reading SHALL be folded as the viewer folds by default
