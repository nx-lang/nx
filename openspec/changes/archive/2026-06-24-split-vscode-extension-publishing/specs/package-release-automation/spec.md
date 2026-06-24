## ADDED Requirements

### Requirement: Release publishing is split into explicit package and extension actions
NX SHALL expose separate explicit publish workflows for package-registry publication and VS Code
extension-registry publication. Build and packaging workflows SHALL produce verified artifacts, and
publish workflows SHALL be the release-pipeline steps that write those artifacts to registries.

#### Scenario: Package publish action is separate from VS Code extension publish action
- **WHEN** a maintainer inspects the release pipeline workflows
- **THEN** the pipeline SHALL provide a `Publish packages` workflow for NuGet and editor-assets
  package registries
- **AND** it SHALL provide a separate `Publish VS Code extension` workflow for Visual Studio
  Marketplace and Open VSX publication
- **AND** each publish workflow SHALL consume artifacts from its corresponding successful
  build/package workflow run

#### Scenario: Build workflows do not require production registry credentials
- **WHEN** `Build` or `VS Code Extension` workflow runs verify package artifacts on pull requests or
  `main`
- **THEN** those workflows SHALL complete artifact verification without requiring production
  registry credentials
- **AND** production registry credentials SHALL be used only by explicit publish workflows that
  target the `production` environment

#### Scenario: Rust tool publishing is out of scope for this release pipeline
- **WHEN** a maintainer reads the package deployment runbook for this release pipeline
- **THEN** the runbook SHALL describe NuGet/editor-assets package publishing and VS Code extension
  publishing
- **AND** it SHALL NOT describe `nxlang`, `nx-lsp`, or Rust crate publication as part of this
  release pipeline
