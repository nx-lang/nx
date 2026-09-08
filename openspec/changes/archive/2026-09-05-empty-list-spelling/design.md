## Context

See `proposal.md` — Why. The constraints that shape the approach:

**`{` has exactly one meaning in value position.** There is no record-literal rule in
`crates/nx-syntax/grammar.js` — no `record_literal`, `record_construction`, or `object_literal` —
and no block form. Records are constructed element-style (`<T ... />`). The named-literal spelling
that `record-construction-validation` uses in a scenario, `ChatLinkConfig { accentColor: "..." }`,
is in fact a parse error against the grammar as written. So an empty brace pair in value position
has nothing to be confused with.

**Admitting the empty form costs no ambiguity.** Wrapping the `choice` in `values_braced_expression`
in `optional` and regenerating produces a conflict set identical to today's. Against the modified
parser, `{}` goes from `MISSING identifier` error recovery to a clean empty `values_braced_expression`
in plain `let`, annotated `let`, attribute, and markup-child position; every existing form —
`{a}`, `{a b}`, `{(a + b) c}`, `type T = { x:string }` — parses identically.

**The change does not reach element braces.** `elements_braced_expression` is a separate rule over
`elements_expression`, which is a `repeat1`. Editing the values rule leaves it untouched:
`<div>if r {} else { <B/> }</div>` and `<div>for x in xs {}</div>` remain parse errors before and
after. Embedded `@{}` is a third rule and is likewise unaffected.

**The type system has a top and no bottom.** `primitive-type-names` fixes the primitive set at nine
names with `object` at the top; there is nothing below. `common_supertype`
(`crates/nx-types/src/semantics.rs:5`) ends in `Type::named("object")`, and `is_compatible_with`
(`crates/nx-types/src/ty.rs:330`) has no bottom case. NX also has no generics — no `type_parameter`
in the grammar — so of the two mechanisms other languages use to make an empty list universally
assignable, parametric polymorphism is unavailable and the other, a bottom element type plus list
covariance, is half-present: arrays are already covariant in `type_satisfies_expected`, and only the
bottom is missing. D10 supplies it.

**Reading the expected type is established machinery.** A scalar coerces to a one-item list at a
list-typed site; an integer literal takes `float32` at a `float32` site; a bare `cover` resolves
against the declared type of the site rather than lexical scope. Typing a value by its site is a
pattern NX already relies on in three places.

**`Type::Void` is live, its spelling is not.** Inference assigns it at
`crates/nx-types/src/infer.rs:316` (an `if` with no `else`), `:426` (a block with no trailing
expression), `:722` and `:725` (a match that may match nothing). No `.nx` file in the repository
writes `void`.

## Goals / Non-Goals

**Goals:**

- An empty list can be written, parsed, type checked, and rendered, and rendering round-trips.
- The spelling continues an existing rule rather than introducing a parallel one.
- What NX source can spell matches what an author can usefully say.

**Non-Goals:**

- A bottom type *spelling*. The type itself is added — see D10 — but it stays inference-internal,
  exactly as D6 argued it should.
- Fixing the `common_supertype` treatment of `Type::Void`. See Open Questions.
- A general list literal syntax. See D1's rejected alternative.
- Empty element bodies or empty interpolations. See D4.
- Removing `Type::Void` or changing where inference assigns it.

## Decisions

### D1: Spell the empty list `{}`

**Decision.** `ValuesBracedExpression` admits zero entries.

**Why.** It is the only candidate that continues an existing rule rather than adding one. The braced
form already scales by arity — `{x}` is one item, `{x y}` is two — and `{}` is the same form at zero.
Nothing in the grammar competes for an empty brace pair in value position (see Context), and
measuring the change rather than reasoning about it confirms it: the regenerated conflict set is
identical, and no existing form's parse changes.

**Alternative considered — a dedicated `items=[]` literal.** Rejected. `[]` is type-suffix syntax
today (`UserBase[]`), which is survivable, but the real cost is that it would be a second list
syntax reachable *only* in the empty case: `[]` legal, `[a b]` not, `{a b}` required. A reader has
no way to make sense of that. `[]` is the better answer only if `[...]` is meant to become NX's
general list literal, which is a much larger change and is not proposed.

**Alternative considered — omit the property.** Rejected as the general answer, as the original
proposal already noted: it is sound only where the declared default is the empty list, and an
explicitly empty list at a property with a non-empty default is exactly the case that must
round-trip.

### D2: State that zero items is list-valued; do not derive it

**Decision.** `braced-value-sequences` gains an explicit rule that a zero-item braced expression is
list-valued.

