# One image: build the native addon, the SPA, and serve both from a single Node process.
#
# The native addon's .node binary is gitignored, so it is built from source here — which is why the
# build stage needs a Rust toolchain and the runtime stage does not.
FROM rust:1-bookworm AS build

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && apt-get install -y --no-install-recommends nodejs && rm -rf /var/lib/apt/lists/*

WORKDIR /repo

# The native binding and the IR runtime are consumed by relative path, so both come along.
COPY bindings/node ./bindings/node
COPY runtime/typescript ./runtime/typescript
COPY crates ./crates
COPY Cargo.toml Cargo.lock ./
COPY src/vscode/syntaxes ./src/vscode/syntaxes

# The IR runtime first: the native binding's own type-checking references its build output.
RUN cd runtime/typescript && npm ci && npm run build
RUN cd bindings/node && npm ci && npm run build

COPY sample-apps/drawnui-react ./sample-apps/drawnui-react
WORKDIR /repo/sample-apps/drawnui-react
RUN npm ci && npm run build

FROM node:22-bookworm-slim AS runtime
WORKDIR /repo

COPY --from=build /repo/bindings/node ./bindings/node
COPY --from=build /repo/runtime/typescript ./runtime/typescript
COPY --from=build /repo/sample-apps/drawnui-react ./sample-apps/drawnui-react

WORKDIR /repo/sample-apps/drawnui-react
ENV PORT=8080
EXPOSE 8080
CMD ["node", "server/index.mjs"]
