# Review: add-color-support

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, implementation-notes.md, specs/color-library/spec.md, specs/color-literals/spec.md, specs/color-tooling/spec.md, specs/graphics-library/spec.md\
**Reviewed code:** crates/nx-value/src/color.rs, color-cases.json, color_corpus_tests.rs; runtime/typescript/src/color.ts, src/index.ts, test/color.test.mjs; crates/nx-api/src/std/color/{color,operations}.nx, artifacts.rs, nx_text.rs, std/graphics/graphics.nx; crates/nx-hir/src/lower.rs (+ ast/scope/components); crates/nx-types/src/infer.rs; crates/nx-syntax grammar.js/highlights/syntax_kind; crates/nx-ir (model/image/explain); crates/nx-ir-runtime/src/{eval,module}.rs; crates/nx-codegen/src/{ir.rs,color_transport_tests.rs,tests.rs}; crates/nx-language-service/src/{source_tree,source_edit,lib}.rs + source_edit_tests.rs; crates/nx-graphics/src/lib.rs; nx-graphics-skia tests; packages/viewer/src/{edit,editors,source,index}.ts + test/colors.test.ts; packages/value-view, language-protocol, bindings/wasm types; sites/playground check-graphics-browser.mjs, graphics/paths.ts, survey example; nx-grammar.md; color.md/graphics.md/design-token docs; specs/ir-conformance/color-library.\
**Independent checks:** a scratch Rust probe built outside the repo (compile, emit IR, link, evaluate on nx-ir-runtime, and call `nx_value::color::parse` directly), the TypeScript `dist/src/color.js` called directly from Node, `nxlang codegen --target javascript|nx-ir`, `nxlang ir explain`, and `nxlang run`. The working tree changed while I reviewed (corpus regeneration, TS color.ts, implementation notes). The findings below were rechecked against the files as they stood at about 22:30.

## Findings

### ✅ Verified - RF1 Rust `parse`/`tryParse` accept hex colors without a leading `#`, unlike CSS, the spec, and the TypeScript runtime
- **Severity:** High
- **Evidence:** `crates/nx-value/src/color.rs:140-145` hands every non-functional input to `csscolorparser`. Its `parse` falls back to "Hex format without prefix '#'" (`csscolorparser-0.7.2/src/parser.rs`, just before the named-color lookup). On Rust IR, `c.parse("bad")` returns `#bbaadd`, `c.parse("ff0000")` returns red, and `c.tryParse("cafe")` returns `ParseResult.parsed`. The TypeScript runtime rejects all three with `color-invalid-text`. This breaks the "same acceptance rules on both runtimes" requirement. It also feeds the language-service diagnostics and swatches, which call the Rust parser, so `parse("bad")` gets a swatch rather than a diagnostic. Ordinary words made of hex letters (`bad`, `fed`, `face`, `decade`) silently become colors.
- **Recommendation:** Stop delegating to csscolorparser's catch-all parser. Decode `#` + 3/4/6/8 hex digits explicitly, look names up in an explicit table (the 148-entry table already exists in TS and color.nx), and handle `transparent` separately. Add corpus cases for bare hex and hex-letter words.

- **Fix:** Rust now decodes only hash-prefixed hex explicitly and looks up names through the fixed CSS named-color table. Bare hex/word rejection is in the shared corpus.
- **Verification:** Verified. `color.rs:144-175` now handles `transparent`, the fixed named table, and only `#`-prefixed 3/4/6/8 hex. A differential run of 50 edge inputs through Rust `color::parse` and freshly compiled TS `color.js` agreed on every input: `bad`, `ff0000` and `cafe` are rejected on both, `#bad` is accepted. The corpus has `bad`/`ff0000`/`cafe`/`fed`/`face`/`decade` error cases.

### ✅ Verified - RF2 Rust hue wrapping can yield 360°, which picks the wrong HSV sector and diverges from TypeScript
- **Severity:** Medium
- **Evidence:** `color.rs:306-319`: `hsv` uses `h.rem_euclid(360.0) / 60.0` and `match h as u8`. For tiny negative hues, `rem_euclid` rounds to exactly `360.0`, giving sector 6, which falls through to `_ => [v, p, q]`. On Rust IR, `c.hsl(-1e-20, 1, 0.5)` returns `{r:1,g:0,b:1}` (magenta); the TypeScript runtime returns red (`colorHue` wraps through `% 360`). This affects the `hsl`/`hsv`/`hwb` constructors, which wrap only once. Hues computed as `toHsl(x).h - k` can easily land there. Spec: "Hue SHALL wrap finite degrees modulo 360"; parity is required within 2e-6.
- **Recommendation:** Normalize `if h >= 360.0 { h = 0.0 }` after `rem_euclid` (and the same in `hsl()`). Clamp the sector index, and add `hsl(-1e-20, 1, 0.5)` and `hsv(-1e-20, …)` to the shared corpus.

- **Fix:** Normalized Rust hue values that round to 360 back to zero before selecting HSV sectors; tiny negative hue constructors are regression-tested.
- **Verification:** Verified. `wrapped_hue` (`color.rs`) maps a `rem_euclid` result of 360 to 0 for `hsv`/`hsl`. On Rust IR, `c.hsl(-1e-20, 1, 0.5)` now returns red. The corpus covers tiny negative `hsl`/`hsv`/`hwb` hues.

### ✅ Verified - RF3 Rust and TypeScript CSS tokenization disagree on whitespace
- **Severity:** Medium
- **Evidence:**
  - Newlines inside a function: `color.ts:193` uses `/^(rgb|…|color)\((.*)\)$/`, and `.` does not match `\n`/`\r`, so TS rejects `"rgb(255\n0 0)"`. Rust (`split_ascii_whitespace`, `color.rs:192`) accepts it as red.
  - Vertical tab: `color.ts:172` trims `\v`, so TS accepts `"\vred"`. Rust's `is_ascii_whitespace` excludes VT, and the named path rejects the input (`color.rs:134,141`). The design says "Trim outer ASCII whitespace", and the Infra definition excludes VT.
  - Unicode whitespace: JS `\s`/`trim()` (color.ts `color(` prefix and modern split) differs from Rust's `char::is_whitespace` and ASCII splitting.

  The spec requires identical acceptance on both runtimes.
- **Recommendation:** Define one ASCII-whitespace class (TAB, LF, FF, CR, SPACE) and use it explicitly in both implementations: the `[^]`/`s` flag in the TS regex, no `\v`, and no `trim()`/`\s`. Add these inputs to `color-cases.json`.

- **Fix:** Both parsers now use TAB/LF/FF/CR/SPACE explicitly, accept function newlines, and reject VT/non-ASCII whitespace; shared cases cover each.
- **Verification:** Verified. Rust uses `is_ascii_whitespace`/`trim_ascii`. TS uses explicit TAB/LF/FF/CR/SPACE classes. Both runtimes accept `rgb(255\n0 0)` and `rgb(255\r\n0\f0)`, and both reject `\vred`, `red\v`, NBSP-prefixed `red`, and NBSP between channels.