**Why.** It does not follow from the existing arity rule and would be wrong if left to it. One item
infers a *scalar* — `{1}` is `int`, not `int[]` — so the existing text reads "one is scalar, more
than one is a list", and a reader extending that downward has no reason to land on "list". Zero is
the one arity that cannot be scalar, and that has to be said rather than implied.

### D3: `{}` is a `never[]`; an unannotated binding is still reported

**Decision.** A zero-item braced expression has type `never[]` — a list of the bottom type — which
is its type outright, not a placeholder for one a site has yet to supply. `never` is below every
type, so `never[]` is below every list type, and one value is usable at every list-typed site.

A binding whose type is *fixed* by an empty list is still reported: `let a = {}` and
`let f(x) = {}` each name the binding and ask for the annotation. That is a legibility rule, not a
typing failure — the system can type them, but a signature saying "a list of nothing in particular"
tells the next reader nothing about what it is a list of.

**Why the bottom type.** The alternative — leaving the element an inference variable and taking the
real type from context at each site — is what this change first implemented, and it does not work
in one place. A variable satisfies nothing and joins to `object`, so *every* site that could
conclude a binding has to be taught to resolve it by hand before analysis ends, and any site that
concludes without doing so leaves either a spurious diagnostic or a live variable. Six of the twelve
review findings on this change were exactly that failure, at six different sites. `never` needs no
resolution because it is already the right type: satisfaction and joining handle it, and there is
nothing to track. See D10.

**Why report the unannotated binding anyway.** `object[]` as an inferred type would be actively
wrong — it is the *top* of the element lattice, so it is not assignable to `string[]` — and that is
what D3 originally rejected. `never[]` has neither problem. What remains is a readability question,
and it is answered the same way: the annotation is required, and the diagnostic names the binding.
C# 12 rejects `var x = [];` (CS9176) and Rust asks for annotations on `Vec::new()`; TypeScript
accepts `const a = []` and infers `never[]`. NX types it like TypeScript and reports it like C#.

**Recovery.** Both sites report and then keep the type they inferred; neither poisons to `Error`.
That follows from the rule being about legibility: the system *can* type the binding, so it types
it, and the one diagnostic is the whole complaint. The consequence for a value binding is that an
unannotated `{}` goes on inhabiting every list type after it is reported — `let a = {}` satisfies
both `string[]` and `int[]` — and that a later genuine mismatch against a non-list site is still
reported rather than being suppressed by an error type.

The two sites recover the same way, but only the value binding's recovery is observable downstream.
An unannotated function's return type reaches its call sites as a bare type variable, so
`let f() = {}` used at a `string[]` site reports there as well. That is not this rule's doing and
not specific to empty lists: `let f() = {"a"}` fails at a `string[]` site identically, on `0fc0463`
as much as here. It is a separate defect — a type variable escaping into a user-facing message,
which is the very class of leak `empty_list_display` exists to prevent — and belongs to its own
change rather than to this one. It is filed as `infer-unannotated-return-types`.

**Note.** The motivating case never reaches the diagnostic: the formatter emits `items={}` at a
typed property site, which is a binding whose type the annotation already fixed.

### D4: Scope the change to `ValuesBracedExpression`

**Decision.** `ElementsBracedExpression` still requires at least one item, and `@{}` stays rejected.

**Why.** Neither is needed by the empty-list problem, and each raises a question of its own. An
empty *element* body is about control flow producing no elements, which `if` without `else` already
expresses; changing it would be a second design decision riding on the first. An empty
interpolation, `@{}`, is not something an author means to write, and admitting it would only create
a way to write nothing where nothing is already the default. Leaving both alone also keeps the
change measurable: element braces are a separate rule, so the parse evidence for the values rule is
evidence for the whole change.

**Consequence to state.** `<List>{}</List>` *does* become legal, because markup child content takes
a `values_braced_expression`. That is coherent — an empty list of children is no children — and the
spec claims it deliberately rather than acquiring it silently.

**Third consequence, found in review.** The empty form reaches *every* rule that references
`ValuesBracedExpression`, not the two enumerated here: a value-position `if` branch, a
value-position `for` body, and match and condition arm bodies are all that rule. Each parses `{}`
where it did not before. None of them declares a type, so what an empty list means there had to be
decided rather than inherited — see D9.

