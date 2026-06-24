## ADDED Requirements

### Requirement: Package publishing uses explicit deployment environments
NX SHALL model package publication through explicit GitHub Actions deployment environments so
preview/test and production publishing can use separate credentials, protection rules, and audit
history.

#### Scenario: Pull request package preview builds
- **WHEN** a pull request build runs for package-related changes
- **THEN** CI SHALL build and verify publishable package artifacts without requiring public registry
  credentials
- **AND** CI SHALL upload the package artifacts for maintainer inspection
- **AND** CI SHALL NOT publish to public package registries from an untrusted pull request context

#### Scenario: Trusted preview environment publishing
- **WHEN** a maintainer-approved Publish packages workflow targets the `preview` environment
- **THEN** CI MAY publish package artifacts to configured preview/test feeds
- **AND** CI SHALL use credentials scoped to the `preview` environment or the repository-scoped
  `GITHUB_TOKEN`
- **AND** CI SHALL keep preview/test package publication separate from public production registry
  publication

#### Scenario: Manual preview publish reuses verified artifacts
- **WHEN** a maintainer manually runs the Publish packages workflow to publish preview packages from a
  previous Build workflow run
- **AND** the maintainer supplies the source workflow run ID
- **THEN** CI SHALL validate that the source workflow run completed successfully
- **AND** CI SHALL download the verified package artifacts from the source workflow run
- **AND** CI SHALL publish those artifacts without rebuilding package contents in the preview publish
  workflow run

#### Scenario: Production environment publishing
- **WHEN** a trusted `main` Build workflow run completes successfully
- **AND** all required package verification and smoke-test jobs pass
- **THEN** the Publish packages workflow SHALL target the `production` environment
- **AND** CI SHALL publish production package artifacts to the configured production registries
- **AND** CI SHALL use only credentials or trusted-publishing policies scoped to the `production`
  environment

### Requirement: Production publishing uses verified immutable package artifacts
NX SHALL publish only package artifacts that were built, inspected, and verified before the publish
step. Production publishing MUST fail before registry writes when artifact verification fails or when
the package version is not publishable.

#### Scenario: Publish jobs consume verified artifacts
- **WHEN** a production publish job runs
- **THEN** it SHALL download or receive artifacts produced by successful build/package jobs
- **AND** it SHALL publish those artifacts without rebuilding package contents in the publish job

#### Scenario: Duplicate or invalid package version blocks publication
- **WHEN** a package artifact has a version that is invalid for its registry or already published to
  the target production registry
- **THEN** CI SHALL fail or skip the duplicate as an idempotent retry before attempting a new public
  version write
- **AND** CI SHALL NOT overwrite an existing public package version

#### Scenario: Partial registry failure is repairable
- **WHEN** one production registry publish succeeds and another registry publish fails
- **THEN** CI SHALL preserve the exact package artifacts used for the successful publish
- **AND** the deployment documentation SHALL describe how to repair the failed registry by publishing
  the same artifact rather than rebuilding

### Requirement: Registry authentication prefers trusted publishing
NX SHALL use keyless trusted-publishing or repository-scoped credentials where the target registry
supports them. Long-lived registry tokens SHALL be documented as fallbacks or for registries that do
not support trusted publishing.

#### Scenario: Trusted publishing policy is configured
- **WHEN** a registry supports trusted publishing for GitHub Actions
- **THEN** the production workflow SHALL be able to publish using GitHub OIDC and the registry's
  trusted-publisher policy
- **AND** the workflow SHALL request only the permissions needed for token exchange and publication

#### Scenario: Token fallback is configured
- **WHEN** trusted publishing is unavailable for a target registry
- **THEN** CI SHALL read the required token from a GitHub environment secret
- **AND** CI SHALL fail before publication if the token is missing
- **AND** CI SHALL NOT store registry tokens in tracked source files

### Requirement: Deployment setup and runbook are documented
NX SHALL include separate deployment documentation for one-time CI/registry setup and ongoing
package publishing operations.

#### Scenario: Maintainer configures deployment environments
- **WHEN** a maintainer reads `docs/deployment-setup.md`
- **THEN** the document SHALL list the GitHub environments to create
- **AND** it SHALL describe recommended protection rules for `preview` and `production`
- **AND** it SHALL identify which secrets, variables, and trusted-publishing policies belong to each
  environment

#### Scenario: Maintainer configures package registries
- **WHEN** a maintainer reads `docs/deployment-setup.md`
- **THEN** the document SHALL describe setup for NuGet.org, npm, the Visual Studio Marketplace,
  Open VSX, and any recommended preview/test feeds
- **AND** it SHALL explain which registries can use trusted publishing and which require token
  secrets

#### Scenario: Maintainer understands automatic main publishing
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL explain that trusted `main` builds can publish production packages
  automatically after verification
- **AND** it SHALL explain the package-registry immutability constraints that make version checks and
  repair commands necessary

#### Scenario: Maintainer follows the release runbook
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL describe how to publish a new release
- **AND** it SHALL list the verification, artifact inspection, environment approval, and registry
  confirmation steps for NuGet, npm editor assets, Visual Studio Marketplace, and Open VSX

#### Scenario: Maintainer repairs or rolls forward a release
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL describe how to repair a partial registry publish using the same
  package artifacts
- **AND** it SHALL describe the rollback posture for immutable registries, including publishing a
  higher-version fix and unlisting or deprecating bad versions where supported
