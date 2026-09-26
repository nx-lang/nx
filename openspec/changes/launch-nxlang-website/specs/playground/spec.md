## MODIFIED Requirements

### Requirement: Site is served under the playground path
Everything the site serves — the gallery, each example's editor view, and every static asset —
SHALL live under the `/playground` path prefix, so that the rest of the domain is served by the
website without the playground changing.

#### Scenario: Gallery address
- **WHEN** a visitor opens `/playground` or `/playground/`
- **THEN** the gallery SHALL be shown

#### Scenario: Example editor address
- **WHEN** a visitor opens `/playground/<id>` for a gallery example's id
- **THEN** the editor view SHALL open with that example loaded, without passing through the gallery

#### Scenario: Unknown example
- **WHEN** a visitor opens `/playground/<id>` and no example has that id
- **THEN** the site SHALL show the gallery rather than an error page

#### Scenario: API lives under the prefix
- **WHEN** a request names a path under `/playground/api/`
- **THEN** the site SHALL answer not found, since it serves no API: the client compiles and answers
  language queries itself, and a static site has no health route

#### Scenario: Assets live under the prefix
- **WHEN** the shell loads its scripts, styles, fonts, images, the CanvasKit binary and the NX
  compiler module
- **THEN** every one of those requests SHALL be for a path under `/playground/`

#### Scenario: Root redirects to the playground
- **WHEN** a visitor opens `/`
- **THEN** the website's landing page SHALL answer, with no redirect to `/playground`
- **AND** the landing page and the site header SHALL link to `/playground`

#### Scenario: Paths outside the prefix are not the shell
- **WHEN** a request names a path that is not under `/playground`
- **THEN** the website SHALL answer it, and the playground's shell SHALL NOT be served

#### Scenario: A missing asset is not the shell
- **WHEN** a request under `/playground/assets/` names a file the build did not produce
- **THEN** the site SHALL answer not found rather than serving the shell

### Requirement: Public deployment is fronted by an edge that serves assets
The playground SHALL be served from Cloudflare's edge as static files, over TLS, with no origin
process behind it. No rate limit SHALL be required, since no request costs more than a file.

#### Scenario: The site is reachable at its public address
- **WHEN** a visitor opens `https://nxlang.org/playground`
- **THEN** the gallery SHALL be served over TLS
- **AND** `http://nxlang.org/playground` SHALL redirect to it

#### Scenario: Assets are served from the edge
- **WHEN** a hashed asset, the CanvasKit binary or the NX compiler module is requested
- **THEN** it SHALL be served from Cloudflare's edge, with no request reaching an origin server

### Requirement: Site documents how to run, sync and deploy it
The site SHALL carry documentation covering how to build and run it locally, what it depends on
including the wasm toolchain, how to move its `drawnui-react` pin and refresh the DrawnUI assets
copied from upstream, and how it is deployed, including the one-time edge setup.

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
  setup, including every edge setting the site depends on
- **AND** they SHALL NOT describe a rate limit, a health route or an origin service the site no
  longer has

## REMOVED Requirements

### Requirement: Service reports its own health
**Reason**: There is no process left to be unhealthy. The playground is static files on Cloudflare's
edge, and a Workers deployment either uploads completely or does not replace the live version.
**Migration**: The deploy workflow's build and tests gate a deployment, and a post-deploy fetch of
the shell and the compiler module verifies it. `/playground/api/health` answers not found.

### Requirement: Origin sets cache policy for the edge
**Reason**: There is no origin. Cache policy is declared with the static files and served by the
edge, which the new requirement "Static files declare their cache policy" states.
**Migration**: The same policy (immutable hashed assets, day-long revalidation for unhashed files,
an always-revalidated shell) moves from the Node server into the site's static headers file.

### Requirement: Site is deployable as a single service
**Reason**: The site no longer runs a service. It deploys as static files, which the new
requirement "Site deploys as static files from main" states.
**Migration**: The Dockerfile, the Node server, `.railway/railway.ts` and the Railway project are
deleted, and the deploy workflow uploads the built files to a Cloudflare Worker instead.

## ADDED Requirements

### Requirement: Static files declare their cache policy
The site's static files SHALL declare cache headers that let browsers hold content-addressed assets
for a long time and never hold a stale shell, so that a deploy is visible on the next page load.

#### Scenario: Hashed assets are cacheable for a long time
- **WHEN** a build-output asset whose file name carries a content hash is served, the NX compiler
  module included
- **THEN** the response SHALL declare itself publicly cacheable, immutable, and valid for at least a
  year

#### Scenario: Static files without a hash are revalidated
- **WHEN** a font, image or other static file whose name carries no content hash is served
- **THEN** the response SHALL be cacheable but SHALL require revalidation within a day

#### Scenario: The shell is not cached
- **WHEN** the shell document is served, for any address that resolves to it
- **THEN** the response SHALL require revalidation on every use

### Requirement: Site deploys as static files from main
The site SHALL be built to static files from a clean checkout, the NX compiler module included, and
a push to `main` that touches the site or a package it depends on SHALL build, test and deploy it.
The deployment's configuration, meaning the route it serves, its client routing and its headers,
SHALL be committed in the repository.

#### Scenario: Build produces its own native dependencies
- **WHEN** the site is built from a clean checkout in CI
- **THEN** the build SHALL produce the NX compiler module the client loads
- **AND** it SHALL NOT depend on artifacts built outside that run

#### Scenario: Deploys follow the main branch
- **WHEN** a commit that touches the site or a package it depends on lands on `main`, and its build
  and tests pass
- **THEN** the new build SHALL replace the live one

#### Scenario: A failing build does not deploy
- **WHEN** the build or tests fail
- **THEN** the live site SHALL keep serving the previous build

#### Scenario: The deploy is verified
- **WHEN** a deployment finishes
- **THEN** the workflow SHALL fetch the shell and the compiler module from
  `https://nxlang.org/playground`, and fail if either does not answer
