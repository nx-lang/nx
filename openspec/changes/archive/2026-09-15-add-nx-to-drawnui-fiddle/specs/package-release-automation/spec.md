## ADDED Requirements

### Requirement: Workspace npm packages ship on the package release track
The package release track SHALL treat the workspace's publishable npm packages
(`@nx-lang/sdk-wasm`, `@nx-lang/ir-runtime`, `@nx-lang/language-core`, `@nx-lang/language-protocol`,
`@nx-lang/language-client` and `@nx-lang/monaco`) the same way it treats the `@nx-lang/language`
editor-assets package: built and verified on pull request and `main` builds, packed at the release
tag's version, attached to the draft GitHub Release, and published to npm from the `production`
environment when the release is published. Their versions SHALL be the release tag's version, and
workspace references between them SHALL resolve to that version in the packed artifacts.

#### Scenario: Pull request builds pack every npm package
- **WHEN** a pull request build runs for package-related changes
- **THEN** CI SHALL pack and verify a `.tgz` for each workspace npm package alongside the
  editor-assets `.tgz`, and SHALL upload them for inspection without publishing

#### Scenario: A release tag attaches every npm package
- **WHEN** a maintainer pushes a valid compiler package release tag
- **THEN** CI SHALL attach one verified `.tgz` per workspace npm package to the draft GitHub
  Release, each carrying the tag's version and, for `@nx-lang/sdk-wasm`, the built module

#### Scenario: Publishing a release publishes every npm package
- **WHEN** a human publishes the GitHub Release
- **THEN** the publish workflow SHALL publish each attached `.tgz` to npm in dependency order,
  skipping any version already on the registry as an idempotent retry

#### Scenario: A member a consumer could not install is not on the track
- **WHEN** a workspace member depends on a package that is not published, as
  `@nx-lang/language-http` depends on the private `@nx-lang/sdk-node`
- **THEN** it SHALL be marked `private`, so the track leaves it out rather than publishing a package
  whose dependency cannot be installed

#### Scenario: Workspace references are resolved in the artifacts
- **WHEN** a packed package depended on another workspace package through a workspace reference
- **THEN** the packed manifest SHALL name the release version instead of the workspace reference
