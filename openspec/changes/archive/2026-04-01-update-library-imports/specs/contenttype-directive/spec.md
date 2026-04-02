## REMOVED Requirements

### Requirement: ContentType directive syntax
**Reason**: The `contenttype` directive is unused and is removed from the NX language.
**Migration**: Delete the `contenttype` directive. If the file still needs imported declarations,
replace it with explicit library imports.

### Requirement: ContentType must appear before imports
**Reason**: Removing `contenttype` eliminates its special file-ordering rule.
**Migration**: Delete the directive and keep ordinary import statements at the top of the file.

### Requirement: ContentType is optional
**Reason**: Files no longer support `contenttype` in any position.
**Migration**: Remove the directive from every file. A file with no imports remains valid without
any replacement.

### Requirement: ContentType in HIR
**Reason**: The HIR no longer tracks `contenttype` because the directive has been removed.
**Migration**: Remove any `content_type` field usage and model remaining dependencies through normal
library imports.
