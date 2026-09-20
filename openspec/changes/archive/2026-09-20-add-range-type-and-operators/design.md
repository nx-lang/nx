## Context

See proposal.md for motivation. What shapes the approach:

- **NX built-ins are names, not declarations.** `Element` and `object` are origin-less
  `Type::Named`s; the four update intrinsics are typed by rule (`infer_intrinsic_call`), with a
  comment saying they are "the typing rules the four prelude signatures would state". Nothing
  built-in has fields, a `RecordDef`, an IR declaration or a typegen output. `Range` needs all four.
- **Two analysis pipelines.** Workspace and single-source builds go through
  `analyze_logical_module_graph` → `prepare_logical_source_file` (`artifacts.rs:1091`). Library
  builds go through `build_library_artifact_with_registry` → `prepare_library_source_file`
  (`artifacts.rs:776`) and never see a host's implicit imports. Both already know how to bind names
  from a `LibraryArtifact` (`add_imported_interface_bindings`), and both skip a name the module has
  already bound (`artifacts.rs:1543`, `:1898`) — local-wins is existing, tested behaviour.
- **Host implicit imports are not a prelude.** `with_implicit_imports` is a host option over real
  workspace modules, binds with wildcard-import semantics (including the eager "provided by both"
  diagnostic at offset 0), and is absent from library builds, the LSP and `nxlang`.
- **NX IR is one image per module, linked by the host.** A reference is *(module slot, name)*; the
  TypeScript runtime resolves every module-table entry through a host-supplied `resolve`, and
  `prepareNxIrProgram` supplies none. The playground and the DrawnUI fiddle each resolve exactly one
  extra module, their catalog. A record value carries only a name (`$type`); a function value carries
  a module identity and a name.
- **Released surface.** IR schema 4 shipped as v0.2.0. Growth since has been by new node kinds plus
  a required-feature string, which is "what makes the list safe to grow" (`docs/nx-ir-format.md`).
- **Post-check rewrites are an established pattern.** `check.rs:275-315` already applies constant
  folds, literal and string conversions, contextual-name resolutions and type-argument removal to
  the prepared module, keeping each `ExprId`.
- **Typegen follows a different pattern per language.** Generated C# depends on `NxLang.Sdk`
  (`NxOptional<T>`, `NxFunctionRef`, referenced as `global::NxLang.Nx.…`). Generated TypeScript
  depends on nothing: it writes its one shared type, `NxRecord`, into a `_nx.ts` helper module.
- **"Prelude" is already a word here.** `packages/language-core/src/prelude.ts` and
  `language-http` prepend a host document's text to the queried document and shift every position.
  It predates implicit imports, the playground has moved off it, and its one remaining user is a
  reference handler in `bindings/wasm/test`. The Node SDK's snapshot, unlike the wasm one, does not
  yet accept implicit imports, which is the only thing the HTTP handler would need to drop it.
- **The repo already checks in generated images.** The conformance corpus keeps `.nxir` files under
  `expected/`, regenerated with `NX_UPDATE_CORPUS=1`, and the playground bundles its catalog image
  as base64.

## Goals / Non-Goals

**Goals:**

- `Range` is an ordinary declaration: every layer that handles a user's generic record handles it
  through the same code, and nothing but the two operators and `for` knows its name.
- The prelude costs a host nothing: no option, no image to ship or resolve, no position shift — while
  staying a real module, so that it can later hold functions, whose values carry a module identity.
- The operators are sugar that ends at the checker. One rewrite, zero backend knowledge.
- A program that does not use the prelude is unchanged in every artifact.
- One thing in the repository is called a prelude.

**Non-Goals:**

- No step, no descending ranges, no open-ended ranges, no slicing, no `x in range` membership, no
  range patterns. None is blocked.
- No non-numeric operands for the operators. `<Range T=string …/>` is constructible by element.
- No integer range *types* (`type Rating = 1..=5`). `specs/future.md` is updated to use this
  change's spelling; the feature stays future work.
- Moving the update intrinsics into the prelude. It needs generic functions, which NX lacks.
- A way to opt out of the prelude, or for a host to extend it.
- Loop or allocation limits in generated executable code, which has none today.
- Changes to the DrawnUI fiddle repository.

## Decisions

### D1: The prelude is a built-in library, bound last on both pipelines