### ✅ Verified - RF4 The shared corpus is too thin and too tolerant to prove parse/decoding parity
- **Severity:** Medium
- **Evidence:** `color-cases.json` has 70 cases, 24 of them parse cases. It has one hex shorthand (`#1234`) and no `#RGB`/`#RRGGBB`/`#RRGGBBAA` valid cases. It covers 2 of the 148 names and has no `grad`/`rad`/`deg`, legacy-comma `rgba()`/`hsla()`, whitespace variants, bare-hex, or rounding-boundary cases. Both runners compare every number with `<= 2e-6` (`color_corpus_tests.rs:37-40`, `color.test.mjs:7`), roughly 30 float32 ULPs near 0.5. The spec's "identical float32 channels" for equivalent input decoding is therefore never checked. Tasks 1.2/1.3 claim the corpus covers "all names, hex lengths, alpha order, whitespace, unsupported syntax… rounding boundaries". RF1–RF3 all got past it.
- **Recommendation:** Compare `parse`/constructor/literal outputs exactly (bitwise float32), and keep 2e-6 only for nonlinear conversions. Generate cases for every named color and each hex length/case. Add whitespace, hue-unit, legacy-comma, and negative cases (bare hex, hex-letter words, `\v`, newline).

- **Fix:** Expanded shared corpus to 248 cases including all 148 CSS names, all hex lengths, whitespace, hue units, legacy syntax, bare-hex and rounding boundaries. Direct decoding and RGB construction compare float32 bits exactly; nonlinear conversions retain the specified tolerance.
- **Verification:** Verified. The corpus has 248 cases, 197 of them parse. 167 `exact` cases (parse/rgb/rgba/srgb) compare float32 bits in Rust (`to_bits`) and with `Math.fround` deep equality in TS. The tolerance applies only to nonlinear cases. `cargo test -p nx-value --lib` passed 25 tests, and `node runtime/typescript/test/color.test.mjs` reports 248 passed; `dist` matches a fresh `tsc` build.

### ✅ Verified - RF5 Modern `hsl()`/`hwb()` reject CSS Color 4 `<number>` components, and the corpus pins that deviation as an error
- **Severity:** Medium
- **Evidence:** `color.rs:259-262` and `color.ts` (the `hsl`/`hwb` `%` check) require `%` on saturation/lightness and whiteness/blackness. CSS Color 4 modern syntax allows `<percentage> | <number>` there (`hsl(120 100 50)` is valid CSS). `color-cases.json:196` expects `"hsl(0 50 50)"` to fail. Also, `color( srgb 1 0 0)` with whitespace after `(` is rejected by both runtimes (`color.rs:150-151`; TS `^(srgb|srgb-linear)\s+`), although CSS permits it. The spec says parsing uses "CSS Color 4 numeric/percentage and hue-unit rules".
- **Recommendation:** Either accept numbers in modern HSL/HWB (and leading whitespace in `color()`) on both runtimes and fix the corpus case, or document the narrower subset explicitly in the spec and in color.md.

- **Fix:** Implemented modern numeric HSL/HWB components on the CSS 0..100 scale and leading ASCII whitespace inside color(); legacy comma HSL still requires percentages.
- **Verification:** Verified. Modern `hsl()`/`hwb()` accept numeric 0..100 components (`hsl(120 100 50)`, mixed `hsl(120 100% 50)`, `hwb(0 50 60)`), legacy comma HSL still requires percentages, and `color( srgb …)` and `color(\nsrgb … )` parse. Rust and TS gave identical results.

### ✅ Verified - RF6 Picker edits of constructors and records write widened-float32 decimals into source
- **Severity:** Medium
- **Evidence:** `source_edit.rs:907`, `:952` and `:959` write `channels[i].to_string()`, `(channels[i] * scale).to_string()` and `format!(", {}", channels[3])`. Here `channels` are float32 values widened to f64 from `parse(text)`. Picking `#5b…` on `c.rgb(...)` writes `91.00000217556953` instead of `91`. On `srgb(...)` it writes `0.35686275362968445`, appending alpha 0x80 writes `, 0.501960813999176`, and record fields get similar text. The existing tests (`source_edit_tests.rs`) only check unchanged channels or exactly representable values (`1`, `0.5`), so this is untested.
- **Recommendation:** Write rgb/rgba channels as the integer byte. Write srgb/record channels and alpha as the shortest decimal that rounds to the intended float32 (for example, format the `f32` with `{}`, or `byte/255` to the shortest f32-roundtrip digits). Add edit tests that change a non-trivial channel on rgb, rgba, srgb, and a record, and that append alpha.

- **Fix:** RGB edits write integer bytes; normalized fields and appended alpha write shortest f32-roundtrip decimals. Added constructor/record/comment tests and preserved untouched precision when editing parsed CSS colors.
- **Verification:** Reopened. RGB arguments are now integer bytes, and srgb/record channels are f32-shortest (a probe gave `c.rgb(91, 110, 245)` and `c.srgb(0.35686275, 0.43137255, 0.9607843, 0.5019608)`). Two paths still write widened float32 decimals.
  - `source_edit.rs:919`: appending a missing record alpha uses `channels[3].to_string()`. `<c.Color r=0 g=0 b=0/>` picked as `#5b6ef580` becomes `… a=0.501960813999176/>`. The test `color_channel_edits_use_short_source_numbers_and_preserve_comments` passes only because it asserts `contains("a=0.5019608")`, a prefix of the widened text.
  - `source_edit.rs:933-956`: the new `parse` path always rewrites the argument through `toCss`. `c.parse("#123456")` picked as `#5b6ef5` becomes `c.parse("rgba(91.00000217556953, 110.00000104308128, 245.00000059604645, 1)")`, and `c.parse("#33669980")` becomes `"rgba(17.00000088661909, …, 0.501960813999176)"`. The author's hex or name spelling is replaced by long widened CSS.

  Fix: use `(channels[3] as f32).to_string()` for the appended alpha. For `parse`, emit lowercase `#rrggbb`/`#rrggbbaa` when every merged channel is byte-representable (always true when the original was hex or a name), and fall back to `toCss` only when an untouched channel has sub-byte precision. Assert exact output in the tests.

- **Fix:** Follow-up fix: missing record alpha now formats the f32 directly. Parsed byte-representable colors emit compact lowercase six/eight-digit hex (retaining an existing explicit alpha suffix); only sub-byte merged values use CSS. Exact whole-source assertions cover appended alpha and four parse edits, and the precision-preservation regression remains passing.
- **Verification:** Verified (second pass). A probe through `WorkspaceSnapshot::source_edit` gave these outputs.
  - The missing record alpha is appended as `a=0.5019608`.
  - Byte-exact `parse` edits stay compact hex: `"#123456"`→`"#5b6ef5"`, `"red"`→`"#00ff00"`, `" RED "`→`"#ff000080"`.
  - An explicit alpha suffix is kept: `"#f008"`→`"#12345688"`, `"#f00f"`→`"#123456ff"`, `"#33669980"`→`"#11223380"`.
  - `color(srgb 0.123456789 0.4 0.8)` becomes `"#ff6666"`, because its untouched g/b float32 values equal 102/255 and 204/255.
  - Only a sub-byte untouched channel falls back to CSS: `rgb(100% 0% 0% / 50%)` with a new RGB keeps alpha as `"rgba(0, 255, 0, 0.5)"`.

  Constructor edits are unchanged and compact (`c.rgb(91, 110, 245)`, `c.srgb(0.35686275, …, 0.5019608)`). The record test now compares the whole source with `assert_eq!` (`source_edit_tests.rs:1502`), and `cargo test -p nx-language-service color` passed 11 tests.

