## Why

Merging to `main` currently runs the VS Code extension workflow and can attempt registry
publication from the same workflow run. That makes merge verification fail when production
Marketplace/Open VSX credentials are missing or gated, even though the package artifacts were built
successfully.

The release pipeline should keep artifact creation separate from registry writes for both package
publishing and VS Code extension publishing, while leaving Rust tool publication for a later
change.

## What Changes

- Split VS Code extension publication into a dedicated `Publish VS Code extension` workflow.
- Keep the existing `VS Code Extension` workflow responsible for building, verifying, and uploading
  VSIX artifacts only.
- Keep `Publish packages` as the separate NuGet/editor-assets publish workflow.
- Document the release pipeline as two explicit publish actions: `Publish packages` and
  `Publish VS Code extension`.
- Do not add a Rust tool publishing workflow in this change.
- Update workspace Rust package metadata so the repository URL points to `https://github.com/nx-lang/nx`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `package-release-automation`: clarify that release publication is composed of separate explicit
  publish workflows for package registries and VS Code extension registries, and that Rust tool
  publication is out of scope for this release pipeline step.
- `vscode-extension-publishing`: require VS Code registry publication to happen in a dedicated
  publish workflow that consumes verified VSIX artifacts from the packaging workflow.

## Impact

- `.github/workflows/vscode-extension.yml`
- New VS Code extension publish workflow under `.github/workflows/`
- `docs/deployment.md`
- `docs/deployment-setup.md`
- `Cargo.toml`
- OpenSpec release automation and VS Code extension publishing specs