`crates/nx-api/src/prelude.nx` is embedded with `include_str!` and built once per process
(`OnceLock`) into a `LibraryArtifact` by the ordinary library build, refactored so the directory
read is separate from the build and the build accepts in-memory sources. Its root and its one
module share the reserved identity `@nx/prelude.nx` (a constant in `nx-hir`, so the checker can name
it). It has to be a legal file name on every platform, because the prelude's image is written like
any module's (D5), which rules out a colon; `/` becomes `__` in an image file name, as it does for
every identity. `LogicalModuleGraph::from_modules` refuses a workspace module under that identity.
The prelude's source is always in the source map, so diagnostic rendering never probes the disk for
it.

One function, `apply_prelude_bindings`, is called at the end of both `prepare_logical_source_file`
and `prepare_library_source_file` — after locals, written imports, host implicit imports and
same-library peers. It binds each exported prelude item through the existing
already-bound-then-skip path and takes no part in the ambiguity or duplicate-import checks, which
is what "binds last and never conflicts" means. It is skipped when the module being prepared is the
prelude itself. The prelude library joins the program's `libraries`, so `ResolvedProgram`,
`CodegenProgram` and the language service's `visible_libraries` loop all receive it as they receive
any library, with its source text in `LibraryArtifact.sources`. Its fingerprint is hashed into the
program fingerprint beside the implicit imports.

*Why a library and not a workspace module:* a library build cannot take a workspace module as a
dependency, so a graph-module prelude would need a second mechanism for libraries. A library is the
one form both pipelines already consume.

*Why not an always-on entry in `implicit_imports`:* that option promises written-wildcard semantics,
so a user library that exports `Range` would produce "provided by both" in every importing module,
pointed at offset 0. It is also per-context, and four entry points build their context without it.

*Alternative considered:* registering `Range` in Rust, like the intrinsics. Rejected with the user:
element construction, companions, the IR declaration and typegen would each need bespoke handling.

### D2: Shadowing is allowed; the operator requires the prelude's `Range`

A module's own `Range`, or an imported one, hides the prelude's, exactly as it would hide a
wildcard import's. This keeps every existing test, spec scenario, example and conformance program
that declares `type Range` valid with no edits.

The checker types a range expression only when `Range` in the module's type namespace resolves to
the declaration whose origin is the prelude's identity; otherwise it reports `range-hidden`, labelling the
hiding declaration or import. This is what makes D3's by-name rewrite sound.

*Alternative considered:* reserving prelude names. Rejected: every name later added to the prelude
would break programs, and ~35 existing scenarios declare `Range`.

### D3: `Expr::Range`, typed by rule, rewritten to a record construction after checking

Grammar: `..` and `..=` join `binary_expression` at `prec.left(100)`, between additive (110) and
relational (90). Left-associativity means `1..5..9` parses and then fails to type, which gives a
better message than a parse error. Lowering produces `Expr::Range { start, end, inclusive, span }`
rather than a `BinOp`, because every consumer of `BinOp` assumes a primitive result.

The lexer needs nothing special: `real_literal` requires a digit after the dot, so `1..5` backs off
to `int_literal`; `..=` and `..` are ordinary tokens that win over `.` by longest match; the
external scanner handles text content only.

Typing, in `infer.rs` beside `infer_binop`:

1. Require the prelude's `Range` (D2).
2. If the expected type is that `Range` with argument `X`, check each operand against `X` with the
   routine a record field binding uses, so literal conversion and widening behave as they do for
   `start={0}`. Otherwise infer both operands, require both numeric, and take
   `Primitive::numeric_promotion`; `None` is the existing "neither converts to the other" error.
3. The result is `nominal_named_type("Range").with_args([("T", X)])`.

After checking, `nx_hir::apply_range_constructions` — in the block at `check.rs:275` — replaces
each `Expr::Range` with `Expr::RecordLiteral { record: "Range", properties: [start, end,
endInclusive] }`, keeping the `ExprId` and its recorded type, and allocating one boolean literal
whose type it records. No `T=` property is needed: type arguments are already consumed at that
point. From here the interpreter, the codegen builder and `emit.rs` see a record construction and
nothing else, including default-free field filling and host-boundary behaviour.

*Alternative considered:* carrying `Expr::Range` to each backend. Rejected: three constructors of
the same record, each of which must agree with the element form forever.

### D4: `for` dispatches statically in the checker and IR, by value in the interpreter

Checker: the `Expr::For` arm accepts `Type::Array` or the prelude `Range` with an integer argument
(`Primitive::is_integer`), binds the item as `X`, and rejects any other `Range` with
`range-not-iterable`. It records the `ExprId` of each range-driven `for`.

Interpreter: `eval_for` works on HIR with no types, so it matches on the value —
`Value::Array`, or `Value::Record` named `Range` with integer `start`/`end`. The checker guarantees
no other record named `Range` reaches it. It iterates by count, computed once as
`end - start (+ 1)` clamped at zero, so a closed range ending at the carrier's maximum does not
overflow, and it never builds the integer list; each body evaluation still passes
`check_operation_limit`.

