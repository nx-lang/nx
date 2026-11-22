## 1. The library

- [ ] 1.1 Move `api/types.nx`, `api/palette.nx` and `api/tokens.nx` to `crates/nx-program/src/std/design/` as the `@nx/design` standard library, with an `unstable` source header, and register it in `crates/nx-program/src/artifacts.rs` beside `@nx/color`. Remove the `api/` drafts from the change.
- [ ] 1.2 Settle the palette's source (Open Questions) and add the attribution line if the Tailwind values stay.
- [ ] 1.3 Add a test that the library is free of diagnostics and analyzed only when imported, and tests for the value types' constraints and alias transparency.
- [ ] 1.4 Add a test that every scale is nondecreasing and that each `on…` role, and each `text` role but `disabled` on `background` and `surface`, meets a contrast ratio of 4.5 in both color modes.

## 2. Conformance

- [ ] 2.1 Turn `examples/` into a conformance corpus program, `specs/ir-conformance/design-tokens/`, with entrypoints for each scenario: roles in each mode, a text role following `env.font`, a component token following the core, a theme tuning one library, and a theme varying by mode. Record results and operation counts for both runtimes.
- [ ] 2.2 Check that generated JavaScript gives the same results, or record what it lacks in `specs/future.md`.

## 3. Documentation

- [ ] 3.1 Add `sites/website/src/content/docs/reference/libraries/design.md`: every type and token with its default, the extension rules, themes as bundles, and the stability contract.
- [ ] 3.2 Extend the style guide's token section with the naming table and the extension rules.
- [ ] 3.3 Rewrite `tutorials/working-with-design-tokens.md` around `@nx/design`: read tokens, build a component with its own tokens, write a light and dark brand theme, and apply it app-wide and to one section.
- [ ] 3.4 Check every new code block with the website's code-block check.

## 4. Completion

- [ ] 4.1 Run the Rust and TypeScript test suites, `cargo fmt` and `cargo clippy`, then the review loop, then validate strictly, sync and archive.
