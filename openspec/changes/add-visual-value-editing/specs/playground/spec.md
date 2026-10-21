## ADDED Requirements

### Requirement: Visual edits the visitor's source through the editor
The playground SHALL set the viewer's `editable` in Visual. For each `nx-edit`, it SHALL ask its
worker's language service for the `sourceEdit` of the visitor's source and apply the answer's edits
to the editor's model as one undoable step, so that Source shows them and undo and redo take them back
and forth in either view; with Visual focused, Ctrl or Cmd with Z SHALL undo and Ctrl+Y or Ctrl or Cmd
with Shift+Z SHALL redo. After an edit from Visual the playground SHALL ask for the source tree at once,
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

#### Scenario: Opening an expression in Source
- **WHEN** a visitor double-clicks a braced expression in Visual
- **THEN** Source SHALL show, with the cursor at the start of that expression
