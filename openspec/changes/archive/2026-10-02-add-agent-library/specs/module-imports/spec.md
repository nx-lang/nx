## ADDED Requirements

### Requirement: A standard library is imported by its reserved name
An import path of the form `@nx/<name>` SHALL name the standard library whose root is `@nx/<name>`,
as `standard-libraries` defines, in every import form: wildcard, namespace and selective. Such a path
SHALL NOT be resolved relative to the importing module or to a library root, SHALL NOT be subject to
the workspace root-escape rule, and SHALL resolve identically from a single source, a workspace
module at any depth, a directory library module and an in-memory library module. Resolution SHALL
NOT consult the filesystem or the workspace's modules. A path under `@nx/` that names no standard
library, including a path with further segments after the library name, SHALL be reported with a
diagnostic that names the path and lists the standard libraries that exist. A relative path that
resolves, from the importing module, to an identity under `@nx/` SHALL be reported with a diagnostic
saying the root is reserved and SHALL bind nothing, so a standard library has exactly one spelling.
The duplicate-import and ambiguous-name rules SHALL apply to a standard library import as to any
library import.

#### Scenario: Wildcard import of a standard library
- **WHEN** a file contains `import "@nx/agent"` and uses `Agent` and `Tool`
- **THEN** analysis SHALL resolve both names to the declarations of `@nx/agent`

#### Scenario: Namespace import of a standard library
- **WHEN** a file contains `import "@nx/agent" as Ai` and `let a = <Ai.Agent name="support">Be brief.</Ai.Agent>`
- **THEN** analysis SHALL accept the file
- **AND** the unqualified name `Agent` SHALL remain unresolved in that file

#### Scenario: Selective import of a standard library
- **WHEN** a file contains `import { Agent, Tool } from "@nx/agent"` and uses `Agent`, `Tool` and `Document`
- **THEN** analysis SHALL resolve `Agent` and `Tool` and SHALL report `Document` as unresolved

#### Scenario: The path is not relative to the importer
- **WHEN** workspace modules `main.nx` and `app/flows/support.nx` each contain `import "@nx/agent"`
- **THEN** both imports SHALL resolve to the same standard library
- **AND** neither SHALL be reported as escaping the workspace root

#### Scenario: An unknown standard library is diagnosed
- **WHEN** a file contains `import "@nx/automation"`
- **THEN** analysis SHALL report a diagnostic at the import naming `@nx/automation` and listing `@nx/agent`

#### Scenario: A module of a standard library is not importable on its own
- **WHEN** a file contains `import "@nx/agent/agent.nx"`
- **THEN** analysis SHALL report the same unknown-standard-library diagnostic, naming the path as written

#### Scenario: A relative path into the reserved root is refused
- **WHEN** `main.nx` contains `import "./@nx/agent"`, or a module of an in-memory library at `libraries/sneaky` contains `import "../../@nx/agent"`
- **THEN** analysis SHALL report a reserved-root diagnostic at the import naming the path
- **AND** the import SHALL NOT bind the standard library's declarations

#### Scenario: The same standard library imported twice is rejected
- **WHEN** a file contains `import "@nx/agent"` and `import { Agent } from "@nx/agent"`
- **THEN** analysis SHALL report a duplicate-library-import compile error for `@nx/agent`
