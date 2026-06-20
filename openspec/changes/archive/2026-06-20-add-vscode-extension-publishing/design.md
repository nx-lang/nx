## Context

The VS Code extension lives in `src/vscode` and is already a standalone pnpm package with grammar
tests, `@vscode/vsce`, `ovsx`, local package/publish scripts, package metadata for publisher
`nx-lang`, and a `files` allowlist. The repository also has `.github/workflows/vscode-extension.yml`,
which runs tests for extension changes and packages a VSIX artifact for tags that start with
`vscode-v`.

The missing piece is a supported release contract. Today, a maintainer can run the local scripts,
but the repository does not define which checks must pass before publishing, how the tag should
relate to `src/vscode/package.json`, which registries are supported, how the generated VSIX should
be inspected, or which secrets/tokens are required. Because registry publishing is credentialed and
externally visible, the implementation should make release failures explicit and avoid storing
tokens in the repository.

## Goals / Non-Goals

**Goals:**

- Make the VS Code extension publishable through a documented, repeatable workflow.
- Package exactly one VSIX for a release and publish that same artifact to the Visual Studio
  Marketplace and Open VSX.
- Verify extension tests, version/tag alignment, and package contents before publishing.
- Document local maintainer commands for packaging, dry-run/package inspection, and registry
  publishing.
- Keep registry tokens in local environment variables or GitHub Actions secrets.

**Non-Goals:**

- Add or change VS Code language features, grammar behavior, snippets, or samples.
- Add a language server or compiled extension runtime.
- Publish the extension from the repository-wide NuGet release workflow.
- Store publisher credentials, generated VSIX files, or registry-specific release state in git.

## Decisions

### Extend the existing VS Code extension workflow

Use `.github/workflows/vscode-extension.yml` as the home for extension publishing instead of adding
publishing to `.github/workflows/release.yml`. The extension workflow already owns Node/pnpm setup,
grammar tests, path filters, and `vscode-v*` tag packaging, so extending it keeps extension release
behavior close to extension validation.

Alternative considered: publish from the repository-wide release workflow. That workflow is shaped
around native/.NET build artifacts and NuGet publishing, which would make the extension release path
depend on unrelated artifacts and secrets.

### Use `vscode-v<version>` tags as the release trigger

Continue using `vscode-v*` tags, but require the tag version to match `src/vscode/package.json`.
For example, package version `0.1.0` publishes from tag `vscode-v0.1.0`. This avoids accidental
publishing from ordinary `main` pushes and gives maintainers one obvious release identity shared by
Git, Marketplace, Open VSX, and the VSIX filename.

Alternative considered: publish from manual workflow dispatch only. Manual dispatch is useful for
reruns, but tags provide a clearer immutable release marker and match the existing packaging
workflow.

### Package once and publish the same VSIX everywhere

The workflow should build a VSIX once after tests pass, inspect the generated package contents, and
then publish that file to both registries. Local scripts should mirror that shape so maintainers can
run the same package and publish commands outside CI.

Alternative considered: run separate `vsce publish` and `ovsx publish` commands that each assemble
their own package. That risks registry drift if package contents or version resolution differ
between commands.

### Treat package contents as part of the release contract

The package should include only the extension manifest, README, changelog, license, language
configuration, grammars, and snippets. Development-only files such as tests, samples, lockfiles,
workspace metadata, local editor settings, generated VSIX files, and `node_modules` should stay out
of the published extension. The implementation should use the existing `files` allowlist plus
package-list verification; adding a `.vscodeignore` would make `vsce` switch to ignore-based
collection and bypass the `files` allowlist, so every development-only path would have to be
excluded separately. The release workflow must make unexpected package contents visible before
publishing.

Alternative considered: rely only on ad hoc reviewer inspection. That leaves a high chance of
shipping test fixtures or local artifacts after future package layout changes.

### Use explicit credential names

Document and use separate credentials for the two registries, such as `VSCE_PAT` for the Visual
Studio Marketplace and `OVSX_PAT` for Open VSX. CI publishing should fail before publishing if a
required token for the selected registry is missing, and local docs should show environment-variable
based commands without echoing or committing tokens.

Alternative considered: use one generic token variable. Separate names make registry failures and
secret configuration easier to diagnose.

## Risks / Trade-offs

- Registry metadata requirements diverge between Marketplace and Open VSX -> publish the same VSIX
  artifact to both registries and document any registry-specific token setup separately.
- The tag version and manifest version can drift -> add a workflow check that compares
  `vscode-v<version>` with `src/vscode/package.json`.
- Package content allowlists can become stale when new extension assets are added -> require package
  inspection in the publish path and document that new runtime assets must be included deliberately.
- CI publishing can partially succeed if one registry accepts the package and the other fails ->
  publish Marketplace and Open VSX in separate named steps so maintainers can see which registry
  needs a rerun or manual reconciliation.
- Local pnpm/Corepack versions can differ from CI -> keep CI pinned through the existing
  `packageManager`/setup-pnpm flow and document local setup before release commands.

## Migration Plan

1. Update extension package scripts and package include/exclude configuration so local packaging
   and package inspection are repeatable.
2. Extend `.github/workflows/vscode-extension.yml` to check tag/version alignment, package the VSIX,
   inspect/upload it, and publish it to configured registries on `vscode-v*` tags.
3. Update `src/vscode/README.md` with maintainer release steps, required token names, and local
   package/publish commands.
4. Validate the workflow in pull requests with tests and packaging checks that do not require
   registry tokens.

Rollback is operational: stop or delete the failing tag-triggered workflow run, remove or rotate
incorrect registry tokens if needed, and publish a corrected version with a new `vscode-v<version>`
tag. Published registry versions are immutable enough that rollback should prefer a follow-up patch
version over rewriting an existing version.

## Open Questions

- None.
