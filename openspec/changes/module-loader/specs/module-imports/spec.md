## MODIFIED Requirements

### Requirement: Imports in HIR
The HIR Module struct SHALL include an imports list. Each import SHALL store the module path, import kind (wildcard or selective), optional alias (for wildcard), and individual name/alias pairs (for selective imports). At evaluation time, the NX evaluation pipeline SHALL use the configured `ModuleLoaderList` to resolve each import's module path, using the importing module's context for relative paths, load the source, parse and lower the imported module, and bind the imported names into the importing module's scope.

#### Scenario: Wildcard import without alias lowered to HIR
- **WHEN** a file contains `import "./ui"`
- **THEN** the HIR Module SHALL contain an import with path `./ui`, kind wildcard, and alias `None`

#### Scenario: Namespace import lowered to HIR
- **WHEN** a file contains `import "./ui" as UI`
- **THEN** the HIR Module SHALL contain an import with path `./ui`, kind wildcard, and alias `Some("UI")`

#### Scenario: Selective imports lowered to HIR
- **WHEN** a file contains `import { Button, Stack as LayoutStack } from "./ui"`
- **THEN** the HIR Module SHALL contain an import with path `./ui`, kind selective, and entries `[("Button", None), ("Stack", Some("LayoutStack"))]`

#### Scenario: Runtime resolution of wildcard import
- **WHEN** a file contains `import "./helpers"` and a `ModuleLoaderList` is configured with a directory containing `helpers.nx`
- **THEN** the evaluation pipeline SHALL resolve, load, and parse `helpers.nx`, and all public definitions from it SHALL be available in the importing module's scope

#### Scenario: Runtime resolution uses importing module context for relative paths
- **WHEN** file `/project/src/main.nx` contains `import "./helpers"` and `/project/src/helpers.nx` exists
- **THEN** the evaluation pipeline SHALL resolve `./helpers` from `/project/src/` before loading and parsing the imported module

#### Scenario: Runtime resolution of selective import
- **WHEN** a file contains `import { greet } from "./helpers"` and `helpers.nx` defines a public function `greet`
- **THEN** the evaluation pipeline SHALL make only `greet` available in the importing module's scope

#### Scenario: Import resolution failure
- **WHEN** a file contains `import "./missing"` and no loader can resolve the path
- **THEN** the evaluation pipeline SHALL convert that loader failure into a diagnostic indicating the module could not be found

#### Scenario: Loader error becomes an import diagnostic
- **WHEN** import resolution fails because a loader returns a `ModuleLoadError` such as a case mismatch or I/O failure
- **THEN** the evaluation pipeline SHALL convert that loader failure into a diagnostic describing the import failure
