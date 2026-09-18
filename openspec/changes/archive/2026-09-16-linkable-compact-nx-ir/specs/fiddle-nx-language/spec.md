## ADDED Requirements

### Requirement: The catalog artifact ships with the NX runtime and is prepared once
The NX runtime bundle SHALL contain the DrawnUI catalog compiled to its own NX IR artifact, stamped
with the catalog version recorded when the catalog was generated, and SHALL prepare it once per page
the first time an NX snippet is compiled or played. Every compile of a snippet SHALL build it against
the catalog as a separate module, and every mount SHALL link the snippet's artifact against the
prepared catalog. The runtime build SHALL fail when the bundled catalog artifact does not match the
committed catalog source.

#### Scenario: The catalog is prepared once per page
- **WHEN** a visitor runs three NX snippets in one editor session
- **THEN** the catalog artifact SHALL be prepared once
- **AND** each snippet SHALL be linked against that preparation

#### Scenario: The player never receives the catalog in a share
- **WHEN** the player opens an NX share
- **THEN** every catalog declaration it uses SHALL come from the runtime bundle
- **AND** the share's module SHALL contain none of them

#### Scenario: A stale catalog artifact fails the build
- **WHEN** the committed catalog source has changed since the bundled artifact was emitted
- **THEN** `npm run runtime` SHALL fail naming the catalog and the command that regenerates it

### Requirement: The host registers NX
NX SHALL be registered by a host that chooses to offer it, after the engine's own registration,
rather than by the engine's default language set. The engine SHALL still ship NX's language class,
presets, script and runtime build so that a host adds NX with one registration line and a script
tag.

#### Scenario: The dev host offers NX
- **WHEN** a visitor opens the dev host's editor
- **THEN** the language switch SHALL offer NX after C# and React

#### Scenario: A host that does not register NX does not offer it
- **WHEN** a host calls only the engine's default registration
- **THEN** the language switch SHALL NOT offer NX
- **AND** an NX share opened there SHALL show its source and say the language is unknown

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

### Requirement: NX is a built-in language of the fiddle engine
The fiddle engine SHALL ship NX as a language with the stable id `nx`, on its React surface, for a
host to register next to C# and TSX. Switching to NX SHALL load NX's starter preset when the buffer
does not hold NX, and SHALL leave the buffer alone when it does.

#### Scenario: NX appears in the language switch
- **WHEN** a visitor opens the editor of a host that has registered NX
- **THEN** the language switch SHALL offer NX after C# and React, and the `fiddle.listPresets()` API
  SHALL list NX presets with `lang` equal to `nx`

#### Scenario: Switching to NX from foreign code loads the starter
- **WHEN** the buffer holds C# or TSX and the visitor switches to NX
- **THEN** the buffer SHALL be replaced by NX's first preset and drawn

#### Scenario: Switching to NX keeps NX
- **WHEN** the buffer holds text that declares a `root` function and the visitor switches to NX
- **THEN** the buffer SHALL be kept as it is

#### Scenario: Presets draw
- **WHEN** any NX preset is selected
- **THEN** it SHALL compile without errors and draw, using only fonts and assets the fiddle host
  serves or fetches by absolute URL