### ✅ Verified - RF7 Static color diagnostics don't locate the offending channel, and malformed hex literals lack a `color-literal` code
- **Severity:** Medium
- **Evidence:**
  - `lib.rs:405-454` anchors each constructor diagnostic on the first `Literal` child of the call. For `c.rgb(1, 2, 300)` the error underlines `1`. Records have no literal children (their children are attributes), so the whole element is highlighted. The message names a generic parameter (`invalid color channel or parameter: rgb` / `channel`), not the field.
  - Hex literal errors are emitted via `add_diagnostic("color-literal: expected …")` (`lower.rs:1541`). They surface with code `lowering-error`; the probe showed `code: Some("lowering-error")`.

  The spec requires diagnostics that "locate the offending input" and use `color-literal` / `color-invalid-channel` codes. The design says to "point to the offending string/channel".
- **Recommendation:** Report the failing argument or attribute index from `nx_value::color` (or validate per argument during discovery) and anchor the label there. Give literal errors a real `color-literal` diagnostic code. Add tests asserting codes and spans for `rgb(1,2,300)`, `<c.Color r=2 …/>`, and `#12`.

- **Fix:** Constructor diagnostics identify r/g/b/a and select the corresponding argument/attribute literal. HIR color-literal failures now expose a real color-literal code; code/span tests cover rgb third argument, record r, and malformed hex.
- **Verification:** Verified. Rust errors name the field (`b`, `alpha`, `a`), and diagnostics anchor on the offending literal: `300` in `c.rgb(1, 2, 300)`, `2` in `srgb(…, 2)`, `5` for record `a=5`, `"bad"` for `parse`. Malformed hex now has code `color-literal` spanning `#ff0000oops`. The message still repeats a `color-literal:` prefix, which is cosmetic.

### ✅ Verified - RF8 `nxlang run` (HIR interpreter) fails on any color literal or constant with misleading errors, and the migrated survey example no longer runs there
- **Severity:** Medium
- **Evidence:**
  - `nxlang run` on `let root() = { #f00 }` prints `Runtime error: Record type not found: $nxColor.Color`.
  - `import "@nx/color" as c` + `{ c.red }` prints `Operation limit exceeded: 1000000 operations`.
  - `nxlang run sites/playground/src/examples/nx/survey.nx` fails with the same `$nxColor.Color` error, though it ran before the migration.
  - Only color *functions* get the explicit `color requires the IR runtime` message.

  The design excludes the interpreter but requires missing support to fail explicitly and documented entry points to be named. The CLI is the user-facing `run` path.
- **Recommendation:** Make literals and color-module constants fail in the interpreter with the same explicit IR-runtime diagnostic, or route `nxlang run` through IR for programs that link `@nx/color`. State in color.md which commands support color.

- **Fix:** HIR literals and canonical color constant evaluation now fail immediately with color requires the IR runtime; added regression tests and documented nxlang run and supported IR entry points.
- **Verification:** Verified. With the decision that the legacy HIR deliberately fails, `nxlang run` now fails explicitly with `color requires the IR runtime` for literals, `c.red`, selective `Color` imports, `parse`, and `survey.nx`. Programs that only import the library, and graphics programs without colors, still run. color.md documents that `nxlang run` uses the interpreter. The text is still wrapped as `Type mismatch in expression evaluation: expected a checked expression, got …`, which is awkward but explicit.

### ✅ Verified - RF9 Viewer alpha control now applies to legacy string `format=color` editors, which changes string visualizer behavior
- **Severity:** Medium
- **Evidence:** `packages/viewer/src/index.ts:2838-2857` adds the `.popover-alpha` input to every color popover, not just `colorLiteral` targets.
  - For a string value such as `"rgba(0, 0, 0, 0.5)"` or `"red"`, the initial alpha is computed via `preserveHexAlpha("#000000", input.value)`, which shows 1 for non-hex strings and so misreports alpha.
  - Changing alpha replaces the authored string with `"#rrggbbaa"`.

  The spec says "Existing string format=color visualizers SHALL retain their presentation behavior".
- **Recommendation:** Add the alpha control only when `target.colorLiteral === true`, or derive alpha correctly from string CSS and keep the string's spelling. Add a viewer test for a string `format=color` target.

- **Fix:** Restricted the alpha control to typed-color edit targets. Added a legacy rgba-string viewer test asserting no alpha control, unchanged spelling, and no edits on dismissal.
- **Verification:** Verified. `index.ts:2842` creates `.popover-alpha` only for `colorLiteral` targets. A viewer test covers a legacy `"rgba(0, 0, 0, 0.5)"` `format=color` string: no alpha control, spelling unchanged, no edit on Escape. The viewer suite passed 224 tests.

### ✅ Verified - RF10 Viewer and source-edit tests don't cover the tooling scenarios that tasks 5.3 and 6.2 claim
- **Severity:** Medium
- **Evidence:** `packages/viewer/test/colors.test.ts` has a single `preserveHexAlpha` unit test. No viewer tests cover open/close without an edit, explicit alpha changes, undo, stale revisions, or read-only computed values (only the browser script touches a subset). `source_edit_tests.rs` adds five color tests, with none for:
  - stale revisions on color edits
  - rgb/rgba edits that change channels
  - 3-arg `srgb` alpha append, and refusal for `rgb` + alpha
  - comments inside constructor calls
  - colors reached through the graphics `Color` re-export
  - `tryParse`/computed calls refusing edits
- **Recommendation:** Add these focused unit tests. They would also have caught RF6 and RF9.

- **Fix:** Added source-edit tests for changed RGB/normalized channels, comments, alpha append/refusal, removed stale keys, computed/tryParse refusal, and graphics re-exports. Added native viewer open/close, alpha commit, read-only computed, and legacy string tests; browser undo coverage remains in the real Chromium scenario.
- **Verification:** Verified. Source-edit tests cover RGB/normalized edits, comments, wrong-kind refusals, computed/tryParse refusal, stale keys (`UnknownNode`), the graphics re-export, and diagnostic spans. Viewer tests cover open/dismiss without edits, explicit alpha commits, read-only computed values, the legacy string editor, and a detached alpha control after a tree change. The record-alpha assertion is too loose to catch the RF6 residue, which is tracked under RF6.

