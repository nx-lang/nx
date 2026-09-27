## Purpose

The public NX website at `https://nxlang.org`: a landing page and the language documentation, with
the playground one click away and the DrawnUI fiddle linked as the place to see NX draw.

## ADDED Requirements

### Requirement: The site is served at the domain root
The website SHALL serve its landing page at `https://nxlang.org/` and its documentation pages at
paths under the domain root, with no base path. Every path under `/playground` SHALL be served by
the playground rather than by the website.

#### Scenario: Landing page at the root
- **WHEN** a visitor opens `https://nxlang.org/`
- **THEN** the landing page SHALL be served over TLS, with no redirect

#### Scenario: A documentation page at its root-relative path
- **WHEN** a visitor opens `https://nxlang.org/language-tour/types/`
- **THEN** the Types page of the language tour SHALL be served

#### Scenario: The playground keeps its prefix
- **WHEN** a visitor opens `https://nxlang.org/playground` or any path under `/playground/`
- **THEN** the playground SHALL answer, not the website

#### Scenario: Plain HTTP and `www` go to the canonical address
- **WHEN** a visitor opens `http://nxlang.org/<path>` or `https://www.nxlang.org/<path>`
- **THEN** they SHALL be redirected to `https://nxlang.org/<path>`

#### Scenario: An unknown path
- **WHEN** a visitor opens a path outside `/playground` that names no page
- **THEN** the website SHALL answer with status 404 and its own not-found page, which carries the
  site header and a link to the documentation

### Requirement: The landing page introduces NX and offers three ways in
The landing page SHALL say what NX is in one sentence and SHALL show a short NX snippet next to the
value it evaluates to. It SHALL offer three ways in: the documentation, the playground, and NX
drawing interfaces in the DrawnUI fiddle.

#### Scenario: The hero
- **WHEN** a visitor opens the landing page
- **THEN** the first screen SHALL show the one-sentence description, the snippet and its evaluated
  value, a primary action that opens the documentation's Getting Started page, and a secondary
  action that opens the playground

#### Scenario: The fiddle link
- **WHEN** a visitor opens the landing page
- **THEN** it SHALL link to `https://fiddle.drawnui.net`, described as where to see NX render
  graphics with DrawnUI

#### Scenario: The landing snippet is real
- **WHEN** the site's documentation check runs
- **THEN** the landing page's snippet SHALL compile without errors

### Requirement: Every page carries the site header
Every page of the website SHALL carry one header that names NX, searches the documentation, and
links to the documentation, the playground and the GitHub repository.

#### Scenario: Header links
- **WHEN** a visitor opens any website page, the not-found page among them
- **THEN** the header SHALL link to the documentation, to `/playground`, and to
  `https://github.com/nx-lang/nx`

#### Scenario: Search
- **WHEN** a visitor searches from the header for a term that appears on a documentation page
- **THEN** that page SHALL be among the results, with no request leaving the site

### Requirement: Documentation code blocks are checked
Every code block fenced as `nx` in the website's content SHALL be checked against the current NX
compiler whenever the repository's CI runs. By default a block SHALL compile without errors. A
fence may mark the block `fragment`, in which case it SHALL have no syntax errors, or `invalid`, in
which case it SHALL produce at least one error. A block that fails its check SHALL fail CI and name
the page and the block's line.

#### Scenario: A complete example compiles
- **WHEN** a page holds an unmarked `nx` block declaring `type User = { id:string name:string }`
- **THEN** the check SHALL pass for that block

#### Scenario: A stale example fails the build
- **WHEN** a page holds an unmarked `nx` block declaring `type <User id:string/>`
- **THEN** the check SHALL fail, naming the page and the line the block starts on

#### Scenario: A fragment is checked for syntax only
- **WHEN** a page holds an `nx fragment` block that references a type declared elsewhere on the page
- **THEN** the check SHALL pass as long as the block has no syntax errors

#### Scenario: An example of an error must still be an error
- **WHEN** a page holds an `nx invalid` block, and the compiler reports no error for it
- **THEN** the check SHALL fail for that block

### Requirement: Internal links are checked
The website build SHALL fail when a link from one page of the site to another, or to a heading
anchor on another page, does not resolve.

