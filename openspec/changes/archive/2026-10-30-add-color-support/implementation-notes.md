# Implementation and validation

Implemented the 26 tasks for the canonical float32 color library, typed hex literals, standard
color function bodies, editor metadata and edits, and graphics-core-v2 integration.

The canonical declarations and constants live in `@nx/color/color.nx`; external operations live
in `@nx/color/operations.nx`. This split lets literal-only images link without standard-color-v1.
The registry permits sibling imports only between statically registered modules of the same
standard library. Color operation bodies use IR node 27; ordinary color values remain records.

JSON decoding enables serde_json's float_roundtrip feature so float32 values widened to doubles
survive the JavaScript-to-WASM JSON bridge without an adjacent-double parsing error. Intermediate
Rust color argument conversions do not charge host-output operation costs; the conformance corpus
checks that TypeScript and Rust have identical evaluation counts.

## Validation

- `cargo test -p nx-api -p nx-codegen -p nx-types -p nx-hir -p nx-syntax -p nx-ir -p nx-ir-runtime -p nx-value --lib`: passed, including 458 API, 388 codegen, and all affected compiler/value/IR tests.
- `cargo test -p nx-language-service`: 311 passed.
- `cargo test -p nx-graphics --lib`: 19 passed.
- `cargo test -p nx-graphics-skia --features native --lib`: 8 passed on Linux.
- `cargo build -p nx-graphics-skia --features preview --bin nx-graphics-preview` and `python3 scripts/check-graphics-preview.py`: passed. Revision fixtures were regenerated for v2.
- `pnpm --dir runtime/typescript test`: passed, including the shared 253-case color reference corpus, binary conformance corpus, emitted-code checks, and runtime benchmark harness checks.
- `pnpm --dir bindings/wasm build` and `pnpm --dir bindings/wasm exec vitest run test/color-transport.test.ts`: passed.
- `pnpm --dir packages/viewer test`: 224 passed; `pnpm --dir packages/value-view test`: 38 passed; protocol and playground TypeScript builds passed.
- `pnpm --dir sites/playground test:graphics-browser`: passed in Chromium, including color-only editing, alpha, undo, computed colors, graphics pixels/selection/limits, and component recovery. One initial run timed out in the deliberate worker-trap recovery scenario; the full retry passed.
- `node --experimental-transform-types --test sites/playground/src/graphics/graphics.test.mjs`: 15 passed.
- `pnpm --dir sites/playground check-examples`: expected outputs regenerated; all 25 examples checked.
- `pnpm --dir sites/website check:code-blocks`: all 227 NX snippets passed.
- `openspec validate add-color-support --strict` and `git diff --check`: passed.

The new IR corpus fixture uses argument-bearing entrypoint cases to exercise the supported IR
runtimes without requiring legacy HIR interpreter color evaluation. Corpus regeneration must run
with `--test-threads=1`, because validation tests otherwise race fixture writers.

External ReachMe adoption and native platforms other than Linux are outside this repository's
validated scope. No archive or commit was created by this implementation.

## Independent review and follow-up validation

Claude CLI ran the independent review and continued the same session for fix verification.
The initial report contained RF1–RF14. Its first verification confirmed 13 fixes, reopened RF6
for two remaining source-formatting paths, and added RF15 for trailing-dot CSS numbers. Both
follow-ups were fixed with exact-output/source assertions and additional negative parser cases.
See review.md for the independent verification statuses.

Follow-up checks include 459 API tests, 389 codegen tests, the full language-service suite,
224 viewer tests, 11 focused color source-edit tests, 253 shared Rust/TypeScript reference cases,
and explicit HIR unsupported-runtime, public diagnostic naming, and default-alpha graphics tests.
The corpus now covers all 148 CSS names and compares direct decoding/RGB construction by exact
float32 bits; nonlinear transformations retain the documented 2e-6 tolerance.

