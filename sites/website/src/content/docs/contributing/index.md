---
title: 'Contributing'
description: 'Build NX from source, run the nxlang command-line tool, and work on the compiler, tooling and this site.'
---

NX is developed in the open at [github.com/nx-lang/nx](https://github.com/nx-lang/nx). Issues and
pull requests are welcome.

## Building from source

You need:

- Rust, through [rustup](https://rustup.rs). The pinned toolchain in `rust-toolchain.toml`,
  `wasm32-wasip1` target included, installs itself on the first `cargo` command.
- Node.js 22.18 or later, with pnpm enabled through `corepack enable`.
- The .NET 10 SDK, only for the .NET binding.

Then, from a clone of the repository:

```bash
cargo build --workspace
cargo test --workspace
pnpm install
pnpm -r build
pnpm -r test
```

`pnpm -r build` also builds the WebAssembly compiler, which needs `clang`. The first build downloads
a pinned WASI sysroot into `.cache/`.

## The `nxlang` command-line tool

The CLI isn't published yet, so run it from your build:

```bash
cargo run -p nx-cli -- run examples/nx/hello.nx
```

`nxlang run <file>` evaluates the file's root and prints the value as NX text; `--format json`
prints JSON instead. `nxlang codegen` emits TypeScript, JavaScript or NX IR, and `nxlang typegen`
emits type definitions for another language. `nxlang --help` lists the rest.

## The VS Code extension

The extension lives in `src/vscode`, outside the pnpm workspace. To run it from source in a VS Code
development host:

```bash
cd src/vscode
pnpm install --frozen-lockfile
pnpm run vscode:launch
```

`pnpm run package:verify` builds a `.vsix` you can install with **Extensions: Install from VSIX**.
`src/vscode/README.md` has the details.

## This site

The website is an [Astro Starlight](https://starlight.astro.build) project in `sites/website`, and
its pages are the Markdown files under `src/content/docs/`:

```bash
pnpm --filter @nx-lang/website dev
```

Every `nx` code block on the site is compiled by `pnpm --filter @nx-lang/website test`, which CI
runs on every pull request. A block must compile on its own, unless its fence says otherwise:

- ` ```nx fragment ` builds on names declared elsewhere on the page, so it's checked for syntax
  only.
- ` ```nx invalid ` shows a form NX rejects, so it must produce at least one error.

The build also fails on a link to a page or heading that doesn't exist.