#### Scenario: A link to a missing page
- **WHEN** a page links to `/reference/syntax/nope`
- **THEN** the build SHALL fail, naming the page and the link

#### Scenario: A link to a missing heading
- **WHEN** a page links to `/reference/syntax/types#no-such-heading`
- **THEN** the build SHALL fail, naming the page and the link

### Requirement: Getting Started is written for someone using NX
The Getting Started page SHALL take a reader from nothing to running NX without building this
repository. It SHALL cover the playground, installing the VS Code extension from its registries,
and using NX from .NET and from JavaScript through the published packages. The command-line tool, which is not published yet, and
building NX from source SHALL be documented in the Contributing section, and Getting Started SHALL
point there.

#### Scenario: No toolchain is needed to start
- **WHEN** a reader follows Getting Started from the top
- **THEN** the first step SHALL be opening an example in the playground
- **AND** no step before the "from source" pointer SHALL require Rust, Cargo or a clone of the
  repository

#### Scenario: Published packages are named
- **WHEN** a reader reaches the .NET or JavaScript section
- **THEN** it SHALL name the published package (`NxLang.Sdk`, `@nx-lang/sdk-wasm`) and show the
  install command

#### Scenario: The editor extension is installed, not built
- **WHEN** a reader reaches the editor section
- **THEN** it SHALL link to the extension's Visual Studio Marketplace and Open VSX pages and name
  its identifier, `nx-lang.nx-language`
- **AND** it SHALL NOT ask the reader to build the extension

### Requirement: The site identifies itself when shared
Every website page SHALL declare a title, a description, a favicon and a social card image, so
that a link to it previews as NX in chat and social apps.

#### Scenario: Link preview metadata
- **WHEN** any website page's HTML is fetched
- **THEN** it SHALL carry `og:title`, `og:description` and `og:image`, where the image is an
  absolute `https://nxlang.org/` URL that answers with an image

### Requirement: The documentation is offered in a form language models read
The website SHALL publish `/llms.txt`, which indexes the documentation, and a single plain-text file
of the whole documentation, both regenerated on every build.

#### Scenario: llms.txt
- **WHEN** `https://nxlang.org/llms.txt` is requested
- **THEN** it SHALL answer with a Markdown index that links every documentation section, and the
  full-text file

### Requirement: The site deploys as static files from main
The website SHALL be built entirely to static files, and a push to `main` that touches the website
SHALL deploy it. Deploying it SHALL NOT require building the Rust workspace. A build that fails,
a broken internal link among the causes, SHALL leave the live site as it was. The code-block check,
which needs the compiler, SHALL run in the repository's CI for every pull request rather than in the
deploy. The deployment's configuration, meaning the routes it
serves and how unknown paths are answered, SHALL be committed in the repository.

#### Scenario: A docs edit deploys
- **WHEN** a commit that changes only a documentation page lands on `main`
- **THEN** the website SHALL be rebuilt and deployed, and the change SHALL be live at
  `https://nxlang.org` without the playground being redeployed

#### Scenario: A broken build does not deploy
- **WHEN** the website build fails, a link check among the causes
- **THEN** no deployment SHALL happen and the previous site SHALL keep serving

#### Scenario: The deploy is verified
- **WHEN** a deployment finishes
- **THEN** the workflow SHALL fetch `https://nxlang.org/` and a documentation page, and fail if
  either does not answer with the new build

### Requirement: The site's hosting is documented
The repository's deployment documentation SHALL describe how the website and the playground are
deployed, how to roll either back, and every by-hand setting the domain depends on: DNS records,
Worker routes, redirect rules, zone settings and secrets.

#### Scenario: A fresh account can be set up from the docs
- **WHEN** a maintainer sets the site up on a fresh Cloudflare account with only the repository and
  the deployment docs
- **THEN** the docs SHALL name every setting and secret needed for both sites to serve at
  `nxlang.org`

### Requirement: Published packages point at the site
Every package this repository publishes that declares a homepage SHALL declare
`https://nxlang.org`, and the repository README SHALL link to the site.

#### Scenario: npm metadata
- **WHEN** the manifest of a published `@nx-lang/*` package is inspected
- **THEN** its `homepage` SHALL be `https://nxlang.org`
