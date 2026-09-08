# One image for the NX Playground: build the native addon, the shared packages and the SPA, and
# serve them from a single Node process. Build it from the repository root, since the context must
# hold every workspace member the site depends on.
#
# The native addon's .node binary is gitignored, so it is built from source here — which is why the
# build stage needs a Rust toolchain and the runtime stage does not. The site is a member of the
# repository's pnpm workspace, so the workspace is installed once at the root and built in
# dependency order. The runtime stage takes the built workspace with its development dependencies
# pruned, rather than `pnpm deploy`, because `@nx-lang/language` is not on the registry yet and
# deploy re-resolves peer dependencies there instead of honoring the workspace's path link.
FROM rust:1-bookworm AS build

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
  && apt-get install -y --no-install-recommends nodejs \
  && rm -rf /var/lib/apt/lists/* \
  && corepack enable

WORKDIR /repo

# The workspace manifests and every member the site depends on. `src/vscode` supplies the published
# grammar and language configuration, which the shared Monaco package links by path until
# `@nx-lang/language` is on the registry.
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY packages ./packages
COPY bindings/node ./bindings/node
COPY runtime/typescript ./runtime/typescript
COPY crates ./crates
COPY Cargo.toml Cargo.lock ./
COPY src/vscode/package.json src/vscode/language-configuration.json src/vscode/README.md src/vscode/CHANGELOG.md src/vscode/LICENSE ./src/vscode/
COPY src/vscode/syntaxes ./src/vscode/syntaxes
COPY src/vscode/snippets ./src/vscode/snippets
COPY sites/playground ./sites/playground

RUN pnpm install --frozen-lockfile
RUN pnpm -r build
# Only what the server needs at runtime: the Rust sources and build tree go, and so do the
# development dependencies (a production install re-links only those). The workspace's own links
# are relative, so the tree moves as one.
RUN pnpm install --prod --frozen-lockfile --config.confirmModulesPurge=false && rm -rf target crates Cargo.toml Cargo.lock

FROM node:22-bookworm-slim AS runtime
WORKDIR /repo

COPY --from=build /repo /repo

WORKDIR /repo/sites/playground
ENV PORT=8080
EXPOSE 8080
CMD ["node", "server/index.mjs"]
