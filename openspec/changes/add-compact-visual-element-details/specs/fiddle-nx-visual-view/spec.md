## ADDED Requirements

### Requirement: NX Visual uses compact composition rows
The playground SHALL enable the viewer's compact presentation for NX Visual on both engines, against
the existing source tree and visualizers. It SHALL use real property editors and the existing source
edit/undo paths. Compact presentation SHALL NOT require a running preview, change source selection
mapping, or add runtime-instance picking. Existing host-driven selection SHALL reveal the required
details. The whole-outline toolbar commands SHALL read Collapse outline and Expand outline and SHALL
leave property disclosure choices and pins unchanged.

#### Scenario: A preset opens compactly
- **WHEN** a visitor loads an NX preset
- **THEN** Visual SHALL show compact composition rows, default child folds, and no explicit property
  openings or pins; selection-driven details SHALL appear only when there is a selected owner

#### Scenario: Preview is unavailable
- **WHEN** a source tree is available but the snippet is not running
- **THEN** compact navigation, property disclosure, pinning, and permitted source edits SHALL work

#### Scenario: Existing host selection
- **WHEN** the host supplies a selection key for a property hidden in compact Visual
- **THEN** Visual SHALL reveal and select that property using the existing source range mapping

### Requirement: Compact view choices follow the editor document lifecycle
The playground SHALL preserve explicit-open, pinned, and suppressed-detail state alongside folds
and selection while switching Visual/Source or engines for the same document. It SHALL restore
choices before revealing selection, retain them through loading and no-op edit refreshes, and apply
the viewer's normalized state after source changes. Loading another preset or share, or selecting
another language, SHALL clear all compact disclosure choices. These choices SHALL NOT be serialized
into source, shares, or browser storage or enter the source undo history.

#### Scenario: Source round trip
- **WHEN** a visitor pins Star, explicitly opens Circle, switches to Source, and returns without edits
- **THEN** the pin and explicit opening SHALL remain and the same selection and place SHALL be restored

#### Scenario: Engine switch
- **WHEN** a visitor switches NX engines while a property sheet is pinned
- **THEN** the pin SHALL remain attached to the same source element

#### Scenario: New document
- **WHEN** a visitor loads another preset after pinning Star
- **THEN** the new document SHALL start without the old pin or property disclosure choices

#### Scenario: Undo concerns source edits
- **WHEN** a visitor edits a property, toggles its pin, and invokes Undo
- **THEN** Undo SHALL revert the property edit and SHALL NOT undo the pin toggle

## MODIFIED Requirements

### Requirement: The switch offers folding, and folds are kept
While Visual shows, the switch SHALL also offer Collapse outline and Expand outline. These commands
SHALL collapse and expand child-content and declaration folds independently of property details and
pins. What the visitor folds SHALL be kept while they switch between Visual and Source and between
engines, and forgotten when another preset or share is loaded or another language is selected.

#### Scenario: Folds survive a trip to Source
- **WHEN** the visitor folds a group in Visual, switches to Source, edits a line outside the group and
  switches back
- **THEN** the group SHALL still be folded

#### Scenario: A new preset starts from the defaults
- **WHEN** the visitor folds a group and then loads another preset
- **THEN** the new preset's reading SHALL be folded as the viewer folds by default

#### Scenario: Outline commands preserve comparison
- **WHEN** the visitor pins Star's properties and invokes Collapse outline followed by Expand outline
- **THEN** Star's pin SHALL remain and its properties SHALL show when its ancestors are expanded
