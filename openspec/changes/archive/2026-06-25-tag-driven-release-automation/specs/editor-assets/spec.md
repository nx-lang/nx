## MODIFIED Requirements

### Requirement: Editor asset package generation is verified before publication
NX SHALL verify the `@nx-lang/language` package before publication. Verification SHALL prove that the
package can be generated with the MinVer-derived release or preview version, contains the expected
files, and exposes the expected public package exports.

#### Scenario: Grammar tests run before package publication
- **WHEN** the `@nx-lang/language` package is generated for release
- **THEN** the existing NX editor grammar tests SHALL run successfully before publication continues

#### Scenario: Packed package uses release pipeline version
- **WHEN** the `@nx-lang/language` package is generated in CI
- **THEN** the staged package manifest SHALL use the MinVer-derived or explicitly staged release
  pipeline version
- **AND** package generation SHALL NOT depend on `dotnet nbgv`

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
source assets but SHALL keep package names, runtime contents, version staging, and publication
commands separate.

#### Scenario: VS Code extension package remains complete
- **WHEN** the VS Code extension workflow packages a VSIX for publication
- **THEN** the package SHALL use the `nx-language` extension identity
- **AND** the package SHALL include the compiled extension client runtime
- **AND** the package SHALL include the target-specific `nx-lsp` server asset

#### Scenario: Editor assets package stays runtime-free
- **WHEN** the release workflow publishes the `@nx-lang/language` npm package
- **THEN** the workflow SHALL use the npm tarball attached to the corresponding compiler package
  GitHub Release
- **AND** the tarball SHALL be packed from a manifest or staging directory named `@nx-lang/language`
- **AND** the workflow SHALL NOT require VS Code extension runtime files or native `nx-lsp` binaries
  in the npm editor-assets tarball

#### Scenario: Pull request editor assets are testable without registry publication
- **WHEN** a pull request build generates the `@nx-lang/language` npm package
- **THEN** CI SHALL upload the tested `.tgz` artifact
- **AND** CI SHALL provide commands for installing that artifact directly from the downloaded tarball
- **AND** CI SHALL NOT publish the artifact to a preview npm registry

#### Scenario: Editor assets documentation separates package outputs
- **WHEN** a consumer reads the editor-assets documentation
- **THEN** the documentation SHALL describe npm package consumption for reusable language assets
- **AND** it SHALL describe VS Code extension packaging and publishing as a separate output
