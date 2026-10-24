## REMOVED Requirements

### Requirement: Visual is read-only and leaves the run loop alone
**Reason**: Visual now edits the snippet, through the editor.
**Migration**: Replaced by "Visual edits the snippet through the editor", which keeps the rule that
switching views runs nothing and that sharing from Visual stores the editor's source.

## ADDED Requirements

### Requirement: Visual edits the snippet through the editor
While the editor accepts typing, the reading SHALL be editable: each edit made in Visual SHALL be
answered by the page's language service and applied to the editor's text as one step of the editor's
undo history, in the order the edits were made, each against the text the one before left. An edit the
service refuses or answers with no change SHALL leave the text as it was and the value reading as
before. After an edit, the reading SHALL be asked for at once rather than after the typing pause.
An edit in Visual SHALL reach diagnostics, run gating, HotReload and sharing as typing the same change
does. While the editor is read-only, as for a share opened to view, the reading SHALL edit nothing.
Showing either view SHALL NOT run, re-run or stop the snippet.

#### Scenario: Editing a number re-runs as typing does
- **WHEN** HotReload is on and the visitor changes `FontSize=26` to `32` in Visual
- **THEN** the editor's text SHALL read `FontSize=32` and the snippet SHALL run once, as after typing it

#### Scenario: One undo step
- **WHEN** the visitor sets a color in Visual and then presses Ctrl+Z in the reading
- **THEN** the editor's text SHALL be what it was before the color was set, and the reading SHALL show
  the old color

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
