## ADDED Requirements

### Requirement: A host can attach actions to nodes
`<nx-value>` SHALL take an `actions` property: a function the element calls with each node and its
index, which returns the actions for that node, each a label and an id, or nothing. The element
SHALL show each action as a button at the end of its node's first line, reachable by keyboard, and
SHALL dispatch an `nx-value-action` event carrying the node, its index and the action's id when the
visitor presses one. The buttons SHALL not change the value's text, so copying and folding work as
before, and a folded node SHALL keep its buttons visible. A stale value SHALL show its buttons
disabled.

#### Scenario: A button for a handler
- **WHEN** a host shows a value holding `onTapped=<ActionHandler action="Button.Tapped"
  token="h1-1" />` and its `actions` returns `{ id: "tap", label: "Tapped" }` for the
  `ActionHandler` record's node
- **THEN** a "Tapped" button SHALL appear on that line, and pressing it SHALL dispatch
  `nx-value-action` with that node and the id `tap`

#### Scenario: No actions
- **WHEN** a host sets no `actions`
- **THEN** the element SHALL show no buttons, as before

#### Scenario: A stale value
- **WHEN** the host sets `stale` on a value with actions
- **THEN** its buttons SHALL be shown disabled, and pressing one SHALL dispatch no event

#### Scenario: Copying leaves the buttons out
- **WHEN** the visitor copies a value with actions
- **THEN** the clipboard SHALL hold the value's text alone
