# Validation

Implementation commits: `0eee4f3` (migration/adapters), `d949683` (separate interpreter deletion).
Independent Claude findings and verification are recorded in `review.md`.

| Check | Result |
| --- | --- |
| Workspace build | Passed |
| Workspace tests | 3,434 passed, 20 ignored across 93 suites; includes compiler, runtime corpus/damage, API, CLI, FFI, language services and doc tests |
| Workspace clippy, all targets, warnings denied | Passed |
| Workspace formatting | Passed |
| Crate-layering guard and injected-edge tests | Passed; 12 guard tests |
| Source semantics migration | 384 passed; 15 ignored: six user-approved compiler follow-ups and nine original array-parser cases |
| Rich runtime view and shared formatting | Nine targeted tests passed, including constrained float32 sites; also included in the final workspace run |
| Generated code / IR corpus | Passed; 297 tests |
| CLI | 272 passed, including operation budget/reset, missing root, imports, round trips and located/source-free runtime errors |
| Node SDK | 36 passed; native and TypeScript builds passed |
| WASM SDK | 127 passed, module build and TypeScript checks passed; intentional trap test emits expected panic text |
| .NET SDK | 92 passed with native ABI 14; build has zero warnings/errors |
| TypeScript IR runtime | Full test script passed (runtime, corpus/damage, emitted IR, math, color, graphics, geometry and benchmark-driver checks) |
| Value-view fixtures | 38 tests passed; all six fixture payloads unchanged |
| Website | 13 tests and 251 NX blocks passed |
| Package export/install consumers | All ten checked packages passed after final review fixes |
| Playground | Type checking passed; 156 tests and all 26 examples passed |
| Native graphics / preview | 13 tests passed with `--features preview` |
| Rust runtime WASI and Emscripten | Both target checks passed |
| Packaged Rust runtime consumer | Passed in isolated checkout; host builds from nx-ir/nx-value/nx-ir-runtime packages without compiler crates |
| Main OpenSpec specs | 95 strict validations passed |
| Retirement and compiler follow-up proposals | Both strict validations passed |

The package export/install check initially exposed a pre-existing Corepack toolchain mismatch in
its scratch consumer. The consumer is now pinned to the repository's declared pnpm version, rather
than inheriting the global default which ignored local dependency overrides. The repeat export/install verification passed all ten checked packages.

Before deletion, the complete original interpreter suite and the original API differential suite
passed. Per-case inventories and the exact reviewed numeric-only corpus differences are in
`implementation-audit.md` and the three `*test-inventory.json` reports. Test-only helpers compose
compiler and runtime; no production source-evaluation API was kept for tests.

The full recursive JavaScript test run has one remaining failure: the unchanged
`packages/language-http` oversized-upload listener test reports `ECONNRESET` (22/23 pass).
The same compiled test fails in the original `be7f870` checkout, and the original/current compiled
HTTP handlers have identical SHA-256 hashes. The isolated listener test can pass; the full suite
reproduces the upload-close race. This is recorded as a baseline failure rather than changing
unrelated HTTP behavior here. Every other recursive suite passed; the playground was rerun after
the two reviewed formatting expectation updates.

Those updates affect only `graphics.out.nx` and `gradients.out.nx`: constrained float64 sites now
print a whole float as `1.0`, rather than `1`, because the formatter resolves their base type.
Token-by-token Decimal normalization proves the values and structure are identical. Claude
independently confirmed the spelling-only diffs. This is separate from the reviewed IR corpus
join-carrier spelling differences documented in the migration guide.

Performance: fifteen interleaved fresh processes per fixture, same debug profile, original and
final binaries. Tiny source: 3.64 ms HIR versus 4.05 ms IR; 300-declaration source: 41.15 ms versus
47.95 ms; standard-math import: 9.54 ms versus 11.13 ms. All outputs match; ratios are 1.11x–1.17x,
below 2x. The initial measurements remain in `implementation-audit.md`; these local timings vary
with machine load and are not a release performance guarantee. wasm-release SDK: 4,582,516 bytes
before versus 4,334,049 after final rebuild (about 5.4% smaller).

Independent review: one Claude CLI reviewer performed the implementation review and two subsequent
verification passes in the same independent session. All eleven findings are verified; none remain
open. Both `--skip-specs` archive and normal synchronization were exercised in scratch copies;
main specs are already synchronized and remain byte-identical on the latter check (apart from the
intentionally retired capability, whose deletion is preserved by the final `--skip-specs` archive).

Full logs, source fixture generators, baselines, timing data and independent Claude transcripts are
under ignored `target/retire-validation/`.
