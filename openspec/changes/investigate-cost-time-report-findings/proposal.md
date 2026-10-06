## Why

`add-ir-runtime-input-limit-and-cost-tests` added a time report that runs generated host values
through both IR runtimes at two scales and names a case whose time grows more than twice as fast
as its operation count plus its input size. On the unmodified runtimes it names two kinds of case,
the first every time it is run and the second in most runs. Neither is a step that does work for no charge, which is what the report is
for: each is charged in proportion to the values it touches, and its time for each unit still
rises between the scales. They are listed as known findings in
`crates/nx-codegen/tests/cost/mod.rs` so that the report stays quiet, and they are tracked here.

- **A wide object in the TypeScript runtime.** The time the runtime spends on each field of an
  object rises about two and a half times between a few hundred fields and a few thousand,
  wherever it compares such an object by name (the probes `compareEq`, `compareSelf` and
  `compareHandlers`: two objects of 333 fields of 63 code units against two of 2,664, with no
  names shared, grow their time 2.3 to 3.0 times as fast as their units) and wherever it writes
  one for the host (`asProperty`, `asElement` and `place`: 124, 207 and 346 ns a unit at 394,
  1,000 and 3,152 fields). Writing sorts the names of the object, which is `n log n` comparisons
  of strings for a charge of `n`, and V8 holds an object of more than about a thousand
  properties in its slower dictionary form.
- **A result of a million values and more in the Rust runtime.** `place` given a list of 374
  short strings and empty lists and repeated 128 times writes 48,000 values in 1.3 ms, 13 ns a
  unit; given 2,992 and repeated 1,024 times it writes three million in 185 ms, 30 ns a unit.
  Every value written is an allocation, and a result of over a hundred megabytes is outside the
  processor's caches. The other probes that write their value on every repeat show the same:
  a list of 235 small records held in an element and written 1,024 times, 13 million units,
  grows its time three to four times as fast as its units in some runs and under twofold in
  others.
- **The same in the TypeScript runtime, at the edge of the rule.** `place` given a list of 65
  floats writes 8,000 values in 0.13 ms and then half a million in 15 to 17.5 ms: 2.0 to 2.2
  times as fast as its units, so the report names it in about one run in three. It has an entry
  of its own, for the TypeScript runtime, with the predicate of the Rust one.

Both are inside the factor the cost model allows itself in spirit, a constant, but the first is a
step that is worse than linear in a host value, which the input limit bounds only as a function of
the limit, and the second is the kind of effect that will keep the report from being read if it is
reported on every run.

## What Changes

- Decide, for a wide object in the TypeScript runtime, whether writing it can stop sorting its
  names (the token walk needs one order in every runtime, and nothing else does), and whether the
  walks that list an object's names can avoid the engine's dictionary form; and if neither, say
  in the format document that a wide object costs up to about `log n` more than its count for
  each field, and charge for it or leave it.
- Decide, for a large result in the Rust runtime, whether a value written for the host can be
  written with fewer allocations, and otherwise make the report tell a memory effect from an
  uncharged step: for example by comparing a case's time for each unit with a calibrated case of
  the same size that is known to be charged in full.
- Remove the three entries from `KNOWN_FINDINGS` when the report no longer names their cases.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `nx-ir-format`: the validation of evaluation cost against generated host values, if the rule by
  which the time report names a case changes; and the cost of a value written for the host, if a
  wide object comes to cost more than one for each field.
- `typescript-ir-runtime`: how a value is written for the host, if the names of an object stop
  being sorted.

## Impact

- `runtime/typescript/src/index.ts` (`canonicalizeRendered`, `recordsEqual`),
  `crates/nx-ir-runtime/src/value.rs` (`to_host`), `crates/nx-codegen/tests/cost_differential.rs`
  (the time report) and `crates/nx-codegen/tests/cost/mod.rs` (`KNOWN_FINDINGS`).
- No operation count changes unless a wide object is charged for the ordering of its names, which
  would change the recorded counts of the corpus cases that write one.
