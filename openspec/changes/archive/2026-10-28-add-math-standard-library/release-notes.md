# Math standard library

Adds unstable `@nx/math`: 22 float64 functions and pi/tau constants for graphics and general
computations. Rust and TypeScript IR runtimes and executable TypeScript/JavaScript share the
numeric contract, including strict finite/domain/overflow diagnostics and signed-zero semantics.
.NET uses the native Rust binding. Math images require `standard-math-v1`; upgrade compiler and
runtime together. Existing non-math images and general arithmetic semantics are unchanged.
