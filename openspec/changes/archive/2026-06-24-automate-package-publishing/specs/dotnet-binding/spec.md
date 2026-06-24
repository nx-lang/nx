## ADDED Requirements

### Requirement: NuGet package metadata identifies the public NX project
The `NxLang.Runtime` package metadata SHALL identify the public NX repository, project, license,
readme, and package purpose accurately before publication.

#### Scenario: Package metadata uses current repository URLs
- **WHEN** `NxLang.Runtime` is packed for publication
- **THEN** the package metadata SHALL use `https://github.com/nx-lang/nx` as the repository and
  project URL
- **AND** the package metadata SHALL NOT reference stale personal repository URLs

#### Scenario: Package metadata includes consumer-facing details
- **WHEN** a consumer inspects the `NxLang.Runtime` package metadata
- **THEN** the metadata SHALL include the package ID, title, description, tags, license expression,
  readme file, authorship, and repository information needed to identify the package
- **AND** the metadata SHALL describe the package as the managed .NET binding with packaged native
  NX runtime assets

### Requirement: NuGet package publication is automated from verified CI artifacts
NX SHALL publish `NxLang.Runtime` from the complete verified NuGet package artifact assembled by CI.
The publish workflow SHALL NOT publish an OS-local incomplete package that is missing supported RID
assets.

#### Scenario: Production publishes complete runtime package
- **WHEN** the Publish packages workflow targets the `production` environment for a trusted `main`
  Build workflow run
- **AND** the complete `NxLang.Runtime` package has passed package inspection
- **AND** package consumption smoke tests pass for every supported RID host in CI
- **AND** NuGet trusted publishing or `NUGET_API_KEY` fallback authentication is configured
- **THEN** CI SHALL publish the verified `.nupkg` to the configured production NuGet registry
- **AND** CI SHALL publish the `.nupkg` artifact produced by the package job without repacking
  different contents in the publish job

#### Scenario: Missing native asset blocks NuGet publication
- **WHEN** the assembled `NxLang.Runtime` package is missing a native `nx_ffi` library for an
  advertised supported runtime identifier
- **THEN** CI SHALL fail package verification before the NuGet publish job can write to a registry

#### Scenario: Preview NuGet publication is separate from production NuGet
- **WHEN** the Publish packages workflow targets the `preview` environment for `NxLang.Runtime`
- **THEN** CI MAY publish the package to a configured preview/test NuGet feed
- **AND** CI SHALL NOT use production NuGet credentials for preview publication
- **AND** CI SHALL keep preview package versions distinct from production package versions
- **AND** CI SHALL publish the `.nupkg` artifact produced by a verified package workflow run without
  repacking different contents in the preview publish job

### Requirement: NuGet deployment setup is documented
NX SHALL document the CI and registry setup needed to publish `NxLang.Runtime`.

#### Scenario: Maintainer configures NuGet publishing
- **WHEN** a maintainer reads `docs/deployment-setup.md`
- **THEN** the document SHALL describe NuGet.org trusted publishing setup for `NxLang.Runtime`
- **AND** it SHALL document the `NUGET_API_KEY` and `NUGET_USER` environment secret fallback when
  trusted publishing is not available
- **AND** it SHALL describe the preview/test feed option separately from production NuGet.org
  publication

#### Scenario: Maintainer follows NuGet release runbook
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL describe how the complete verified `NxLang.Runtime` package is
  published
- **AND** it SHALL describe how to confirm NuGet.org publication and repair or roll forward a failed
  NuGet release using the same `.nupkg` artifact where possible
