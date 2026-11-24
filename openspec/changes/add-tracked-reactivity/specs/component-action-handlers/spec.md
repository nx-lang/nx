## ADDED Requirements

### Requirement: A handler captures only the slots its body reads
When a runtime makes an action handler, it SHALL capture the values of the slots the handler's node
and the nodes beneath it read, as they stand at that moment, and no other slot of the frame. The
runtime SHALL find those slots from the image, as it finds a formula's captured locals, at no
charge, and MAY keep them for each handler node. The Rust and TypeScript IR runtimes SHALL capture
alike, so that two handlers made by one node compare equal exactly when the slots that node reads
held equal values, and a handler's comparison and its written form SHALL cover only what it
captured. A state field the handler reads SHALL still be read live when the handler runs against the
component that bound it.

#### Scenario: A handler is equal when only a slot it does not read changed
- **WHEN** a component's body binds `<State.Update taps={taps + 1} />` in two renders between which
  only its prop `note` changed
- **THEN** the two handlers SHALL compare equal
- **AND** they SHALL compare unequal when `taps` changed

#### Scenario: Both runtimes capture alike
- **WHEN** both IR runtimes render a body that binds a handler reading one of three bound slots,
  and compare it with the handler of another render
- **THEN** both SHALL compare only that slot and count the same operations
