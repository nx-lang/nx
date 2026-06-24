## Context

`Publish packages` already separates NuGet/editor-assets registry writes from the `Build` workflow
by consuming artifacts from a successful Build run. The VS Code extension workflow still packages
and publishes in the same workflow on `main`, so a normal merge can fail because production
Marketplace/Open VSX credentials are missing or protected even when VSIX packaging succeeds.

The repository also has Rust binaries and libraries, but Rust tool publication is a separate release
surface. This change only prepares the package and VS Code extension publishing split and fixes the
workspace repository metadata URL.

## Goals / Non-Goals

**Goals:**
- Make `VS Code Extension` build, verify, and upload VSIX artifacts without writing to registries.
- Add a dedicated `Publish VS Code extension` workflow that publishes verified VSIX artifacts from a
  successful source workflow run.
- Keep `Publish packages` as the dedicated NuGet/editor-assets publish action.
- Update deployment documentation to describe the two explicit publish actions.
- Fix the Rust workspace repository URL to the public `nx-lang/nx` repository.

**Non-Goals:**
- Do not add Rust tool, Rust crate, or CLI binary publishing.
- Do not combine VSIX publishing into the NuGet/editor-assets publish workflow.
- Do not change VSIX package contents or extension versioning behavior beyond moving the publish
  boundary.

## Decisions

### Decision: use a dedicated VS Code extension publish workflow

Create a new workflow that triggers from `workflow_run` after `VS Code Extension` completes on
`main`, plus manual `workflow_dispatch` for repair from an explicit artifact run ID. This mirrors
`Publish packages` and gives Marketplace/Open VSX publication its own run history, environment
approval, and failure surface.

Alternative considered: keep publishing as a job inside `VS Code Extension`. That keeps the YAML
smaller, but merge builds fail when production credentials are absent or gated and it blurs package
verification with registry writes.

### Decision: validate source workflow runs before publishing

The publish workflow will confirm the source run completed successfully, came from
`.github/workflows/vscode-extension.yml`, came from this repository, and has expected VSIX
artifacts. Production publication will require a `main` source run.

Alternative considered: accept any manually supplied run ID. That would be easier to wire but risks
publishing artifacts from an unexpected workflow or branch.

### Decision: keep Rust tooling publication out of scope

Only update `Cargo.toml` package metadata in this change. Publishing `nxlang`, `nx-lsp`, or Rust
crates needs separate decisions around crates.io support, GitHub Release binaries, semver, and
installer strategy.

Alternative considered: add a placeholder Rust publish workflow. That would make the release
pipeline look more complete but would create an unsupported public distribution surface too early.

## Risks / Trade-offs

- Duplicate workflow setup for Node/pnpm between package and publish workflows -> keep publish
  workflow focused on installing only the tools required for version checks and registry publish.
- Automatic `workflow_run` publishing can still require production environment approval or secrets
  -> document that production credentials belong to the publish workflow and that the package
  workflow remains green when publishing is blocked.
- Manual repair depends on artifact retention -> deployment docs should call out using a successful
  VS Code Extension run ID while artifacts are retained.
