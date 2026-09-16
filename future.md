# Future work

Things worth investigating later. Each entry says what was observed, why it might matter, and what
would settle it.

## Logical operands: the IR runtime coerces, the interpreter demands a boolean

**Observed.** The two runtimes disagree about what a non-boolean operand of `&&` or `||` means. The
TypeScript IR runtime passes each operand through a truthiness helper, `truthy` in
`runtime/typescript/src/index.ts`, which is `Boolean(value)`, so a string or a number is accepted
and coerced. The Rust interpreter rejects it, raising a type error naming `logical and` at
`crates/nx-interpreter/src/interpreter.rs`.

**Why it might matter.** This is the same family as the short-circuit divergence that was fixed by
making the IR runtime non-strict in the right operand of `and` and `or`. That one was observable and
wrong. This one may not be observable at all, because the type checker probably rejects a
non-boolean operand before either runtime sees it, in which case the coercion is dead code rather
than a semantic difference. It was not verified either way.

**What would settle it.** Try to compile a program whose `&&` operand is not a boolean, for example
`let root() = { "a" && true }`. If static analysis rejects it, the coercion is unreachable and the
helper can be replaced with a check that fails loudly, which is the safer thing for a runtime that
reads images written by strangers. If static analysis accepts it, the two runtimes genuinely
disagree and one of them is wrong, and a conformance corpus case should pin whichever behavior the
language intends.

**Related.** The conformance corpus gained short-circuit cases in
`specs/ir-conformance/expressions/main.nx`. A case for this would belong beside them.
