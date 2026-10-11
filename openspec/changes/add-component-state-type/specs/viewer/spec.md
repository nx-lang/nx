## MODIFIED Requirements

### Requirement: Handlers read as events
The element SHALL read an attribute flagged `handler` as "When" followed by its name, without an
`on` prefix, split into lower-case words. When the handler's value is an element flagged
`stateUpdate`, it SHALL list one line per attribute of that element reading "set", the attribute's
name, "to" and its value. Any other handler value SHALL be read after an arrow as an ordinary value.

#### Scenario: A state update
- **WHEN** an element binds `onIntegerAnswered=<State.Update answered={answered + 1} teamSize={action.answer.value} />`
  inside a component body
- **THEN** it SHALL read "When integer answered" followed by "set answered to answered + 1" and "set
  teamSize to action.answer.value"
