# Math references

`crates/nx-value/src/math-cases.json` is shared by the Rust numerical implementation, both Rust evaluators, TypeScript IR,
and generated TypeScript/JavaScript. Inputs and expected values are strings so negative zero and
non-finite host inputs survive JSON parsing without losing their meaning.

`generate.py` uses mpmath 1.3.0 at 500 decimal digits. It converts input strings to binary64 first,
then evaluates their exact values in high precision, rounding the result once to binary64. NX's
rounding/zero/endpoint/domain conventions are explicit cases rather than outputs copied from one
runtime. Ordinary tests only read the checked-in corpus and require no Python dependency.

To regenerate in an isolated environment with mpmath installed:

```sh
python3 specs/math-conformance/generate.py
```

The corpus includes all 22 operations, all atan2 zero quadrants, negative rounding ties, a value just
below a tie, huge-angle trigonometry, a near-pole tangent, the smallest positive subnormal, large
finite distances, convex interpolation, extrapolation, and the three diagnostic categories.
`examples/math-graphics/main.nx` supplies renderer-independent polygon, star and arrow coordinates;
Rust and TypeScript integration tests check independent coordinate expectations.

The corpus lives inside `nx-value` so its packaged Rust tests remain self-contained.
