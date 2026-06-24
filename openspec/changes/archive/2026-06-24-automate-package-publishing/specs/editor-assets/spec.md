## ADDED Requirements

### Requirement: Editor assets participate in CI package release environments
NX SHALL publish the `@nx-lang/language` editor-assets package through the same preview and
production CI release model used by the other package outputs while preserving its separate package
identity and runtime-free contents.

#### Scenario: Pull request packages editor-assets tarball
- **WHEN** a pull request build runs for editor-assets-related changes
- **THEN** CI SHALL run the editor grammar tests
- **AND** CI SHALL pack the `@nx-lang/language` tarball
- **AND** CI SHALL verify and smoke-test the packed tarball
- **AND** CI SHALL upload the tarball without publishing it to the public npm registry from an
  untrusted pull request context

#### Scenario: Production publishes verified editor-assets tarball
- **WHEN** the Publish packages workflow targets the `production` environment for a trusted `main`
  Build workflow run
- **AND** the editor-assets tarball has passed package verification and smoke tests
- **AND** npm trusted publishing is configured
- **THEN** CI SHALL publish the verified `@nx-lang/language` tarball to the configured production npm
  registry
- **AND** CI SHALL publish the tarball artifact produced by the package job without repacking
  different contents in the publish job

#### Scenario: Preview editor-assets publication is separate from npm production
- **WHEN** the Publish packages workflow targets the `preview` environment for editor assets
- **THEN** CI MAY publish the tarball to a configured preview/test npm-compatible feed
- **AND** CI SHALL NOT use production npm credentials for preview publication
- **AND** CI SHALL keep preview package versions distinct from production package versions
- **AND** CI SHALL publish the tarball artifact produced by a verified package workflow run without
  repacking different contents in the preview publish job

### Requirement: Editor assets deployment setup is documented
NX SHALL document the CI and registry setup needed to publish `@nx-lang/language`.

#### Scenario: Maintainer configures npm publishing
- **WHEN** a maintainer reads `docs/deployment-setup.md`
- **THEN** the document SHALL describe npm trusted publishing setup for `@nx-lang/language`
- **AND** it SHALL document that production npm publication uses trusted publishing without an
  `NPM_TOKEN` fallback
- **AND** it SHALL describe the preview/test feed option separately from production npm publication

#### Scenario: Maintainer follows npm release runbook
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL describe how the verified `@nx-lang/language` tarball is published
- **AND** it SHALL describe how to confirm npm publication and repair a failed npm publish using the
  same tarball artifact
