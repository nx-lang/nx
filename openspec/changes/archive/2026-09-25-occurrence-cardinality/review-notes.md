# Review notes

## 8.5 Whole-corpus diff against the pre-change baseline

Baseline: `nxlang` built from `c197f4e` (the last commit before this change), run as
`nxlang run --format nx|json` over every `.nx` file under `examples/`,
`sites/playground/src/examples/nx/`, `specs/ir-conformance/`, `src/vscode/samples/` and `docs/`
(58 files). After: the same run on this branch after tasks 8.2 to 8.4.

- Every file that parsed before still parses. No file gained a syntax error.
- Canonical output is byte-identical for every file that evaluated before, in both formats. No
  corpus program had produced a `null` field, so the omitted-key rule changed no output.
- One new corpus program, `specs/ir-conformance/occurrences`, evaluates and is pinned by the
  conformance tests.
- Diagnostics changed only in their type text: `DrawnNode[]` reads `DrawnNode+`,
  `SkiaLabel[]` reads `SkiaLabel+`, `object[]` reads `object+` (the playground examples, run
  without the catalog as an implicit import, report the same pre-existing errors as before).
  `examples/nx/complex.nx` lost its two "Expected identifier" errors because the rewrite
  replaced an unparseable `<:>completed</>` fragment; its remaining errors are the pre-existing
  missing imports and unsupported `.length`.
- **Accepted behaviour change:** `sites/playground/src/examples/nx/keyboard.nx`, run without the
  catalog, now fails with `Undefined variable: Vertical` where the baseline exited 0 and printed
  `Orientation=null`. The baseline interpreter's catch-all evaluated an unresolved contextual name
  to `null`; there is no `null` to fall back to now, and an unresolved name is an error. With the
  catalog (`pnpm run check-examples` in `sites/playground`) the example compiles and evaluates.
- No new `+`-site rejection appeared in the corpus: every sequence property that the rewrite
  spelled `T+` is fed at least one item everywhere it is constructed.
- `src/vscode/samples/tally-survey.nx` is a highlighting sketch that has never parsed (nested
  `let`, bracketed lists, sixty-odd pre-existing syntax errors). Removing its ternary line shifted
  the parser's error recovery, so it reports a few more syntax errors after line 193; none of them
  concerns an occurrence spelling, and the sample is kept as a sketch.