### ✅ Verified - RF11 `SetValue { Color }` is accepted on any literal node, so a client can change a string or number literal into a typed color
- **Severity:** Low
- **Evidence:** `source_edit.rs:248-280` (`set_value`) checks only that the node is a whole `Literal`/`Case`. It then spells the `Color` value as `#rrggbb[aa]`, with no check that the target is a `COLOR_LITERAL`. A string literal `"#ff0000"` in a `string` field (for example, a legacy `format=color` string) can be rewritten to an unquoted color literal, which introduces a type error. The reverse (`String` onto a color literal) is likewise unchecked.
- **Recommendation:** Refuse `SourceEditValue::Color` unless the node's text is a color literal, and refuse non-color values on color-literal nodes.

- **Fix:** SetValue Color now refuses non-color literal targets and non-color values are refused for existing typed color literals; tests cover both directions.
- **Verification:** Verified. `source_edit.rs:259` refuses `Color` values on non-`#` literals and non-`Color` values on color literals. Tests cover a string literal, a number literal, and the reverse direction.

### ✅ Verified - RF12 The internal `$nxColor.Color` alias leaks into user-facing type diagnostics
- **Severity:** Low
- **Evidence:** `type Brand = { accent:string }` + `<Brand accent=#5b6ef5/>` reports `Record field 'accent' on 'Brand' expects string, found $nxColor.Color`. The alias comes from the synthesized import in `lower.rs` (record `$nxColor.Color`).
- **Recommendation:** Display the canonical type (`Color` or `@nx/color.Color`) in type-mismatch messages for synthesized literal records.

- **Fix:** Type display removes the unspellable synthesized qualifier from literal type names; API diagnostic test asserts found Color without $nxColor.
- **Verification:** Verified. The mismatch now reads `Record field 'accent' on 'Brand' expects string, found Color`.

### ✅ Verified - RF13 Graphics preparation rejects raw SDK colors that omit the defaulted alpha
- **Severity:** Low
- **Evidence:** `crates/nx-graphics/src/lib.rs:343` calls `nx_value::color::channels`, which requires an `a` property (`color.rs:124`, `invalid("a")`). `Color.a` defaults to 1, and typed IR boundaries fill it in (`color_transport_default_alpha_and_invalid_channels`), but a raw SDK scene record `{r,g,b}` fails with "invalid color channel: a". The graphics spec explicitly covers validating raw SDK records.
- **Recommendation:** Treat a missing `a` as 1 in `channels` (or in graphics preparation) and test that case.

- **Fix:** Shared validation uses the declared alpha default of 1 for a missing a field (including TypeScript parity); a raw SDK graphics record test passes.
- **Verification:** Verified. Rust `channels` and TS `colorChannels` default a missing `a` to 1. `raw_color_records_use_the_default_alpha` is in nx-graphics, and `cargo test -p nx-graphics --lib` passed 19 tests.

### ✅ Verified - RF14 nx-grammar.md puts `ColorLiteral` into the `SignedNumericLiteral` and `StringLiteral` productions
- **Severity:** Low
- **Evidence:** `nx-grammar.md:405` (`SignedNumericLiteral ::= ColorLiteral | "-" (...)`) and `:686` (`StringLiteral ::= ColorLiteral | '"' ...`). These contradict the implementation (`grammar.js` adds it only to `literal`) and the doc's own statement that quoted colors remain strings. The `StringLiteral` edit also misaligns the single-quoted alternative.
- **Recommendation:** Keep `ColorLiteral` only in `Literal` (line 503) and revert the other two insertions.

- **Fix:** Removed ColorLiteral from SignedNumericLiteral and StringLiteral, keeping it only in Literal and its own production.
- **Verification:** Verified. `ColorLiteral` now appears only in `Literal` and its own production. `StringLiteral` has its two quoted alternatives, now with the missing `|`.

## New Findings Discovered During 2026-10-29 23:40 Verification

### ✅ Verified - RF15 Both CSS parsers accept numbers with a trailing decimal point, which CSS does not allow
- **Severity:** Low
- **Evidence:** `rgb(255. 0 0)` parses as red on both Rust (`color.rs` `num` passes any `[0-9+-.e]` text to `f64::parse`, which accepts `255.`) and TypeScript (`color.ts` number regex `\d+\.?\d*`). The CSS Syntax `<number>` token requires at least one digit after a `.`. The two runtimes agree, so this is a conformance gap, not a parity one; the spec says parsing uses CSS Color 4 numeric rules.
- **Recommendation:** Require digits after `.` in both parsers (TS: `\d*\.\d+|\d+`; Rust: reject a trailing `.` or use an explicit token grammar), and add `rgb(255. 0 0)` to the error corpus.

- **Fix:** Rust rejects a decimal mantissa ending in a dot before an optional exponent; the TypeScript grammar requires digits after a decimal point. Added four shared negative cases for trailing dots in RGB, exponents, percentages, and hue units; all 253 corpus cases pass on both runtimes.
- **Verification:** Verified. A differential run of 28 inputs through Rust `color::parse` and a fresh TypeScript build (identical to `dist`) agreed on every input.
  - Rejected on both: `255.`, `1.e2`, `100.%`, `120.deg`, `1.rad`, `30.` in `oklch`, `/ 1.`, `100.` in `hsl`, `1..2`, `1e2.5`, `1e+`, `--1`, `+-1`.
  - Still accepted: `.5`, `+.5e1`, `1.5e+1`, `1e-1`, `255.0`, `1.5e2deg`, `1e2%`.

  The corpus has 253 cases; `cargo test -p nx-value --lib` passed and `color.test.mjs` reports 253 passed.

## Questions
- None

## Summary
- The core architecture is sound and matches the design. Color is an ordinary record. Literals lower to typed record constants that link only `color.nx`; I confirmed the entry image links `@nx/color/color.nx` without `standard-color-v1`. Operations are gated behind node 27 with feature and arity validation in both decoders. Graphics now consumes numerical channels with the `graphics-core-v2` profile. Same-name user `Color` types are not treated as canonical. Named-color tables match across Rust, TS, and NX (148 each). Premultiplied mix and linear composite results are correct.
- The main risks are Rust/TypeScript parsing parity (RF1–RF3), which a corpus that is too thin and too tolerant (RF4) let through, and source-editing quality (RF6, RF9, RF11) with thin tooling tests (RF10). RF1 is the one finding I'd fix before archiving. The others are moderate or polish.
- Verification (2026-10-29 23:40): 13 of 14 fixes verified, including the parity fixes RF1–RF5. RF6 is reopened: an appended record alpha and edits to `parse(...)` arguments still write widened float32 text. RF15 (trailing-dot CSS numbers) is new and low severity. RF6 is the remaining item before archiving.
- Verification (second pass): RF6 and RF15 are verified, so all 15 findings are now verified. No regressions in the earlier 13; the RF1–RF5 parser parity holds on the new inputs. Nothing is open.

## New Findings Discovered During 2026-10-30 01:20 Review

### Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/{color-library,color-literals,color-tooling,graphics-library}/spec.md, implementation-notes.md (sections "Source formatting follow-up", "Named color literals", "Constant naming and grouping"), and this review through RF15.\
**Reviewed follow-up code and docs:** sites/website/src/content/docs/reference/style-guide.md, and its links in AGENTS.md, CONTRIBUTING.md, and contributing/index.md; crates/nx-api/src/std/color/{color,operations}.nx; specs/ir-conformance/color-library/main.nx and its regenerated `expected/` images, text, origins, and operation snapshots; `LoweringContext.defines_color_type`, `lower_with_color_runtime`, and the `color_literal_tests` in crates/nx-hir/src/lower.rs; the `build_standard_library` call in crates/nx-api/src/artifacts.rs; `all_named_color_constants_match_css_parser_channels_exactly` and `authored_color_type_cannot_shadow_the_literal_type` in crates/nx-codegen/src/color_transport_tests.rs; color.md naming text; crates/nx-syntax/src/doc_comments.rs (unchanged, read to check how the trailing-comment rule behaves).\
**Checks performed:**
- Rebuilt `nxlang` from the current tree into a scratch target dir (`/tmp/nxrev/target`) and ran probes from `/tmp/nxrev`.
- Ran `cargo test -p nx-hir -p nx-api -p nx-codegen --lib` against that target dir, without `NX_UPDATE_CORPUS`: 186, 459 and 391 passed. Fixtures were not modified.
- Compiled the TypeScript runtime to a scratch outDir. It is identical to the tracked `dist/src`.
- Ran `node test/corpus.test.mjs`, which passed, including both `color-library` variants, and `node test/color.test.mjs`, which passed 253 cases.
- Confirmed that a literal-only entry image links only `@nx/color/color.nx`.
- Grepped the tree for lowercase compound constant names. The only stale `c.rebeccapurple` hits are in the gitignored `sites/website/dist` build output.
- Checked the trailing-comment layout and the 100-column limit in both color modules. No line exceeds 100 columns.
- Did not rerun the website snippet or link checks, the viewer and browser suites, or WASM.

### ✅ Verified - RF16 The graphics `Color` re-export does not work as an application type: hex literals, `c.parse` results and color-function arguments are rejected
- **Severity:** Medium
- **Evidence:** `graphics.nx:28` declares `export type Color = colors.Color`, where `colors` is graphics' own import alias. In an application, the alias target appears to be resolved in the importing module's scope, not graphics' scope. These fail on the current build:
  - `let a: g.Color = #5b6ef5` reports `expects colors.Color, found Color`.
  - `let a: g.Color = { c.parse("red") }` fails the same way.
  - `let a: g.Color = <c.Color r=1 g=0 b=0 />` reports `found c.Color`.
  - `type B = { accent: g.Color }` with `<B accent=#5b6ef5 />` fails.
  - `let f(x: g.Color): string = { c.toHex(x) }` reports `Argument 0 expects Color, found colors.Color`.

  Only `<g.Paint.solid color=…/>`, `{ c.red }` and `<g.Color …/>` construction work, and they are the only forms the tests use (`source_edit_tests.rs:1646`, graphics tests).

  The root cause is general. A workspace probe (`a.nx: export type P = { x: int }`, `b.nx: import "./a.nx" as base` + `export type Q = base.P`, then `let v: b.Q = <a.P x=1 />`) fails with `expects base.P, found a.P`. It passes only when both modules happen to use the same import alias. That error also has no source location.

  This predates the follow-up pass: the 22:44 binary behaves identically. It was not covered by RF1–RF15. It breaks the design rule that type identity is preserved "including through aliases/re-exports", the graphics requirement to "import and re-export the canonical Color", and the shared-color scenario as soon as an application annotates a value as `g.Color`.
- **Recommendation:** Resolve exported alias targets to declaration identities in the defining module's scope when building its interface, not by re-resolving the qualified text in importers. Add graphics regression tests for `g.Color` annotations with a literal, a `parse` result, and an argument to `c.toHex`. Add a workspace test of a re-exported alias with distinct import aliases. If the type-checker fix is deferred, track it explicitly and stop presenting `g.Color` as interchangeable until it is fixed.

- **Fix:** TypeAliasInfo now retains the defining module identity and resolves alias targets in that namespace. Record-definition walks also use the alias owner namespace and module-qualified cycle keys; constructor inference preserves the target nominal identity; codegen resolves alias constructor references before emitting typed records. Added graphics annotations/parse/field/function-argument regressions, a workspace alias-chain test with distinct import names and local shadowing, and an IR evaluation test across graphics/color imports, including a g.Color constructor.
- **Verification:** Verified, with RF19 as a regression introduced by this fix. I rebuilt `nxlang` and the scratch IR probe from current sources.
  - Every RF16 reproduction now compiles: `g.Color` annotations with a hex literal, `c.parse`, and `<c.Color …/>`; a `g.Color` record field with a literal; and a `g.Color` parameter passed to `c.toHex`. `c.Color = <g.Color …/>` and `g.Color?` also compile.
  - Evaluation matches on the Rust IR runtime and the JavaScript target: `#ff8000ff`, `#5b6ef580` and `#ff0000ff`. `<g.Color …/>` yields a record with `$type: "Color"`.
  - The general case is fixed too. A workspace where `a.nx` declares `P`, a string alias, and a union, `b.nx` (`import a as base`) aliases them, including `base.P*` and `base.P?`, and `c.nx` (`import b as mid`) aliases those again resolves and evaluates correctly from a main module that declares its own unrelated `P`.
  - A wrong local `P` through the chain is rejected, and a broken target (`base.Missing`) is reported in its own module.
  - The `records.rs` cycle key is now (module, name).
  - Tests: nx-api 461, nx-types 137, nx-hir 186 and nx-codegen 393 passed on my scratch target, with no corpus update.


### ✅ Verified - RF17 Rejecting a hex literal for an authored `Color` type reports "expects Color, found Color"
- **Severity:** Low
- **Evidence:** Rejecting the literal is correct per spec: an authored type named `Color` must not change literal meaning, which `authored_color_type_cannot_shadow_the_literal_type` and the lowering test cover. The problem is the message.
  - `type Color = { r: float32 g: float32 b: float32 a: float32 = 1 }` + `let accent: Color = #5b6ef5` reports `Initializer for value 'accent' expects Color, found Color`.
  - The RF12 display fix strips the synthesized qualifier, so both sides now print identically, and the message gives no hint that the literal always has the canonical `@nx/color` type.
  - The same happens when the canonical `color.nx` is analyzed outside `build_standard_library`. Compiling a copy as an ordinary file produces 149 `expects Color, found Color` errors, because only the registry build passes `defines_color_type = true`.
  - This is intended trust behavior, but contributors analyzing that file with the CLI or editor get only noise. The new shadowing tests check channels, not the diagnostic text.
- **Recommendation:** When the found and expected display names collide, print the canonical identity for the literal's type (for example, `found @nx/color Color (hex literal)`), and assert that text in the shadowing test. Optionally, note in the color module's header comment that it type-checks only as the registered standard library.