**Second consequence, found while implementing.** A function body is also a
`values_braced_expression`, so `let f():string[] = {}` becomes legal too — a function that returns
an empty list. That is the rule working, not a leak: the body is a value expression like any other,
and it is accepted only where a list return type is declared. The case it changes is the *empty*
body, `let <f /> = { }`, which was a parse error (recovery inserted a `MISSING identifier`) and is
now an empty list. It is still rejected, by D3's diagnostic rather than by the parser, because an
unannotated binding supplies no element type. Component bodies are unaffected — they are
`ElementsBracedExpression` — so `component <N /> = { }` remains a parse error.

### D5: Remove `void` from the source surface; keep the internal type

**Decision.** `void` leaves `primitive_type`, `builtin_type`, and the editor completions. `Type::Void`
is unchanged, keeps its inference sites, and keeps rendering as `void` in diagnostics.

**Why.** The name is offered and never used, and there is nothing for an author to use it for:
functions are expression-bodied so they always produce a value, and NX has no diverging expression.
The internal type is doing real work at four inference sites and none of that work requires a
spelling. `primitive-type-names` is a spec whose entire purpose is a closed, minimal set of names,
so a name in that set that no source names is precisely the thing it exists to prevent.

**Why the diagnostic keeps saying `void`.** Rendering a type the author cannot write is not a
contradiction: the author does not write it, they receive it. `Type::Void` needs *a* rendering, and
`void` is the accurate and conventional one. Inventing a second name to avoid the overlap would make
diagnostics worse to serve a consistency nobody is reading for.

### D6: Do not rename `void` to `never`, and do not spell `never` in source

**Decision.** No reserved word and no replacement spelling. A bottom *type* is added — D10, which
this decision anticipates and constrains — but it is inference-internal, so `never` stays as
unspellable in NX source as `void` becomes.

**Why not rename.** It would rename the wrong thing. Two of `Type::Void`'s four sites — an `if` with
no `else`, a block with no trailing expression — are genuinely *unit*: they complete normally and
produce a value. A bottom type must be uninhabited, so calling this one `never` would make the name
false. The rename is also self-defeating as a reservation: it reserves `never` by spending it on the
type that already exists.

**Why not reserve the word.** An unreserved unused word and a reserved unused word behave
identically until one is needed, and the reserved one costs spec text. The argument for reserving is
that a user could declare `type never = ...` and be broken later — real, since `primitive-type-names`
explicitly permits user declarations of non-primitive names like `i64` — but `AGENTS.md` states there
is no backward-compatibility requirement yet, so the insurance and the risk are both cheap. Adding
the internal type in D10 does not spend the word: `never` is absent from `primitive_type` and from
`builtin_type`, so `type never = { ... }` resolves to the user's declaration exactly as `type void`
does.

**And a bottom type would need no syntax anyway.** NX has no `throw`, `panic`, or `raise`, so no
expression in the language can *have* bottom type. The only user-facing slot is a function return
annotation, and `let f(x:int): never = expr` is meaningless without a diverging body. If NX ever
wants a bottom type it wants an inference-internal one — exactly what `Type::Void` already is — and
that needs a variant in `ty.rs` and a case in `common_supertype`, not a keyword.

**And that is what D10 does.** The empty list turned out to need one within this change, and it
needed precisely the variant and the join case named above — no keyword, no reservation, no rename.

### D7: Ship both halves as one change

**Decision.** The empty-list spelling and the `void` removal land together.

**Why.** They are the same question asked twice: does what NX source can spell match what its type
system holds? One half adds a spelling for a value that had none; the other removes a spelling for a
type that needs none. They were found in one investigation, they touch the same two documentation
files (`nx-grammar.md`, `nx-grammar-spec.md`), and reviewing them together is what makes the
symmetry visible.

**Trade-off accepted.** The change's name describes only the first half. They have no code
dependency and can land in either order, so splitting them costs nothing but a second set of
artifacts if that is preferred later.

### D8: A call argument takes a braced value; a list item still does not

**Decision.** `call_expression` draws each argument from a hidden `_call_argument` rule,
`choice(value_expression, values_braced_expression)`, so `f({})`, `f({a})`, `f({a b})` and
`f({}, x, {a b})` parse. `value_list_item_expression` is untouched, so a brace is still not an item
of a brace at any arity.

**Why here.** The empty-list work exposed the asymmetry rather than created it, but it is the reason
the asymmetry now bites: `f({})` is the obvious way to pass an empty list, and the workaround — bind
it to an annotated `let` first — exists only because the argument position could not supply the type
the annotation supplies. A parameter is a binding site with a declared type, exactly like a property;
`infer_call` already checks each argument through `check_typed_binding_for` with the parameter type,
which is the same seam a property binding uses. So the type system was already ready for this and
only the grammar was not.

