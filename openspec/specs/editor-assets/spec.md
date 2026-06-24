# editor-assets Specification

## Purpose

Define how NX publishes reusable editor language assets so JavaScript and TypeScript consumers can
import NX grammars, language configuration, and snippets without depending on an NX source checkout,
and how those assets relate to the VS Code extension packaging workflow.

## Requirements

### Requirement: NX editor assets are published as an npm package
NX SHALL publish reusable editor language assets as the `@nx-lang/language` npm package generated
from the repository's editor asset source. The package SHALL allow JavaScript and TypeScript
consumers to import NX language assets without depending on an NX source checkout or a file-based
package dependency.

#### Scenario: Web editor consumer installs editor assets package
- **WHEN** a JavaScript application installs the published `@nx-lang/language` npm package
- **THEN** the application SHALL be able to import the NX TextMate grammar from
  `@nx-lang/language/grammar`
- **AND** the application SHALL be able to import the NX markdown code-block grammar from
  `@nx-lang/language/markdown-codeblock-grammar`
- **AND** the application SHALL be able to import the NX language configuration from
  `@nx-lang/language/language-configuration`
- **AND** the application SHALL be able to import the NX snippets from `@nx-lang/language/snippets`
- **AND** the application SHALL NOT need to reference `external/nx/src/vscode` or any other NX
  repository path

#### Scenario: Package contains reusable language assets
- **WHEN** the `@nx-lang/language` npm package is packed for release
- **THEN** the package SHALL include the NX TextMate grammar
- **AND** the package SHALL include the NX markdown code-block grammar
- **AND** the package SHALL include the NX language configuration
- **AND** the package SHALL include the NX snippets file
- **AND** the package SHALL NOT include VS Code extension runtime outputs or native `nx-lsp`
  binaries

#### Scenario: Browser editor bridges grammar into Monaco
- **WHEN** a browser editor such as Monaco/Shiki imports the `@nx-lang/language` package
- **THEN** the imported grammar and language configuration SHALL be usable as data assets by the
  consuming application's editor integration
- **AND** NX SHALL NOT require the consumer to install a VS Code extension to access those assets

### Requirement: Editor asset package generation is verified before publication
NX SHALL verify the `@nx-lang/language` package before publication. Verification SHALL prove that
the package can be generated, contains the expected files, and exposes the expected public package
exports.

#### Scenario: Grammar tests run before package publication
- **WHEN** the `@nx-lang/language` package is generated for release
- **THEN** the existing NX editor grammar tests SHALL run successfully before publication continues

#### Scenario: Packed package exposes expected exports
- **WHEN** package verification inspects the generated `@nx-lang/language` npm tarball
- **THEN** the tarball SHALL contain public exports for `@nx-lang/language/grammar`,
  `@nx-lang/language/markdown-codeblock-grammar`, `@nx-lang/language/language-configuration`, and
  `@nx-lang/language/snippets`

#### Scenario: Import smoke test uses packed package
- **WHEN** the `@nx-lang/language` package is packed locally or in CI
- **THEN** a smoke test SHALL import the public JSON exports from the packed package
- **AND** the smoke test SHALL NOT import assets from the NX repository source path

### Requirement: VS Code extension and editor asset packages are both published
NX SHALL publish the full VS Code extension through its VSIX workflow and SHALL publish reusable
editor language assets through the `@nx-lang/language` npm package. The two packages SHALL share
source assets but SHALL keep package names, runtime contents, and publication commands separate.

#### Scenario: VS Code extension package remains complete
- **WHEN** the VS Code extension workflow packages a VSIX for publication
- **THEN** the package SHALL use the `nx-language` extension identity
- **AND** the package SHALL include the compiled extension client runtime
- **AND** the package SHALL include the target-specific `nx-lsp` server asset

#### Scenario: Editor assets package stays runtime-free
- **WHEN** the release workflow publishes the `@nx-lang/language` npm package
- **THEN** the workflow SHALL pack the npm assets from a manifest or staging directory named
  `@nx-lang/language`
- **AND** the workflow SHALL NOT require VS Code extension runtime files or native `nx-lsp` binaries
  in the npm editor-assets tarball

#### Scenario: Editor assets documentation separates package outputs
- **WHEN** a consumer reads the editor-assets documentation
- **THEN** the documentation SHALL describe npm package consumption for reusable language assets
- **AND** it SHALL describe VS Code extension packaging and publishing as a separate output

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
