## Why

NX has no way to count. Rendering five stars, `n` skeleton rows, or pages 1 through `pageCount` needs a
list to iterate, and there is none unless a host supplies it; and a control that takes bounds — a
slider, a validator — has no type to say so with. Generic records landed so that `Range` could be an
ordinary record rather than a special case, but an ordinary record needs somewhere to be declared
that every module can see, and NX has no such place: its built-ins today are names the checker
knows, never declarations. This change adds that place, a prelude, puts `Range` in it, and adds the
`..` and `..=` operators and `for` over an integer range on top.

## What Changes

**A language prelude.**

- A prelude module, written in NX and embedded in the compiler, is part of every build — a single
  source, a workspace, a library, an editor snapshot — with no option to supply and none to forget.
  Its exported declarations are in scope in every module without an import.
- Prelude names bind last and never conflict: a module's own declaration or any written import of the
  same name wins silently, as it does over any wildcard import. A module's text, positions and
  diagnostics are unchanged.
- The prelude is not part of a library's exports, has a reserved identity (`@nx/prelude.nx`) that a
  workspace may not supply, and contributes to a program's fingerprint, so a compiler upgrade that
  changes it invalidates caches.
- Its first declaration is `export type Range = { T:type start:T end:T endInclusive:boolean }`.

**Range expressions.**

- Two binary operators, as in Rust: `a..b` is the half-open range `[a, b)` and `a..=b` the closed
  range `[a, b]`. They bind looser than `+`/`-` and tighter than the comparisons, so `0..n + 1` is
  `0..(n + 1)`.
- A range expression is sugar for constructing the prelude's `Range`. Its operands are numeric; `T` is
  the type the site expects when it expects a `Range`, and otherwise the operands' common numeric
  type. `1..5` is a `<Range T=int/>`, and `<Slider range={0..1}/>` against `range:<Range T=float64/>`
  is a `<Range T=float64/>`. Construction never fails: `5..2` is a valid, empty range.
- Because the operator means the prelude's `Range`, it is rejected in a module where the name `Range`
  means something else.
- `for` accepts a `Range` of an integer type beside a list. The item is each integer from `start`
  up to `end`, excluded or included; an optional index variable counts from zero; an empty or
  reversed range yields no items. A range of a non-integer type is a type error in a `for`, not a
  guessed step. There is no step, no descending form and no open-ended range.

**Below the checker.**

- The operator leaves no trace: after checking it *is* a record construction, so the interpreter, NX
  IR and generated code need no new expression for it.
- In NX IR the prelude is an ordinary module that a using image links against, so a declaration
  still lives in exactly one module and the prelude can later hold functions, whose values carry a
  module identity. **No host has to supply it**: the TypeScript runtime package ships the compiled
  prelude image and serves it whenever the host's resolver does not, and the compiler emits the
  image on request for any other runtime. `for` over a range is a new node kind, `forRange`, behind
  a new required feature `ranges-v1`. **The IR schema version does not change.**
- The TypeScript IR runtime evaluates `forRange` as a counting loop and refuses a range longer than a
  configurable limit, since a range makes an enormous loop one token long.
- Generated contracts name the prelude's types: C# maps `<Range T=X/>` to a hand-written
  `NxRange<X>` in `NxLang.Sdk` — avoiding `System.Range` — and TypeScript emits `Range<T>` into the
  helper module it already writes beside `NxRecord`.

**Retiring the other "prelude".** `language-core` and `language-http` have an older feature of the
same name that prepends a host's text to the queried document and shifts every position. Implicit
imports superseded it and the playground no longer uses it. **BREAKING** for hosts of those two
packages: the `prelude` option and its exports are removed, replaced by a `context` of documents and
implicit imports, which needs the Node SDK's language snapshot to accept implicit imports as the
wasm SDK's already does.

**Editor and docs.** `..` and `..=` highlight as operators in both grammars; the prelude's
declarations complete and hover like any library's; the `for`, expressions, types and modules
reference pages, both grammar documents, the IR format document and `specs/future.md`'s note on
integer range types are updated.

## Capabilities

### New Capabilities

- `language-prelude`: the prelude module — where it is present, how its names bind and are shadowed,
  what it must not affect, and what it declares.
- `range-expressions`: the `..` and `..=` operators, their typing and the value they produce, and
  `for` over an integer range.

### Modified Capabilities

- `nx-ir-format`: adds the prelude as an ordinary linked module with an empty version and its
  emission rule, the `forRange` node kind, and the `ranges-v1` required feature, with the schema
  version unchanged.
- `typescript-ir-runtime`: adds the built-in prelude module the runtime supplies during linking,
  evaluation of `forRange`, the `ranges-v1` feature, and the range length limit.
- `cli-code-generation`: adds how generated C# and TypeScript refer to a prelude type.
- `dotnet-binding`: adds the hand-written `NxRange<T>` SDK type and its wire shape.
- `editor-syntax-highlighting`: adds the scoping of the two range operators.
- `language-http-service`: removes the prepended prelude document and adds host context served as
  implicitly imported documents.
- `sdk-node`: adds implicit imports to the language snapshot.

## Impact

- **`crates/nx-api`**: embedded `prelude.nx`; the prelude built once as an in-memory library; bound
  into every module on both the workspace and the library pipeline; program fingerprint.
- **`crates/nx-syntax`**: `..` and `..=` in `binary_expression` at a new precedence level;
  regenerated parser; highlight queries.
- **`crates/nx-hir`**: `Expr::Range`; the prelude's module identity; a post-check rewrite of a range
  into a record construction.
- **`crates/nx-types`**: typing of a range expression; `for` over `Range`; the shadowing diagnostic.
- **`crates/nx-interpreter`**: `eval_for` counts over a `Range` record without materializing it.
- **`crates/nx-codegen`**: the prelude's identity and emission rule; the checked-in prelude image and
  its staleness test; `forRange` (node kind 22) and `ranges-v1`;
  `nxRangeMap` in executable TypeScript; `docs/nx-ir-format.md`.
- **`runtime/typescript`**: the generated `prelude-image.ts` and the link fallback; `forRange`, the
  feature, `maxRangeLength`.
- **`crates/nx-cli/src/typegen`**, **`bindings/dotnet/src/NxLang.Sdk`**: prelude type references and
  `NxRange<T>`.
- **Language service, playground**: prelude declarations in completions and hover; the playground
  stops classifying a label inside the prelude as the visitor's own text.
- **`packages/language-core`, `packages/language-http`, `bindings/node`**: the text prelude removed,
  `context` added, snapshot implicit imports.
- **Editor assets**: `src/vscode/syntaxes/nx.tmLanguage.json`, `queries/highlights.scm`.
- **Breaking only for `language-core`/`language-http` hosts** that pass `prelude`. NX source is
  unaffected: a program that declares its own `Range` keeps working, because a local name shadows
  the prelude; it only cannot use `..` in that module.
- An image that uses the prelude needs a runtime that has the built-in prelude, so it does not load
  on the released 0.2.0 runtime. The DrawnUI fiddle lives in another repository and needs only its
  next NX update: its resolver never has to know the prelude exists.