**Why not a wider rule.** Adding `values_braced_expression` to `value_expression` would have reached
the argument position too, but also parenthesized expressions, binary operands, member-access
targets, and conditional arms, and would have made `{ ({}) }` legal. A rule scoped to the argument
list adds exactly the position that was asked for. Regenerating the parser produces a conflict set
identical to the one before the edit, order-insensitively, so the narrow rule costs no ambiguity.

**Why not nested braces.** `{{"a" "b"}}` and `{{}}` stay parse errors, and not because they are hard
to parse. At arity one the brace is a scalar, so a nested brace collapses: `{{"a" "b"}}` would mean a
scalar holding a `string[]` — the outer brace a no-op — and `{{}}` would mean what `{}` means. A
two-row `string[][]` would be writable as `{{"a"} {"b"}}` and a one-row one would not, silently,
because the collapsed value still binds at a `string[]` site. That hole cannot be closed without
giving up the arity rule, and the arity rule is what makes `{name}` an expression escape everywhere
else. A list of lists wants a distinct literal whose delimiters carry no escape meaning, not a
second brace.

**Consequence to state.** Uncheckable calls needed one fix. `infer_call` returns early on an error
callee, a wrong argument count, and a non-function callee, each after a diagnostic exists for the
site. An empty list among the arguments would still be sitting in `pending_empty_lists` at those
exits and would draw a second diagnostic telling the author to annotate a binding — the same
spurious-follow-up shape D3's sweep has at an annotated non-list binding. All three exits now
discharge the argument's pending entries, so a broken call reports once.

### D10: Add `never` as an inference-internal bottom type

**Decision.** `Primitive::Never` joins `Primitive::Void` as a type inference assigns and source
cannot spell. Two rules carry it: `never` satisfies every expected type, and joining `never` with
anything yields the other side. `{}` infers `Array(Never)`.

**Why here rather than later.** It is not an addition to this change so much as the correct form of
something this change already wrote. Admitting `{}` required a rule that an empty list satisfies any
list type, and a rule that an empty list joined with a list yields that list. Those *are* bottom-type
subtyping and bottom-as-join-identity — they were written as special cases over `Array(Variable)`,
a shape used as a nonce for `never[]` and detected by a structural test. Landing the nonce and
replacing it afterwards would mean shipping specification, tests and a `pending_empty_lists`
apparatus built only to compensate for the nonce not being a real type.

**What it removed.** The map of lists owing an element type, the end-of-analysis sweep that reported
them, the discharge obligation at every concluding path, and the three helpers that met it: 45
references in `infer.rs` reduced to 12, of which nine are rendering and three are D3's diagnostic.
The two mechanisms it needs already existed — arrays are covariant in `type_satisfies_expected`, and
any array already satisfies `object` — so `never` reaches every site it needs to through rules that
were there before.

**Why not a keyword.** Unchanged from D6, which anticipated exactly this: "If NX ever wants a bottom
type it wants an inference-internal one — exactly what `Type::Void` already is — and that needs a
variant in `ty.rs` and a case in `common_supertype`, not a keyword." There is still no expression in
NX that has bottom type, so there is nothing an author could write it on. `never` is not in
`primitive_type` and not in `builtin_type`, so a user may declare `type never` exactly as they may
declare `type void`.

**Why not a rename of `Void`.** Also unchanged from D6. Two of `Void`'s four inference sites are
genuinely *unit* — they complete normally and produce a value — so calling that type `never` would
make the name false. `Never` is a new variant beside it, and `Void` keeps every site it had.

**Consequence to state.** No value has bottom type, so `never` never reaches the interpreter, the
FFI, or a runtime type test — a strictly smaller surface than `Void`, which does reach all three.
The only backends that can see it are the ones that render inferred types, which is `nx-codegen`
(TypeScript, JavaScript, NX IR); both places it appears there are filled in, and TypeScript spells
`never` natively. The C# and TypeScript *typegen* backends map from source type annotations by name,
so they cannot reach it: `never` has no source spelling to map from. The host-without-a-bottom-type
problem therefore does not arise, rather than being solved.

## Risks / Trade-offs

**`{}` is visually light — an empty property value is easy to miss when skimming** → Accepted. It is
the same weight as the non-empty form it generalizes, and any heavier spelling reintroduces D1's
second-syntax problem. Nothing about it is ambiguous to a reader who stops on it.

