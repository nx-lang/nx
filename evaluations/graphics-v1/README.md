# Graphics authoring trials, version 1

Deterministic tests require no model key: run `node scripts/verify-graphics-fixtures.mjs`, the
Rust graphics tests and the TypeScript graphics IR test. The SVG references state shared semantics
and explicit tolerances; native tests check interior pixels and analytic coordinates. Edge
antialias pixels are not compared across machines. This is a small reference corpus, not a claim
of full SVG compatibility.

Optional model trials: run each tasks.json prompt twice, once restricted to primitives and once
allowing the ordinary NX helpers. Keep model/version, budget, seed, fonts, renderer and assets
identical. Save initial NX, PNG and manifest; then deliver the requested revision, saving all three
again. Record one result.schema.json record per task/variant with elapsed milliseconds, bytes,
round trips and separate initial/revision success. Score each rubric item 1..5 with a written
rationale. Record unavailable seed as null. Do not claim quality improvement from fixture tests;
that requires the saved, comparable model trials. No trials have been run for this change.