IR: the builder emits `forRange` (kind 22, same layout as `for`) for the recorded `ExprId`s and
adds `ranges-v1` in `required_features`. `explain` renders it `for i in <range> yield`. The
conformance manifest gains a coverage entry for the kind, which forces a corpus program.

*Why a new kind rather than widening `for`:* the format says an intentional change to how a runtime
evaluates an image bumps the ABI, while a new kind behind a feature is the documented way to grow.
It also keeps `$type`-sniffing out of the runtime's hot `for` path.

Executable TypeScript: `emit.rs` emits `nxRangeMap(range, (item, index) => body)` and collects the
helper as it does `nxIntDiv`.

TypeScript runtime: `forRange` evaluates the iterable, validates a `Range` record with
safe-integer bounds, computes the count, refuses a count above `NxRuntimeOptions.maxRangeLength`
(default 1,000,000, the interpreter's `max_operations`) with `nx-ir-resource-limit`, then loops.

### D5: The prelude is a real linked IR module, and the runtime ships its image

The IR builder treats the prelude as the library module it is: a reference to `Range` is
*(prelude slot, "Range")*, the prelude gets a module-table entry, and its declarations live in its
own image and nowhere else. This needs no new builder code beyond giving the module the reserved
identity instead of a path. Its module-table version is empty, as a library module's already is, so
the link-time version check passes and compatibility is decided by which declarations the runtime's
prelude holds: a newer runtime runs older images as the prelude grows, and a real gap is
`nx-ir-link-missing-declaration`, naming the prelude and the declaration.

`selected_modules` emits the prelude when it is named and, for an every-module request, when some
module references it, so `nxlang codegen --target nx-ir` writes `@nx__prelude.nxir` beside a
program that needs it and nothing extra beside one that does not.

Hosts still do nothing, on the protobuf well-known-types model. `@nx-lang/ir-runtime` gains a
generated `src/prelude-image.ts` holding the image as base64, and `tryLinkNxIrProgram` wraps the
host's `resolve`: host first, then the built-in for the prelude's identity, prepared once and cached
at module scope. `prepareNxIrProgram`'s resolver-free path gets the same fallback, so "self-contained"
comes to mean "needs nothing but the prelude". The image is produced by an `nx-codegen` test that
rewrites the file under `NX_UPDATE_PRELUDE_IMAGE=1` and otherwise fails, naming that command, when
the file differs from what the compiler emits — the corpus pattern.

*Why not inline prelude declarations into each referencing image* (the first draft of this design):
inlining is sound only for records, whose values carry nothing but `$type`. A function value is
`{ module, name }`, so copies of one prelude function in two images would be unequal values, and
every image would carry every helper it calls. The checker already describes the update intrinsics
as future prelude signatures, so the prelude's first growth would have forced this design anyway.
Inlining also needed a slot-0 special case, transitive copying and a debug-span exception, against
no builder changes here.

*Alternative considered:* have the compiler hand every host the prelude image to resolve. Rejected:
the playground, the fiddle in another repository, and every `prepareNxIrProgram` caller would each
have to learn about it. Any other runtime can still get the image by naming it in `emit`.

### D6: C# gets an SDK type, TypeScript gets a generated one

Each language keeps its existing pattern. C#: `csharp_type_inner` maps a name that resolves to a
prelude declaration to `global::NxLang.Nx.NxRange<…>`, beside the `NxFunctionRef` arm, and
`NxLang.Sdk` gains `NxRange.cs`. It is `NxRange`, not `Range`, because generated files open with
`using System;`. An SDK test builds the prelude, reads `Range`'s fields from the artifact, and
compares them with `NxRange<T>`'s members, so the two cannot drift silently.

TypeScript: the export model takes the prelude library as a source of `ExportedRecord`s, and the
emitter writes the referenced ones through the ordinary record emitter into `_nx.ts` (library
output, imported by each module that needs them and re-exported from `index.ts`) or inline
(single-file output). Generated TypeScript keeps having no package dependency.

In both, resolution goes through the module's own namespace first, so a module that declares its
own `Range` generates its own.

*Alternative considered:* emit `Range<T>` into every C# output too. Rejected: two generated
libraries in one namespace would both declare it.

### D7: The language service needs almost nothing; the playground needs one line

Completions and hover read `prepared_bindings` and `visible_libraries`, both of which include the
prelude after D1, with source text available for declaration-kind heuristics. The LSP builds with
an empty context and still gets the prelude, because D1 does not go through the context. A
declaration whose origin is the prelude hovers as `(built-in type)`, the label `Element` already
has, with its real signature beneath.

*Terminology.* "Prelude" is the technical name — the module in scope without an import, as in Rust
and Haskell — and is the word for specs, code and the modules reference page. Everywhere a user
meets `Range` first (hover, the `for` and types pages) it is a "built-in type", which a C# or
TypeScript reader needs no glossary for. "std" is avoided: NX is pure and host-embedded, and the word
promises a large importable library. The reserved `@nx/` root leaves room for one: modules that need
an explicit import could later sit beside `@nx/prelude.nx` with nothing renamed.
`sites/playground/src/compile/catalog.ts` `classify` gains a case so a label whose file is
the prelude's identity is not handed to the visitor as a span in their own document. Its resolver
is untouched: it returns nothing for the prelude and the runtime's fallback serves it, for the
snippet and for the catalog alike.

### D8: The host text-prepending "prelude" is retired, replaced by context documents

`packages/language-core/src/prelude.ts`, the `prelude` option of `createSnapshotLanguageService` and
`createNxLanguageHandler`, `PRELUDE_ORIGIN`, and the position shifting in `answer.ts` are deleted.
In their place both factories take `context: { documents, implicitImports }`: the documents join
every query's document set, and the identities go to the snapshot as implicit imports. Nothing is
shifted, because the queried text is analyzed as written, and a diagnostic in a context document is
reported under that document's URI — which is what the playground's worker already does through
the wasm SDK. The Node SDK's `NxLanguageSnapshot` gains `implicitImports`, plumbed to the
`with_implicit_imports` the native snapshot already has access to through its build context. The
wasm test that used the HTTP handler's text prelude as its reference switches to the new option, so
it keeps comparing two transports over one mechanism.

*Alternative considered:* rename it. Rejected: it has no remaining production user, and implicit
imports give strictly better answers, since positions are never rewritten.

## Risks / Trade-offs

- [Every build now analyzes one more library] → Built once per process behind a `OnceLock`; the
  prelude is three lines. A benchmark in `nx-api` guards single-source build time.
- [The rewrite allocates an expression after checking] A consumer keyed by `ExprId` could miss the
  new literal's type. → The pass records it, and an IR test and an executable-codegen test cover a
  range in each position a record construction can occupy.
- [The runtime's prelude image can drift from the compiler's] → The staleness test fails the build,
  releases ship compiler and runtime together, and at run time a gap is a named
  missing-declaration diagnostic rather than a wrong answer.
- [Images that use the prelude no longer load on a runtime that lacks the fallback] The released
  0.2.0 runtime reports `nx-ir-link-missing-module` for the prelude. → Accepted; backward
  compatibility is not a goal yet, and the diagnostic names the module.
- [An emittable identity must be a file name] → `@nx/prelude.nx` is legal on every platform once `/`
  is written `__`; a CLI test writes and re-reads it.
- [The interpreter recognizes `Range` by name] → Sound only under D2; an interpreter test asserts a
  module with its own `Range` cannot reach `eval_for` with it, because the checker rejects it first.
- [`maxRangeLength` is a new failure mode] A legitimate large loop now fails under defaults in the
  TypeScript runtime only. → The default equals the interpreter's operation budget, the diagnostic
  names the option, and the docs say so. Generated executable code stays unlimited (Non-Goals).
- [Checker tests in `nx-types` run without `nx-api`, so without a prelude] → A range expression
  there reports `range-hidden`'s sibling, "the prelude is not available"; range typing tests live in
  `nx-api`, where builds are real. `nx-types` keeps one test for the missing-prelude diagnostic.
- [Float ranges are constructible but inert] `0.0..1.0` can only be passed along or read. → That is
  the slider use case; membership and clamping are listed as follow-ups.
- [Retiring the text prelude breaks `language-http` hosts that pass `prelude`] → The option is
  removed, not ignored, so such a host fails at construction with a message naming `context`; the
  README shows the two-line migration.

## Migration Plan

No source changes meaning, and the IR schema stays 4. An image that uses the prelude needs a runtime
with the built-in prelude; one that does not is unchanged. `@nx-lang/ir-runtime`, `NxLang.Sdk` and the compiler ship together as
usual; a host on an older runtime gets `Unsupported NX IR required feature 'ranges-v1'`, by name.
`parser.c` and the other generated grammar files are regenerated with `grammar.js`. Conformance
`expected/` files for existing programs must not change; a diff there is a bug in D1 or D5. Hosts of
`@nx-lang/language-http` or `language-core` replace `prelude: { source }` with
`context: { documents, implicitImports }`.
