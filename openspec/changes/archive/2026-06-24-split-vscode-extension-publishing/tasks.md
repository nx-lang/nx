## 1. Workflow Structure

- [x] 1.1 Remove registry publishing from `.github/workflows/vscode-extension.yml` so it only packages, verifies, and uploads VSIX artifacts.
- [x] 1.2 Add a dedicated `.github/workflows/vscode-extension-publish.yml` workflow that publishes VSIX artifacts from a successful `VS Code Extension` workflow run or an explicit manual source run ID.
- [x] 1.3 Validate source workflow runs and required VSIX artifacts before any Marketplace or Open VSX registry writes.

## 2. Metadata and Documentation

- [x] 2.1 Update the Rust workspace repository URL in `Cargo.toml` to `https://github.com/nx-lang/nx`.
- [x] 2.2 Update deployment setup documentation for the separate `Publish packages` and `Publish VS Code extension` production workflows and secrets.
- [x] 2.3 Update the deployment runbook with the two explicit publish actions and VS Code extension repair flow.
- [x] 2.4 Document that Rust tool publishing is intentionally not part of this release pipeline change.

## 3. Verification

- [x] 3.1 Run workflow linting for the changed GitHub Actions files.
- [x] 3.2 Run OpenSpec validation for `split-vscode-extension-publishing`.
- [x] 3.3 Review the resulting diff for accidental Rust tool publishing or unintentional registry writes from build/package workflows.
