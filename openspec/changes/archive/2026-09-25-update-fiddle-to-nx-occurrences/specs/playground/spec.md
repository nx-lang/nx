## MODIFIED Requirements

### Requirement: Site documents how to run, sync and deploy it
The site SHALL carry documentation covering how to build and run it locally, what it depends on
including the wasm toolchain, how to move its `drawnui-react` pin and refresh the DrawnUI assets
copied from upstream, and how it is deployed, including the one-time edge and hosting setup.

#### Scenario: Local run is documented
- **WHEN** a contributor reads the site's documentation
- **THEN** it SHALL describe the prerequisites, the wasm toolchain among them, and the steps to run
  the site locally under the `/playground` prefix with no compile server

#### Scenario: Vendored source provenance is recorded
- **WHEN** a contributor inspects the DrawnUI assets the site copies from upstream (fonts, images,
  animations, shaders and the reference demo pages)
- **THEN** the upstream tag they were taken from SHALL be recorded
- **AND** that tag SHALL be the release of the `drawnui-react` version the site pins

#### Scenario: Deployment is documented
- **WHEN** a maintainer needs to deploy, roll back, or set the site up on a fresh hosting account
- **THEN** the repository's deployment docs SHALL describe the day-to-day flow and the one-time
  setup, including every edge rule the site depends on
- **AND** they SHALL NOT describe a rate limit the site no longer needs

## ADDED Requirements

### Requirement: The site draws with the pinned DrawnUI package
The site SHALL draw with the published `drawnui-react` package, pinned to an exact version, and
SHALL carry no copy of DrawnUI's runtime source. Every DrawnUI class, control and React entry the
site uses SHALL come from the package's published entry points. The package version SHALL be the
one the DrawnUI fiddle pins when both are updated together, so that the two sites' catalogs describe
the same controls.

#### Scenario: No vendored runtime source
- **WHEN** a contributor searches the site's sources for DrawnUI's runtime code
- **THEN** there SHALL be none
- **AND** every DrawnUI import SHALL name the `drawnui-react` package or one of its published
  subpath entries

#### Scenario: Controls outside React are built from exported classes
- **WHEN** the site builds a control outside the React reconciler, as a templated list's cell does
- **THEN** it SHALL construct the control from the class the package exports under the tag's name
- **AND** it SHALL report an unknown tag once and draw nothing in its place, rather than failing the
  cell

#### Scenario: Moving the pin is one reviewable step
- **WHEN** a maintainer moves the `drawnui-react` pin and regenerates the catalog
- **THEN** the site's tests and example checks SHALL pass only once the catalog records the new
  version
- **AND** the catalog's changes SHALL show as an ordinary diff
