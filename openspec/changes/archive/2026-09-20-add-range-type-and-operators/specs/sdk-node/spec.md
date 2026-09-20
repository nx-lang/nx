## ADDED Requirements

### Requirement: Node language snapshots accept implicit imports
The Node SDK's language snapshot SHALL accept, beside its build context, an optional list of
identities that every other document imports implicitly, with the meaning `workspace-programs` gives
implicit imports and the behaviour the wasm SDK's snapshot already has: the same names in scope for
hover, completions and diagnostics, each document's own text and positions unchanged, and a
diagnostic naming an identity the snapshot does not contain.

#### Scenario: Hover through an implicit import
- **WHEN** a snapshot is built from a catalog document and an author's document, with the catalog's identity as an implicit import
- **THEN** a hover on a tag the catalog declares, in the author's document, SHALL carry its signature

#### Scenario: An unknown implicit import is reported
- **WHEN** a snapshot names an implicit import that none of its documents has as an identity
- **THEN** its diagnostics SHALL include one naming that identity
