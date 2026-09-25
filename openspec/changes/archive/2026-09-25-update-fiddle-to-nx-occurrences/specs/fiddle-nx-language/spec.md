## MODIFIED Requirements

### Requirement: An NX share stores its program and plays without the compiler
Sharing an NX snippet SHALL store the snippet's source with language `nx` and, as the share
artifact, a module carrying the snippet's own NX IR artifact and nothing else: no catalog
declaration, no runtime, no debug section. That artifact SHALL name the catalog and the catalog
version it was compiled against. The `/p/` player SHALL prepare the bundled catalog, link the share's
artifact against it, and draw with the NX runtime bundle and the DrawnUI engine alone, never
fetching the compiler module. Every NX preset's share module SHALL be under 128 KB, half the
backend's limit.

#### Scenario: A share round-trips in NX
- **WHEN** a visitor shares an NX snippet and another visitor opens the share's editor link
- **THEN** the editor SHALL open in NX with the same source

#### Scenario: The player draws without the compiler
- **WHEN** the player opens an NX share
- **THEN** it SHALL draw the same output the editor showed, SHALL post the engine's `ready` message
  after the first drawn frame, and SHALL make no request for the compiler module

#### Scenario: A share names its catalog version
- **WHEN** a visitor shares a snippet compiled against catalog version `9`
- **THEN** the stored module's artifact SHALL list the catalog with version `9` in its module table

#### Scenario: A share survives a regenerated catalog
- **WHEN** a share was compiled against a catalog in which `SkiaLabel` was the tenth component
- **AND** the host's bundle now carries a regenerated catalog of the same version in which a new
  component precedes `SkiaLabel`
- **THEN** the player SHALL draw the share unchanged

#### Scenario: A share from another catalog version is reported
- **WHEN** a share names catalog version `9` and the bundle carries version `10`
- **THEN** the player SHALL attempt to link by name and draw when every referenced declaration exists
- **AND** it SHALL draw the engine's failure label naming both versions when one does not

#### Scenario: Presets fit the budget
- **WHEN** the runtime's tests compile every NX preset
- **THEN** each share module SHALL be under 128 KB

#### Scenario: Screenshots work for NX
- **WHEN** a thumbnail or frame is taken of a running NX snippet
- **THEN** it SHALL show the drawn output, as it does for TSX

#### Scenario: A host without NX shows the source
- **WHEN** an NX share is opened on a host that has not registered NX
- **THEN** the host SHALL show the source and say the language is unknown rather than failing

#### Scenario: A share from another NX IR schema is reported
- **WHEN** the player opens a share whose NX IR image was written under a schema version other than
  the one the NX runtime bundle reads
- **THEN** the player SHALL draw the engine's failure label naming both schema versions, and SHALL
  NOT attempt to link or draw the share
- **AND** the share's editor link SHALL still open the stored source in NX, where compiling it
  reports whatever the current language rejects in the snippet's own line numbers
