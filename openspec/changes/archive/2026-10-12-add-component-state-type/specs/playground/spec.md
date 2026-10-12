## MODIFIED Requirements

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
