## ADDED Requirements

### Requirement: The package gives a host an edit queue
The package SHALL export `createEditQueue(service, uri, document, onApplied, onUnchanged?)`, which
makes the element's edits in the order they are fired: for each, it SHALL ask `service.sourceEdit` for
the document's text as it is when the edit's turn comes, and apply the answer's text edits to the
document as one step unless the text changed while it was asked. It SHALL call `onApplied` with the new
text after an edit that changed it, and `onUnchanged` after one that was refused, answered with no
edits, answered for text since changed, or failed; a failed edit SHALL NOT hold up the ones after it.
The queue's `after(step)` SHALL run `step` once every edit fired before it has been made or refused.

#### Scenario: Edits in order
- **WHEN** a host fires two `move` edits on the same item before the first is answered
- **THEN** the second SHALL be asked for the text the first left, and both SHALL be applied

#### Scenario: Text changed meanwhile
- **WHEN** the document's text changes while an edit is being asked
- **THEN** the answer SHALL NOT be applied and `onUnchanged` SHALL be called

#### Scenario: Undo after an edit in flight
- **WHEN** a host calls `after(undo)` while an edit is being asked
- **THEN** `undo` SHALL run after that edit is applied
