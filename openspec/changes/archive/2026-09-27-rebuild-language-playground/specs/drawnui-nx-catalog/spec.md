## REMOVED Requirements

### Requirement: Catalog declares the full DrawnUI control set
**Reason**: The playground no longer draws with DrawnUI, so this repository no longer carries a
DrawnUI catalog. The DrawnUI fiddle generates and specifies its own (`fiddle-nx-language`).
**Migration**: The fiddle's catalog in `DrawnUi.FiddleEngine/nx/catalog`.

### Requirement: Catalog is derived from DrawnUI sources rather than hand-maintained
**Reason**: The generator is deleted with the catalog; the fiddle keeps its own copy.
**Migration**: `DrawnUi.FiddleEngine/nx/generate-catalog.mjs`.

### Requirement: Catalog excludes members that cannot be authored
**Reason**: The catalog is removed from this repository.
**Migration**: The fiddle's catalog generator.

### Requirement: DrawnUI types map onto NX types by documented rules
**Reason**: The catalog is removed from this repository.
**Migration**: The fiddle's `nx/CATALOG.md`.

### Requirement: Catalog declares DrawnUI events as emits
**Reason**: The catalog is removed from this repository.
**Migration**: The fiddle's catalog generator.

### Requirement: Catalog compiles to NX IR
**Reason**: The catalog is removed from this repository.
**Migration**: The fiddle's catalog tests.

### Requirement: Catalog divergence from DrawnUI is recorded
**Reason**: The catalog is removed from this repository.
**Migration**: The fiddle's `nx/CATALOG.md`.

### Requirement: Catalog declares templated controls
**Reason**: The catalog is removed from this repository.
**Migration**: The fiddle's catalog generator.
