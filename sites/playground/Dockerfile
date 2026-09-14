# One image for the NX Playground: build the WebAssembly compiler, the shared packages and the SPA,
# and serve the bundle from a single Node process. Build it from the repository root, since the
# context must hold every workspace member the site depends on.
#
# The compiler ships as a WebAssembly module built from the Rust crates, and it is gitignored, so it
# is built from source here — which is why the build stage needs a Rust toolchain, the wasm target
# and clang, and the runtime stage needs none of them. The site is a member of the repository's pnpm
# workspace, so the workspace is installed once at the root and built in dependency order. The
# runtime stage takes the built workspace with its development dependencies pruned, rather than
# `pnpm deploy`, because `@nx-lang/language` is not on the registry yet and deploy re-resolves peer
# dependencies there instead of honoring the workspace's path link.
FROM rust:1-bookworm AS build

# clang compiles tree-sitter's C runtime and the NX grammar for wasm32-wasip1, against the pinned
# WASI sysroot the build fetches for itself.
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
  && apt-get install -y --no-install-recommends nodejs clang \
  && rm -rf /var/lib/apt/lists/* \
  && corepack enable

WORKDIR /repo

# The workspace manifests and every member the site depends on. `src/vscode` supplies the published
# grammar and language configuration, which the shared Monaco package links by path until
# `@nx-lang/language` is on the registry.
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages ./packages
COPY bindings/wasm ./bindings/wasm
# The Node binding's crate, and nothing else of it: it is a member of the Cargo workspace, so cargo
# will not load the workspace without its manifest, and it has no package.json here so pnpm never
# sees it. It is never built — the runtime image carries no native addon.
COPY bindings/node/native ./bindings/node/native
COPY runtime/typescript ./runtime/typescript
COPY crates ./crates
COPY scripts ./scripts
# No Cargo.lock: the repository ignores it, so the checkout this image is built from has none and
# cargo resolves one here, as it does in the validate job that precedes a deploy. `rust-toolchain.toml`
# names the toolchain and brings the wasm target with it.
COPY Cargo.toml rust-toolchain.toml ./
COPY src/vscode/package.json src/vscode/language-configuration.json src/vscode/README.md src/vscode/CHANGELOG.md src/vscode/LICENSE ./src/vscode/
COPY src/vscode/syntaxes ./src/vscode/syntaxes
COPY src/vscode/snippets ./src/vscode/snippets
COPY sites/playground ./sites/playground

RUN pnpm install --frozen-lockfile
# Builds the wasm module (fetching the pinned sysroot on the way), the shared packages, then the
# site's bundle, which carries the module as a hashed asset. `--filter-prod` rather than `-r`: it
# selects the site and what it actually depends on, leaving out `packages/language-http` and the
# wasm SDK's own test-time dependency on the Node SDK — neither is in this context, and nothing in
# this image serves HTTP language queries or loads a native addon any more.
RUN pnpm --filter-prod "@nx-lang/playground..." build
# Only what the server needs at runtime — the built bundle and the Node process that serves it. The
# Rust sources, the build tree, the sysroot cache and the development dependencies all go (a
# production install re-links only those). `bindings/wasm` itself stays: the site's node_modules
# links to it, and the module it holds is already copied into the bundle as a hashed asset. The
# workspace's own links are relative, so the tree moves as one.
RUN pnpm install --prod --frozen-lockfile --config.confirmModulesPurge=false \
  && rm -rf target .cache crates Cargo.toml Cargo.lock rust-toolchain.toml bindings/node bindings/wasm/native

FROM node:22-bookworm-slim AS runtime
WORKDIR /repo

COPY --from=build /repo /repo

WORKDIR /repo/sites/playground
ENV PORT=8080
EXPOSE 8080
CMD ["node", "server/index.mjs"]
