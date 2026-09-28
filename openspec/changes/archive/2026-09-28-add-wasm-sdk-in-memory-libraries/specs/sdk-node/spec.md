## ADDED Requirements

### Requirement: Node SDK loads in-memory libraries with wasm parity
The Node SDK SHALL load a library from a logical root, an optional version, and in-memory modules,
alongside directory loading. For the same libraries, workspace, implicit imports and options, the
Node SDK and the wasm SDK SHALL emit byte-identical NX IR images, including library module images,
and SHALL report equal diagnostics.

#### Scenario: Node and wasm agree on an in-memory library graph
- **WHEN** `libraries/chat-link` and `libraries/question-flow` are loaded from memory into both SDKs
- **AND** the same tenant workspace is built with both libraries implicitly imported
- **THEN** the entry and library module images SHALL be byte-identical across the two SDKs