**Removing `void` from the grammar frees the name, so a user may declare `type void = { ... }`** →
Intended, and identical to how `bool`, `float`, and `f64` already behave after their rename. The
hazard is narrow but specific: the primitive-name-to-host-type maps in `crates/nx-codegen/src/emit.rs`
and `crates/nx-cli/src/typegen/languages/` still contain a `"void"` entry, so a user type named
`void` must not reach them and be silently emitted as host `void`. Task 4.4 covers exactly this.

**A diagnostic can name a type the author cannot write** → Accepted, per D5. It was already true in
practice, since no source named `void`.

**Someone reads "zero items is a list" and expects `{x}` to be a one-item list** → The arity rule is
genuinely irregular, and this change makes the irregularity visible rather than creating it. The
requirement states all three arities together so the shape is read at once.

**The empty-list diagnostic fires somewhere the expected type ought to have been available** →
Retired by D10. It was the dominant risk while the element type had to be threaded to each site by
hand, and six review findings landed on it. With `never[]` there is no threading and no site-by-site
plumbing: satisfaction and joining carry the type, so a site cannot fail to pass along something it
is never handed.

**`never` is below every type, so an empty list satisfies every list type** → Intended, and sound
because NX lists are immutable — the generated TypeScript is `readonly T[]`, and no NX expression
writes into a list. Array covariance is unsound only for mutable arrays, and there is nothing to
write. The one place it is visible is that a single `{}` can be bound at a `string[]` site and an
`int[]` site in one program, which is correct: it is the same empty list, and it is a member of both.

**A binding whose type an empty list fixed is reported even though the system can type it** →
Accepted, and it is the one deliberate strictness left in D3. `let a = {}` could infer `never[]` and
work; it is reported because the annotation is what tells the next reader what the list is of. The
cost is one rule an author can be surprised by; the alternative is signatures that say nothing.

## Migration Plan

No migration, in either half.

The empty-list half is purely additive to what *parses*: every program that compiles today still
compiles.

One program shape changes meaning, and it is worth naming because it contains no `{}`. Binding the
empty list for a body that produced no values is a rule about the body, not about the spelling, so
an element whose body is a `for` over an empty list now binds no children where it used to fall back
to the content property's declared default. The alternative — keying on `{}` itself — would make two
bodies that evaluate identically mean different things, which is worse than the change. Reviewing
the repository's 111 `.nx` files found no source that hits it, and no valid program's output
changes.

One already-invalid file does change: `src/vscode/samples/tally-survey.nx`, which uses an
unsupported positional attribute form and fails to parse both before and after, goes from 21
diagnostics to 35. Admitting `{}` makes `{` followed by `}` a valid parse, so error recovery takes
a different path through the malformed input. No valid program is affected, so this does not
weaken the migration position — but the claim is "no valid program changes output", not
"byte-identical output", and the difference is recorded rather than rounded off. Task 10.8 carries
the detail.

The `void` half removes a spelling, which is breaking in principle and inert in practice — no `.nx`
file in the repository names `void`, so there is nothing to update. Should a source outside the
repository name it, the failure is a clean unresolved-type diagnostic at the reference, not a silent
change of meaning.

Rollback of either half is reverting it; neither leaves data or generated artifacts behind.

## Open Questions

- **`common_supertype` has no case for `Type::Void`, so it falls through to `object`.** Inference
  pushes `Type::Void` into `result_tys` at `infer.rs:722` and `:725` and joins; `is_compatible_with`
  has no void case either, so neither direction satisfies and the join lands on
  `Type::named("object")` (`semantics.rs:32`). A condition-list match with no `else` — `if a => "x",
  b => "y"` — therefore infers `object` rather than `string`. This is the unit-where-you-want-bottom
  problem in the existing code, and it is what a bottom type would be *for* (D6). It is deliberately
  not fixed here: it changes inferred types for programs that compile today, so it needs its own
  change, its own before/after evidence, and a decision about whether the honest answer is the arms'
  type, a nullable of it, or an error at the binding site.
- Whether `ElementsBracedExpression` should later admit an empty body (D4). It would want a reason
  of its own; `if` without `else` covers the case today.
- **`T[][]` is a type no literal can write** (D8). A host can hand a nested list across the FFI
  boundary — `nx-api`'s value conversion round-trips one — and the type annotation `string[][]` is
  accepted. Review found that NX source can construct one after all, through a value-position `for`
  with a multi-item body: `let xs:string[][] = {for y in ys {y y}}` type checks and evaluates
  today, before this change, and `{for y in ys {}}` is its empty case. So the gap is narrower than
  first stated — a nested list has no *literal* spelling, which is why first-party formatting
  reports one rather than rendering it, but it is reachable. The answer is a distinct list literal
  rather than a nested brace, and it is not this change.