The second Claude verification confirmed RF6 and RF15. All 15 findings are Verified, with no open findings or new issues. The final WASM rebuild, 227 documentation snippets, and color transport binding test passed after those fixes.

## Source formatting follow-up

Added the published NX Style Guide under `reference/style-guide.md` and linked it from AGENTS.md
and contributor documentation. Formatted both new color library modules and the color conformance
source with 2-space indentation, declaration spacing, and a 100-character line-length target.
API comments now describe behavior and channel units. Numeric constant values are unchanged.

Regenerated color conformance artifacts for updated source positions and fingerprints. Validation:
459 nx-api tests and 389 nx-codegen tests passed, including the regenerated corpus; all 233 website
NX snippets passed; the website build and internal link validation passed; `git diff --check` passed.

## Named color literals

The compiler registry now explicitly identifies the module defining the canonical Color type
when lowering the color library. Its literals resolve to that local type without synthesizing
a self-import. Other modules retain the protected canonical import, including authored modules
named color.nx or declaring an unrelated Color type.

All 149 constants (148 CSS names plus transparent) now use lowercase hex literals. Conversion
verified that every channel retained identical float32 bits. Regression tests compare every
constant with the CSS parser and cover trusted local resolution and authored type shadowing.
Updated conformance snapshots reflect the new source and literal expressions.

Validation passed: 459 nx-api, 391 nx-codegen, 186 nx-hir, and 317 nx-language-service tests;
the TypeScript IR conformance corpus; and `git diff --check`.

## Constant naming and grouping

Named color exports now use camelCase word boundaries, including lightSkyBlue, darkOliveGreen,
and rebeccaPurple. All 149 hex values are unchanged. The constants form one compact group with
a single shared comment. Updated the style guide to permit this layout for self-explanatory
constant groups, and updated API documentation and the change specification. CSS parsing keeps
its existing case-insensitive behavior.

Regenerated color conformance artifacts after the renames. Validation passed: 459 nx-api and
391 nx-codegen tests, including exact-channel comparisons for all named constants; the TypeScript
IR corpus; 234 documentation snippets; strict OpenSpec validation; and `git diff --check`.

## Follow-up review fixes and independent verification

Fixed RF16–RF22 through the OpenSpec fix/verify loop, resuming the same independent Claude CLI
reviewer session for each verification pass. All 22 review findings are now Verified, with none
open or awaiting verification. See review.md for reproduction details and independent checks.

- Re-exported aliases resolve in their defining namespace, including constructors and IR emission.
  Cross-module alias cycles report declaration-site errors, while valid same-name chains work.
- Mismatch diagnostics distinguish local and canonical Color identities, render foreign types
  without private import aliases, and exclude synthesized names from typo suggestions.
- Imported libraries retain dependency type-resolution context without exposing dependency names
  to consumer bindings. Graphics-only Paint.solid payloads reject strings, numbers, and unrelated
  local Color records, while valid canonical colors compile and evaluate.
- The style guide documents the blank line required when switching from trailing to leading
  documentation and includes a compiling example.

Final validation passed: 463 nx-api, 400 nx-codegen, 186 nx-hir, 317 nx-language-service, and
137 nx-types tests (1,503 total); 235 documentation snippets; website build and internal links;
strict OpenSpec validation; and `git diff --check`. Claude independently rebuilt the CLI and IR
probe, checked Rust IR and JavaScript alias behavior, and reran relevant Rust tests without
regenerating the corpus. No runtime/IR format extension was needed for these fixes.

The reviewer also recorded an existing JavaScript-target sequence-nesting difference involving
imported constants as an out-of-scope follow-up note, rather than an open finding for this change.

## Finalization

On 2026-10-30, synced color-library, color-literals, color-tooling, and graphics-library into
openspec/specs and archived this change. All 26 tasks are complete and all 22 review findings
are independently Verified. The separate JavaScript sequence-nesting issue is tracked in
specs/future.md, whose review link now points to this archive. All four synced specs passed
strict OpenSpec validation.
