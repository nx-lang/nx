## Why

The VS Code extension already has local package and publish scripts, but the repository does not
define a repeatable release path for publishing it to extension registries. This makes releases
easy to perform inconsistently and leaves versioning, package contents, credentials, and validation
as manual knowledge.

## What Changes

- Add a supported publishing workflow for the `src/vscode` extension.
- Ensure the extension can be packaged into a VSIX with predictable contents before publishing.
- Support publishing the same extension version to both the Visual Studio Marketplace and Open VSX.
- Add release documentation that covers version updates, changelog updates, local verification,
  required credentials, dry-run/package inspection, and publish commands.
- Keep the extension's language features unchanged; this change is about release operations and
  distributable package quality.

## Capabilities

### New Capabilities

- `vscode-extension-publishing`: Repeatable packaging and publishing of the NX VS Code extension to
  supported extension registries.

### Modified Capabilities

- None.

## Impact

- Affects `src/vscode/package.json`, `src/vscode/README.md`, `src/vscode/CHANGELOG.md`, package
  include/exclude configuration, and any release automation added for extension publishing.
- May add a `.vscodeignore` or equivalent package inspection support so generated VSIX artifacts do
  not include tests, samples, lockfiles, local editor settings, or other development-only files.
- Requires documented publisher credentials for `vsce` and `ovsx`, without storing tokens in the
  repository.
- Adds or updates verification steps that run the extension grammar tests and package inspection
  before publishing.
