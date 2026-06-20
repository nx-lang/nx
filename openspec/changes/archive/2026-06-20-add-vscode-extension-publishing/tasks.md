## 1. Package Controls

- [x] 1.1 Update `src/vscode/package.json` scripts so maintainers can run tests, build a VSIX,
  inspect the packaged contents, publish a prebuilt VSIX to the Visual Studio Marketplace, publish a
  prebuilt VSIX to Open VSX, and publish to both registries without rebuilding separate packages.
- [x] 1.2 Add package include/exclude configuration for `src/vscode` so release VSIX files include
  only the manifest, README, changelog, license, language configuration, TextMate grammars, and
  snippets.
- [x] 1.3 Ensure generated VSIX files, tests, samples, lockfiles, workspace metadata, local editor
  settings, dependency directories, and other development-only files are excluded from packaged
  extension output.

## 2. Publishing Workflow

- [x] 2.1 Extend `.github/workflows/vscode-extension.yml` to extract the `src/vscode/package.json`
  version and fail tagged release runs when `github.ref_name` is not `vscode-v<package version>`.
- [x] 2.2 Update the tagged packaging path to run extension tests, package exactly one VSIX, inspect
  or list its contents, and upload that same VSIX as a workflow artifact.
- [x] 2.3 Add a tagged publish preflight that fails before registry publishing when either
  `VSCE_PAT` or `OVSX_PAT` is missing.
- [x] 2.4 Add Visual Studio Marketplace publishing that uses the packaged VSIX artifact and the
  `VSCE_PAT` secret.
- [x] 2.5 Add Open VSX publishing that uses the same packaged VSIX artifact and the `OVSX_PAT`
  secret.
- [x] 2.6 Ensure pull request and ordinary `main` branch validation can run tests and package
  verification without registry secrets.

## 3. Maintainer Documentation

- [x] 3.1 Update `src/vscode/README.md` with the supported package manager setup, test command,
  package command, and package inspection command.
- [x] 3.2 Document the release preparation flow: update `src/vscode/package.json` version, update
  `src/vscode/CHANGELOG.md`, run verification, create a `vscode-v<version>` tag, and push the tag.
- [x] 3.3 Document required local environment variables and GitHub Actions secrets for Marketplace
  and Open VSX publishing without committing tokens.
- [x] 3.4 Document local repair commands for publishing an already-built VSIX to one registry if a
  CI publish step partially succeeds.

## 4. Verification

- [x] 4.1 Run the VS Code extension dependency install using the package manager version expected by
  `src/vscode/package.json`.
- [x] 4.2 Run the VS Code extension grammar test suite.
- [x] 4.3 Run the package verification command and confirm the listed VSIX contents satisfy the
  `vscode-extension-publishing` spec.
- [x] 4.4 Run `openspec status --change "add-vscode-extension-publishing"` and confirm all required
  proposal artifacts are complete.