- **Fix:** Type display and nominal-collision detection share the public spelling for synthesized color names. Colliding types now qualify both declaring modules without exposing $nxColor. Added exact identity checks for value, record-field, and function-argument mismatch diagnostics.
- **Verification:** Verified.
  - The colliding case now reads `expects shadow.nx:Color, found @nx/color/color.nx:Color`.
  - It is equally clear for a record field, a function argument, and a `c.parse` result assigned to a local `Color`.
  - The analyzed copy of `color.nx` reports `expects authored/color.nx:Color, found @nx/color/color.nx:Color`.
  - The simple case stays concise and hides the internal alias: `Record field 'accent' on 'Brand' expects string, found Color`.
  - Spellings that already differ are unchanged, for example `expects c.Color, found Color`. A related display leak in names that don't collide is RF20.


### ✅ Verified - RF18 The style guide's rule to move a long trailing comment above the field produces a hard error unless a blank line is added, which the guide doesn't mention
- **Severity:** Low
- **Evidence:** `style-guide.md` (Comments and documentation) says: "If more than 2 lines are needed, move the entire comment above the declaration or field as a leading comment." It also says not to insert blank lines between record fields.
  - In a record where the previous field has a trailing `///` (the guide's own aligned style), a leading block for the next field is read as a misaligned continuation.
  - The probe was `a: float64  /// Short A.`, then a three-line `///` block indented 2 spaces, then `b: float64`. `nxlang codegen` fails with `misaligned-doc-comment-continuation` (`doc_comments.rs:204-223`).
  - A blank line before the block is required, and `reference/syntax/comments.md` documents that, but the style guide doesn't.
  - Two-line aligned trailing comments do behave as the guide describes: the probe compiled and the continuation attaches to the field. The 2-line cap itself is a style rule that nothing enforces, which is acceptable for a guide.
- **Recommendation:** In that paragraph, say that the leading comment needs a blank line before it when the preceding line ends with a trailing doc comment, or recommend using leading comments for the whole record. Link to the comments syntax reference, and add a short example.

- **Fix:** The style guide explicitly requires a blank line before leading documentation following a trailing doc comment, links the attachment rules, and includes a complete compiling example.
- **Verification:** Verified.
  - The new paragraph states the blank-line requirement and links `/reference/syntax/comments#attachment-errors`, which exists ("## Attachment errors").
  - The `Sample` example compiles without diagnostics. `nxlang typegen --language typescript` attaches "The measured value." to `value` and the three-line leading block to `interval`.
  - A small wording imprecision: without the blank line, an unaligned block is the `misaligned-doc-comment-continuation` error, not a continuation. Only an aligned block continues the comment. The guidance is still correct and actionable.


### Questions
- The named-constant group comment uses `//`, so hover and API docs show nothing for `c.lightSkyBlue`. The spec allows one shared group comment, and the style guide's example also uses `//`. Is having no per-constant API documentation intended? If not, a `///` that the doc system could attach to the group would need language support.

### Summary
- The scoped follow-ups are sound.
  - **Bootstrap:** the trusted flag is set only from the registry build for `@nx/color/color.nx`. Authored `color.nx` modules and same-named `Color` types still get the protected `$nxColor` import. Literal-only images link only `color.nx`.
  - **Constants:** all 149 are hex literals with exact float32 bits, checked against the independent Rust/TS name tables. The camelCase rename has no stale source, test or doc references.
  - **Tests and snapshots:** the regenerated conformance artifacts pass on both runtimes, and the trailing-comment layout matches the guide's 2-space rule.
- Three new findings. RF16 (Medium) is a pre-existing type-checker bug with re-exported aliases that breaks `g.Color` as an application type, and it is the one worth fixing or tracking before archiving. RF17 (Low) is a confusing diagnostic for authored `Color` shadowing. RF18 (Low) is a style-guide instruction that leads to a compile error. RF1–RF15 statuses are unchanged.

### Named-constant documentation decision

The user explicitly requested one shared comment instead of repetitive per-constant comments.
Keeping the ordinary group comment and no individual hover prose is intentional; no language
change for group-level doc attachment is part of this follow-up.

## New Findings Discovered During 2026-10-30 01:40 Verification

**Checks performed:**
- Rebuilt `nxlang` and the scratch `/tmp/nxrev/probe` (Rust IR link/evaluate) against current sources.
- Wrote workspace alias probes under `/tmp/nxrev/ws3`–`ws5`.
- Evaluated JavaScript-target output with Node.
- Ran `cargo test -p nx-types -p nx-hir -p nx-api -p nx-codegen --lib` without `NX_UPDATE_CORPUS`: all passed.
- Checked typegen documentation attachment.
- Did not rerun website, viewer, browser, or WASM checks.

### ✅ Verified - RF19 After the RF16 fix, a type alias cycle that spans two modules is accepted silently, and the alias then accepts any value
- **Severity:** Low
- **Evidence:** The probe workspace is:
  - `a.nx`: `import "./b.nx" as other` + `export type Y = other.X`
  - `b.nx`: `import "./a.nx" as base` + `export type X = base.Y`
  - `main.nx`: `import "./b.nx" as b`, `let v: b.X = 42`, and `let w: b.X = <b.X />`

  With current sources, `nxlang codegen --target javascript ws4 --entry main.nx` exits 0 with no diagnostic, and `root()` evaluates to `42`. Using `a.nx` or `b.nx` as the entry also reports nothing.

  The pre-fix binary (22:44) rejected the same program, though with an imprecise mismatch rather than a cycle error. A cycle within one module still reports `type-alias-cycle`.

  The new namespace walk appears to resolve the cycle to an error type without reporting it: each module treats the other's alias as foreign and defers the report. A cyclic alias therefore disables type checking wherever it is used, and no regression test covers cross-module alias cycles.
- **Recommendation:** Key alias cycle detection by (declaring module, name) in `resolve_named_type`, as `records.rs` now does. Report `type-alias-cycle` in at least the module that declares an alias on the cycle, even if it is reached as foreign. Add a two-module cycle test asserting the diagnostic and that `let v: b.X = 42` is not accepted silently.

- **Fix:** Alias walks now track declaring origins across local and foreign resolution, retain cycle evidence through quiet walks, and report type-alias-cycle on the defining declarations. Foreign namespace lookup now resolves references back into the current module instead of losing them. Added a two-module cycle regression (annotations and constructors) and a valid chain with identical alias names in different modules.
- **Verification:** Verified with a rebuilt `nxlang` (01:57).
  - The original `ws4` probe (`a.nx: Y = other.X`, `b.nx: X = base.Y`, `main: let v: b.X = "s"` and `let w: b.X = 42`) now fails. It reports `Type alias 'Y' forms a cycle` at `a.nx:3:1` and `Type alias 'X' forms a cycle` at `b.nx:3:1`, the same two declaration-site errors whether the entry is `main.nx`, `a.nx` or `b.nx`.
  - A three-module cycle (`A → nb.B → nc.C → na.A`) reports all three declarations. A single-module cycle still reports both aliases.
  - Legitimate chains are unaffected:
    - A chain where every module declares `T` and imports the previous one as `a`, `b` and `c` resolves `c.T` to `int`. It evaluates `[5,"local"]`, with a local `type T = string` kept separate, and rejects `let bad: c.T = "nope"` with `expects int, found string`.
    - The earlier `ws3` chain (record, string alias, union, `P*`, `P?`, distinct import names, a local `P`) still evaluates `[1,2,3,"key",{P x=4},{P x=5},{P x=6},"mine"]`.
  - Tests: nx-api 463, nx-codegen 395, nx-hir 186 and nx-types 137 passed on my scratch target, with no corpus update.


### ✅ Verified - RF20 Type mismatch diagnostics spell re-exported alias targets with the defining module's private import alias
- **Severity:** Low
- **Evidence:** The type is resolved correctly, but its display uses the import alias of the module that declared the re-export, which the user's file cannot name:
  - `import "@nx/graphics" as g`, a local `type Color`, and `let a: g.Color = <Color r=1 g=0 b=0 />` report `expects colors.Color, found Color`. `colors` is graphics' own import alias.
  - The reverse direction prints `expects Color, found g.Color`, so the same canonical type is spelled two ways.
  - The workspace probe `let bad: top.R = <P x=1 />` reports `expects base.P, found P`, where `base` exists only inside `b.nx`.

  The RF17 collision qualifier doesn't trigger because the raw spellings differ, even though both refer to a type named `Color` or `P`. Rejecting these is correct; only the message misleads.
- **Recommendation:** Display a resolved alias target by the name the current module can write: the importing alias such as `g.Color`, or else the origin-module form RF17 introduced (`@nx/color/color.nx:Color`). Never use a foreign module's import alias. Base collision detection on the displayed base names, and add assertions for the graphics alias case.

- **Fix:** Foreign nominal alias targets preserve the alias spelling visible to the consumer (g.Color/top.R), retaining their canonical origin and type arguments. Added graphics mismatch tests in both directions and function arguments, and a multi-module alias-chain mismatch test that rejects inaccessible colors.Color/base.P/hop.Q spellings.
- **Verification:** Reopened: the reported paths are fixed, but the same leak remains when the alias is reached through another module's declaration.
  - **Fixed:**
    - `let a: g.Color = <Color …/>` with a local `Color` now reads `expects g.Color, found Color`.
    - `let a: g.Color = { "red" }` reads `expects g.Color, found string`, and `f(1)` with a `g.Color` parameter reads `Argument 0 expects g.Color, found int`.
    - The workspace case reads `expects top.R, found P`.
    - The RF17 canonical-vs-local qualification is unchanged (`expects sh2.nx:Color, found @nx/color/color.nx:Color`), and so is the concise `expects string, found Color`.
  - **Still leaking:** a field or parameter whose declared type is a foreign alias still prints the alias target with the declaring module's private import name.
    - Workspace probe `ws11`: `a.nx` declares `P`; `b.nx` has `import "./a.nx" as base`, `export type Q = base.P`, `Shape = none | solid { p: Q }` and `Holder = { p: Q }`.
    - From `main` (`import "./b.nx" as b`), `<b.Shape.solid p="x" />` reports `Property 'p' on 'b.Shape.solid' expects base.P, found string`, and `<b.Holder p="x" />` reports `… on 'b.Holder' expects base.P …`.
    - `base` exists only inside `b.nx`, and the consumer can write the type as `b.Q`.
  - The same happens with no alias at all: a field `p: hidden.P` or a parameter `take(p: hidden.P)` reports `expects hidden.P, found P` (`ws8`).
  - Fix: render a foreign declaration's field and parameter types through the consumer's view, either by the alias it can reach (`b.Q`) or by the origin-module form RF17 uses (`a.nx:P`), never the declaring module's import alias. Add `ws11`/`ws8`-style assertions.

- **Fix:** Follow-up: nominal record/component types resolved in a foreign namespace now use declaration-based display names; qualified private import spellings use origin-module-qualified names, including foreign union types. Consumer-visible direct alias names remain intact. Added regressions for record fields, union-case fields, and function parameters with both alias and direct private-import type references, checking strings and unrelated same-shape records.
- **Verification:** Verified with a rebuilt `nxlang` (02:10).
  - The original reproductions now spell the type by its origin module, not a private import:
    - `ws8` (field `p: hidden.P` and parameter `take(p: hidden.P)`) reports `Property 'p' on 'b.Box' expects a.nx:P, found P` and `Argument 0 expects a.nx:P, found P`.
    - `ws11` (union-case field and record field typed by the foreign alias `Q = base.P`) reports `expects a.nx:P, found string` for both `b.Shape.solid` and `b.Holder`.
  - Direct alias references keep the consumer's spelling: `expects top.R, found P`, `expects g.Color, found Color` / `found string` / `found int`, and `expects g.Paint …`.
  - Unchanged: the RF17 collision form (`expects shadow.nx:Color, found @nx/color/color.nx:Color`), the concise `expects string, found Color`, and valid ws3 chain evaluation.
  - Tests: nx-api 463, nx-codegen 399, nx-hir 186, nx-types 137 and nx-language-service 317 passed on my scratch target, with no corpus update.


### Notes
- Out of scope, recorded only for follow-up: the JavaScript codegen target nests a sequence that contains a reference to an imported constant, while Rust IR flattens it.
  - Workspace probe: `let xs: int* = { m.one 2 }` with `root() = { 0 xs }` gives `[0,[1,2]]` in JavaScript and `[0,1,2]` in Rust IR. A local `one` flattens on both.
  - It reproduces without colors and is unrelated to RF16–RF18, but color constants such as `{ #fff c.red }` make it easy to hit.

### Summary
- Verified 3: RF16, RF17, RF18. Reopened 0. New 2: RF19 (Low, a cross-module alias cycle regression from the RF16 fix) and RF20 (Low, foreign import-alias spelling in diagnostics). RF1–RF15 statuses are unchanged.

## New Findings Discovered During 2026-10-30 01:59 Verification

**Checks performed:**
- Rebuilt `nxlang` and `/tmp/nxrev/probe` (Rust IR link/evaluate) from current sources.
- Wrote workspace probes `ws4`, `ws6`–`ws11` and single-file graphics/color probes under `/tmp/nxrev`.
- Ran `cargo test -p nx-types -p nx-hir -p nx-api -p nx-codegen --lib` without `NX_UPDATE_CORPUS`: all passed.
- Did not rerun website, viewer, browser or WASM checks.

**RF18 recheck:** the reworded paragraph (`style-guide.md:161-165`) now says that aligned comments continue the trailing comment and unaligned ones produce an attachment error. This matches `doc_comments.rs`, and RF18 remains Verified.

### ✅ Verified - RF21 Properties of the graphics `Paint.solid` case are not type-checked at compile time, so strings, numbers and unrelated local `Color` records compile
- **Severity:** Low
- **Evidence:**
  - `import "@nx/graphics" as g` + `let p = <g.Paint.solid color="red" />` compiles with no diagnostic. So do `color={ 1 }`, `color={ "red" }`, and `color=<Color name="red" />` for a local `type Color = { name: string }`.
  - Other graphics properties are checked: `<g.ViewBox x="a" …/>`, `<g.Stroke paint="x" …/>` and `<g.Drawing … background="red" />` all report type errors. A workspace union with an alias-typed payload (`ws11`) and a local union (`<Shape.solid size="big" />`) are checked too, so the gap is specific to this standard-library case payload.
  - On the Rust IR runtime, a string fails only at the output boundary (`nx-ir-boundary-type: Expected Paint.solid.color to be an object`), and a wrong-shape record fails with `nx-ir-boundary-field`.
  - A user-declared `type Color = { r: float32 g: float32 b: float32 a: float32 = 1 }` passes both compile and runtime and comes out as `{"$type":"Color", …}`. That contradicts the rule that an unrelated user `Color` is not the canonical color.
  - The pre-fix binary (22:44) behaves the same, so this predates RF16–RF20.
  - Task 7.1 claims compiler tests that "reject direct strings", but no graphics test asserts a compile-time rejection for `Paint.solid color`. The spec's "fail type checking or scene validation" requirement is partly met only because the runtime boundary rejects strings.
- **Recommendation:** Find why property checking is skipped for `Paint.solid` elements from the standard library (compare the `ws11` workspace path, which is checked). Make `color` check against the canonical Color. Add tests asserting compile errors for `color="red"`, a number, and a same-shape local `Color`.

- **Fix:** Imported library modules now share their retained dependency peer context, without adding any bindings to the consumer namespace. This lets union-case payload field checking resolve the canonical Color even with a graphics-only import. Added compile-rejection regressions for string, braced string, numeric, and unrelated same-shape Color payloads, plus Rust IR evaluation of a valid graphics-only color constructor/annotation/solid-paint path.
- **Verification:** Verified.
  - With only `import "@nx/graphics" as g`, `Paint.solid` payloads are now checked:
    - `color="red"` reports `Property 'color' on 'g.Paint.solid' expects @nx/color/color.nx:Color, found string`.
    - `color={ 42 }` reports `found int`.
    - A local `type Color = { name: string }` and a same-shape local `Color` both report `expects @nx/color/color.nx:Color, found Color`.
  - Valid graphics-only paths compile and evaluate on the Rust IR runtime with canonical `$type: "Color"` records: `#ff0000`, a `g.Color` annotation and constructor, a translucent `<g.Color … a=0.5 />` nested in `Paint.solid`, and a `Stroke` paint.
  - The shared peer context does not leak into the consumer's namespace:
    - With a graphics-only import, `colors.Color`, `math.Vector` and bare `Color` are reported as not visible, and `colors.red` is an undefined identifier.
    - From `ws11`'s main, `base.P` is reported as not a visible type.


### Summary
- Verified 1: RF19 (cross-module alias cycles are reported at their declarations, and legitimate same-name chains still resolve).
- Reopened 1: RF20 (direct alias references are fixed, but field and parameter types of foreign declarations still print private import aliases such as `base.P` and `hidden.P`).
- New 1: RF21 (Low, `g.Paint.solid` payload not type-checked at compile time).
- RF16–RF18 remain Verified, with RF16's alias evaluation and the RF17 diagnostics rechecked. RF1–RF15 are unchanged. The JavaScript-target sequence nesting stays an out-of-scope note.

## New Findings Discovered During 2026-10-30 02:10 Verification

**Checks performed:**
- Rebuilt `nxlang` and `/tmp/nxrev/probe` (Rust IR runtime) from current sources at 02:10.
- Reran the `ws8`, `ws11` and `ws3neg` workspace probes, the graphics payload probes (`gp`, `gp2`, `gp3`, `gp5`), the RF17 diagnostic probes, valid graphics-only evaluation, and probes for leaks of transitive names into the consumer's scope.
- Ran `cargo test -p nx-types -p nx-hir -p nx-api -p nx-codegen --lib` and `cargo test -p nx-language-service` without `NX_UPDATE_CORPUS`: all passed.
- Did not rerun website, viewer, browser or WASM checks.

### ✅ Verified - RF22 The "did you mean" suggestion for an unknown type can offer the synthesized `$nxColor.Color` name
- **Severity:** Low
- **Evidence:**
  - `import "@nx/graphics" as g` + `let x: colors.Color = #fff` reports `` `colors.Color` is not a visible type; did you mean `$nxColor.Color`? ``.
  - The hex literal adds the unspellable `$nxColor` import, and `did_you_mean_type` offers its qualified name as a candidate. The user cannot write that name, which defeats the RF12/RF17 rule that `$nxColor` never appears in user diagnostics.
  - The pre-fix binary (22:44) prints the same message, so this predates RF20 and RF21. Neither fix leaks names: the other probes in this pass show no transitive names becoming visible.
- **Recommendation:** Exclude synthesized `$`-prefixed import aliases from `visible_type_names` suggestions, or suggest the public spelling (`@nx/color` `Color`, or an import the user could add). Add an assertion for this message.

- **Fix:** Type-suggestion candidates now exclude compiler-synthesized $-prefixed names while semantic visibility remains unchanged. Added regressions for the graphics-only colors.Color example, a similar unimported type, and a public c.Colr typo that must still suggest c.Color.
- **Verification:** Verified with a rebuilt `nxlang` (02:14).
  - `visible_type_names` (`infer.rs:4914-4926`) now drops `$`-prefixed names, and only the suggestion text uses it.
  - Probes:
    - The graphics-only `let x: colors.Color = #fff` now reads `` `colors.Color` is not a visible type `` with no suggestion.
    - `nxColor.Color` without an import gets no `$nxColor` suggestion.
    - `import "@nx/color" as c` + `c.Colr` still suggests `c.Color`, and a local `Colur` still suggests `Colour`.
  - Resolution is unchanged:
    - `let x: c.Color = #5b6ef5` and a bare `#fff` compile.
    - A literal in a string field still reports `expects string, found Color`.
  - Tests: nx-types 137 and nx-codegen 400 passed on my scratch target, including `unknown_type_suggestions_hide_internal_color_imports`, with no corpus update.


### Summary
- Verified 2: RF20 (field, union-case-field and parameter types now use origin-module names, and direct aliases keep consumer spellings) and RF21 (`g.Paint.solid` payloads are checked with a graphics-only import, valid paths evaluate, and no transitive names leak).
- Reopened 0.
- New 1: RF22 (Low, pre-existing `$nxColor.Color` "did you mean" suggestion).
- RF1–RF19 remain Verified. The JavaScript-target sequence nesting stays an out-of-scope note, and the shared group comment for named colors follows the user's stated preference.

## Verification Pass 2026-10-30 02:14

- Verified 1: RF22. Reopened 0. New 0.
- All 22 findings (RF1–RF22) are now Verified. No findings remain open.
- The JavaScript-target sequence nesting stays an out-of-scope note, and the shared group comment for named colors follows the user's stated preference.
