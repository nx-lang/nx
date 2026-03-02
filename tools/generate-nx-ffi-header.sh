#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cbindgen >/dev/null 2>&1; then
  echo "cbindgen is required to generate bindings/c/nx.h. Install it with: cargo install cbindgen --locked" >&2
  exit 1
fi

cbindgen "${repo_root}/crates/nx-ffi" \
  --config "${repo_root}/crates/nx-ffi/cbindgen.toml" \
  --output "${repo_root}/bindings/c/nx.h"
