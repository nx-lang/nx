# Review: add-with-expressions (proposal)

Reviewed: proposal.md, design.md, tasks.md, specs/with-expressions, and the deltas to environment-values and
nx-ir-format, against the main specs (environment-values, nx-ir-format, update-records,
component-instance-tree, executable-code-generation, rust-ir-runtime, typescript-ir-runtime,
editor-language-service, editor-syntax-highlighting), `crates/nx-syntax/grammar.js`,
`docs/nx-ir-format.md` and the current implementation. `openspec validate add-with-expressions --strict`
passes. Every requirement has SHALL on its first line. The MODIFIED blocks repeat the full main requirement,
all of its original scenarios included, and the REMOVED header matches the main spec's name exactly.

## Findings

### ✅ Verified - RF1 Precedence 125 is already taken by `??`, which is right-associative
- **Severity:** High
- **Confidence:** High
- **Evidence:** crates/nx-syntax/grammar.js:687 (`[prec.right, 125, '??']`); design.md:131-134;
  specs/with-expressions/spec.md:62-65 (which names unary, member access, calls and `*` but not `??`).
- **Recommendation:** Give `with` its own level. For example, use 127 so that `a ?? b with c` is
  `a ?? (b with c)`, or 123 so that `with` binds less tightly than `??`. Then state how it relates to `??` in
  the design and in the precedence requirement, and add a scenario for each mixed form (`x ?? y with { … }`,
  `x with y ?? z`). At the same level, tree-sitter would have to resolve a left-associative and a
  right-associative rule against each other. It would either report a conflict or pick an order that the
  spec doesn't state. Separately, the right operand is restricted to name, member access, call, element or
  braced expression, so the spec should also say whether `x with y ?? z` is rejected or parses as
  `(x with y) ?? z`.
- **Fix:** `with` is now left-associative at precedence 127, above `??` (125) and `*` (120), below unary (130). The with-expressions precedence requirement is retitled and has a `??` scenario; design decision 2 lists the readings; task 1.1 names the levels.
- **Verification:** `with` is at 127, between `??` (125) and prefix unary (130), so it no longer shares a level with a rule of the other associativity. The requirement (specs/with-expressions/spec.md:65-69), design decision 2 and task 1.1 agree on both mixed readings. The `saved ?? fallback with { name="Bo" }` scenario checks `a ?? (b with c)`.

### ✅ Verified - RF2 Formulas bring back reads that have no source left, and the cycle rules don't cover them
- **Severity:** High
- **Confidence:** High
- **Evidence:** specs/environment-values/spec.md:244-246 ("a read of that same value SHALL see the sources
  after it"); openspec/specs/environment-values/spec.md:247-253 (`environment-cycle` follows defaults only,
  and "a provide cannot loop"). The change doesn't modify that requirement, yet tasks.md 2.3 says bundles
  are counted in `environment-cycle`.
- **Example:** `color`'s default reads `env.colorMode`, and a body renders
  `<A env.colorMode={ if env.color == x { dark } else { light } }>`. Inside `A`, a read of `color`
  evaluates the default, which reads `colorMode`, so `A`'s formula runs and reads `color` while `color`'s
  default is being evaluated. Under the stated rule that read sees "the sources after" the default, and
  there are none. Under the old rule, `A`'s value was computed outside `A` and this couldn't happen.
  Overrides are placed dynamically, through descriptors that are built in one body and placed by another,
  so the static check can't find every such case.
- **Recommendation:** Say what that read gives: a runtime error with a named diagnostic, or the
  declaration's default evaluated without that read. Also say whether "sees the sources after it" is a
  dynamic rule (any read nested anywhere inside the evaluation of a source of X) or applies only to the
  source's own expression. Modify "A default that depends on itself is rejected" so that it covers override
  formulas and bundles, as task 2.3 already assumes, and add a conformance scenario for the case.
- **Fix:** Added the rule "while a source of `V` is evaluated, a read of `V` at any depth sees only the sources after it" (environment-values ADDED requirement, nx-ir-format lookups). The static `environment-cycle` check now covers an override, bundle entry or main-module provide of `V` that reads `W` whose default reads `V`, and a runtime `nx-ir-environment-cycle` covers what placement reveals. Scenarios for both; design "Cycles" paragraph; tasks 2.4, 4.1, 6.1, 6.2.
- **Verification:** environment-values now has a MODIFIED copy of "A default that depends on itself is rejected" (spec.md:281-310). It adds the static check for an override, bundle entry or main-module provide of `V` that reads `W` whose default reads `V`, plus the runtime `nx-ir-environment-cycle`, and both have scenarios. "At any depth" settles the dynamic reading in environment-values (spec.md:320-322) and in nx-ir-format's lookup (spec.md:125-128). I checked the other shapes: two overrides that read each other, and a provide of `V` reading `W` reading `V`. Under the at-any-depth rule both terminate, so this is the only shape that can loop. Tasks 2.4, 3.1, 4.1, 6.1 and 6.2 cover it.

### ✅ Verified - RF3 A bundle held in a local, prop or loop item is still evaluated where it was built
- **Severity:** High
- **Confidence:** High
- **Evidence:** docs/nx-ir-format.md:709 (a bundle evaluates to `{ "$type": "env.Update", "$entries": […] }`,
  with each entry's `to` a value); design.md:175-182 ("the locals it captured, which are data… Inline and
  named bundles therefore behave the same"); specs/environment-values/spec.md:240-249.
- **Example:** `component <Banner theme: env.Update /> = { provide env with theme { … } }`, or
  `for theme in themes { provide env with theme { … } }`. The formula is just the local `theme`. Its
  captured value is an already-evaluated bundle whose entries were computed where it was built, so a theme
  that varies by mode doesn't follow an `env.colorMode` set inside the section. That is the exact case the
  change sets out to fix. Replacing a top-level name with a parameter changes the meaning, which design.md
  rejects as an alternative at line 186.
- **Recommendation:** Pick one, and write it into "Every environment value is computed where it is read"
  with a scenario:
  - make an `env.Update` value itself lazy: entries hold formulas, not `to` values, which changes the
    bundle data shape;
  - or state the limit: a bundle passed as a value is fixed where it is built, and only a bundle expression
    written in the `provide` (names of top-level values, calls, elements) is evaluated at the read.
- **Fix:** `env.Update` values now hold their entries as `to` or `formula`, like `$env`, so a bundle passed through a prop, local or `for` item computes each entry where a value is read. New scenario with a bundle passed as a prop; design "Descriptors and bundles carry formulas"; nx-ir-format `$entries` shape.
- **Verification:** `env.Update` values now hold their entries as `to` or `formula` (environment-values spec.md:263-267; nx-ir-format spec.md:69-81). The prop scenario (spec.md:274-279) pins the case from the finding, and design decision 3 drops the "named bundles only" claim. See RF17 for what this means for generated code, where those entries become closures.

### ✅ Verified - RF4 The NX IR encoding of the new forms is missing from the format spec
- **Severity:** High
- **Confidence:** High
- **Evidence:** openspec/specs/nx-ir-format/spec.md:1881-1897 ("NX IR encodes environment values behind a
  required feature"). The change doesn't modify it. design.md:221-228; specs/nx-ir-format/spec.md:4-21.
- **Details:** Nothing normative defines any of the following:
  - the `envUpdate` `base` operand and its layout;
  - the layout of a bundle override inside `envProvide`'s override list, which today is `[ref, [str…], node]`
    and has no slot for "a bundle";
  - how the image marks an override as a formula or a value. The descriptor requirement makes this a SHALL
    ("An override whose value reads no environment value SHALL be evaluated once"), but a runtime can't
    know that without an emitter-computed flag. It also has to be static and agreed on, because it decides
    the `$env` shape and the operation counts;
  - when a module lists `environment-v2`, and whether it replaces `environment-v1` or comes in addition.
    design.md:226-228 says only formulas and bundles in blocks need it, but its list at 221 also puts the
    `base` operand "behind" v2. So a program that uses `brand with { … }` only at the top level is left
    ambiguous.
- **Recommendation:** Add a MODIFIED copy of that requirement covering all four points, with scenarios: a
  bundle in a block, `envUpdate` with `base`, a formula override's flag, and a v1-only program unchanged.
  Settle whether "reads the environment" means the static read set, including calls and top-level values.
- **Fix:** nx-ir-format's environment requirement now encodes the formula flag (from the static read set), bundle overrides in `envProvide`, `envUpdate`'s optional `base`, and `environment-v2` (with v1-only scenarios). Design decision 7 lists the IR additions; task 3.1.
- **Verification:** The environment requirement is now MODIFIED in nx-ir-format (spec.md:3-61). It covers the formula flag and how it is derived, the bundle override layout, `envUpdate`'s `base`, and when `environment-v2` is listed alongside v1, with a scenario for each. The `base`-only ambiguity is gone: `base` alone requires v2. One loose end remains, about how the flag is encoded; it is RF18.

### ✅ Verified - RF5 Generated code can't produce the `$env` formulas that the runtimes produce
- **Severity:** High
- **Confidence:** Medium
- **Evidence:** openspec/specs/executable-code-generation/spec.md:883-891 (generated `$env` SHALL be what the
  runtime's `enter` reads, with results equal to the native runtime's); specs/nx-ir-format/spec.md:17-19
  (a formula is "the module identity and node of the value"); tasks.md 4.3 ("formulas (as closures)").
- **Details:** A closure isn't data, so it can't be equal to the IR runtime's canonical `{module, node,
  locals}`. A generated program has no node table that `enter` could evaluate from. The change has no
  delta for executable-code-generation.
- **Recommendation:** Add an executable-code-generation delta. Either define a generated formula encoding
  that `enter` understands, or exempt formulas from the "same `$env`" requirement and say what a host of
  generated code gets instead. Also cover design decision 8 (top-level `env.Update` values as functions)
  there. Decide whether a value whose type merely contains an `env.Update`, such as a record of light and
  dark bundles, is also emitted as a function.
- **Fix:** New executable-code-generation delta: a formula is a closure of no arguments over its locals; scenarios for a main theme following a local mode and a generated formula. Design decision 8; task 4.3 and 6.4 (removes the `darkAccent` divergence).
- **Verification:** There is now an executable-code-generation delta. It states that a formula is a closure of no arguments and that such an entry is meaningful only to the generated program that made it, which settles the conflict with "same `$env`". It also has scenarios for a generated formula and for the #75 `darkAccent` case, and task 6.4 removes the divergence. A residual issue: the delta says a top-level bundle loaded at module load "SHALL still follow the environment of each read". That holds only for entries whose own expression reads the environment. An entry that only names a top-level value, such as `accent={myAccent}` where `myAccent` matches on `env.colorMode`, still gets that value as it was at load, which is the existing "Generated values are evaluated at load" limit. Saying so in a sentence is enough. The new issues are in RF17. Follow-up: the executable-code-generation delta now says this (spec.md:20-22), and so does design decision 8.

### ✅ Verified - RF6 The runtime specs and the host `enter` API aren't updated
- **Severity:** High
- **Confidence:** High
- **Evidence:** openspec/specs/rust-ir-runtime/spec.md:863-871 and
  openspec/specs/typescript-ir-runtime/spec.md:1731-1739 (they accept modules that require `environment-v1`,
  and `Environment::root`/`enter` build an environment from root values and `$env`).
- **Details:** A formula or bundle carried in `$env` has to be evaluated with the program image at each
  read. Today an environment is an immutable value with no program, and `enter(descriptor)` takes only the
  descriptor. A malformed formula entry also needs a defined refusal (`nx-ir-boundary-type`?).
- **Recommendation:** Add rust-ir-runtime and typescript-ir-runtime deltas: accept `environment-v2`, say how
  `enter` and the environment reach the program needed to evaluate formulas, and validate formula and
  bundle entries.
- **Fix:** New rust-ir-runtime and typescript-ir-runtime deltas: accept `environment-v2`, evaluate formulas with the prepared program of the reading evaluation, so `enter(descriptor)` needs no program, and refuse a bad formula with `nx-ir-boundary-type`. Tasks 4.1, 4.2.
- **Verification:** There are now rust-ir-runtime and typescript-ir-runtime deltas. Both accept `environment-v2` and evaluate formulas with the prepared program of the evaluation that reads them, so `enter` keeps its signature. Both refuse an unevaluable formula with `nx-ir-boundary-type`. When that refusal happens is inconsistent across the deltas; that is RF19.

### ✅ Verified - RF7 Read tracking in the instance tree doesn't cover formulas and bundles
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** openspec/specs/component-instance-tree/spec.md:486-488 ("including those read while
  evaluating a default or an entry-module provide"); tasks.md 4.1 (it mentions formula reads, but no spec
  backs it).
- **Details:** A node that reads `env.gap` through an ancestor's formula `env.space * 2` has to be found when
  the root value of `space` changes. Under the current requirement it isn't, so `set_environment` leaves the
  node stale.
- **Recommendation:** Add a component-instance-tree delta that extends the sentence to formulas and bundles
  carried in `$env` or in `envProvide`. Add a scenario: a reader through an override formula renders again
  when a value the formula reads changes at the root.
- **Fix:** New component-instance-tree delta: reads made while evaluating a formula or bundle entry are recorded for the reading node, with the scenario "A reader through an override formula renders again". Task 4.1.
- **Verification:** There is a component-instance-tree MODIFIED requirement with the full original text and scenarios, extended to formulas and bundle entries. It adds the scenario "A reader through an override formula renders again" (`space` 4→6, `Cell` reads 12), which checks the index path.

### ✅ Verified - RF8 What evaluating a bundle for one value means is underspecified, and so is its cost
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** specs/nx-ir-format/spec.md:53-57 ("a bundle SHALL contribute the overrides it holds for the
  value"); specs/nx-ir-format/spec.md:74-79; design.md:225.
- **Details:** A read of `env.space` under `provide env with marketing` has to evaluate the whole bundle
  expression, every entry included, to learn whether it sets `space`. The spec doesn't say:
  - whether every entry is charged, or only the matching one;
  - whether the "sees the sources after it" rule applies to every value the bundle sets or only to the value
    being read. This changes results when one entry reads a value that another entry of the same bundle
    sets;
  - how `with` chains (`base`) are charged.

  Two runtimes could agree on results and still differ on operation counts.
- **Recommendation:** State the evaluation unit and its cost in the lookup and cost requirements. Add a
  corpus scenario in which a bundle sets two values and one entry reads the other.
- **Fix:** The lookup requirement now says a bundle contributes only its entries for the value being read, last first, and no other entry is evaluated; the cost requirement states the charges for formulas, capture and bundle entries. Scenario where one bundle entry reads another value the bundle sets.
- **Verification:** The lookup requirement (nx-ir-format spec.md:122-123) and environment-values (spec.md:319-323) now say a read evaluates only the bundle's entries for the value being read, and that reads of other values see every source, including sibling entries. The cost list covers `envUpdate`, `base`, capture and bundle entries. The `dense` scenario (`space=2 gap={env.space * 3}`, read 6) pins it. The corpus requirement lists that case.

### ✅ Verified - RF9 `env with marketing` outside a `provide` would parse, and the existing error isn't a syntax error
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** specs/environment-values/spec.md:167-169 (it expects "a syntax error at `env`");
  crates/nx-hir/src/scope.rs:544-546 and 458-462 (a bare `env` is reported as `reserved-name`, "read an
  environment value as `env.name`").
- **Details:** With a general `with_expression`, `{ env with marketing }` parses: `env` is an
  `identifier_expression` and the parser then accepts `with marketing`. The result is today's
  `reserved-name`, not a syntax error. The task list has no grammar step that excludes `env` on the left
  of `with`.
- **Recommendation:** Either change the scenario to the diagnostic that will actually be reported (better
  still, a dedicated message: "`env with` is allowed only after `provide`"), or add a grammar task and test
  that excludes it.
- **Fix:** `env with` outside `provide` is `reserved-name` with a message that it is written only after `provide`; the old main-module `provide brand.theme` is a syntax error that suggests `provide env with brand.theme`. Scenarios in environment-values; tasks 1.2, 2.3, 6.2.
- **Verification:** The spec now asks for `reserved-name` at `env` with a message saying `env with` is written only after `provide` (environment-values spec.md:152-153 and 184-187). That matches what the current bare-`env` path in scope.rs reports, with a dedicated message. Task 2.3 covers it.

### ✅ Verified - RF10 The bundle after `provide env with` is "any expression", but the grammar can't take one there
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:** specs/environment-values/spec.md:137 and 173; crates/nx-syntax/grammar.js:269-291
  (`provide_definition` restricts a bundle to `qualified_name | element | values_braced_expression`, and
  `provide_expression` overrides are space-separated `property_value`s followed by a braced body).
- **Details:** A bare binary expression (`provide env with a ?? b env.x=1 { … }`) is hard to delimit
  against the following overrides and the body. A field list or braced operand right before the body
  (`provide env with brand with { radius=4 } { … }`) depends on the first-entry detection.
- **Recommendation:** Specify the bundle as the `with` right-operand set, optionally followed by a
  `with` chain, with anything else braced. Add grammar scenarios for: a named bundle followed by attribute
  overrides; an inline element; a `with` chain followed by the body; and an empty field list before the
  body.
- **Fix:** The operand after `provide env with` is now the same operand set as `with`'s right side (name, member access, call, element, braced expression), optionally followed by more `with` steps, then overrides and the body. Design decision 2; task 1.2 lists the grammar tests.
- **Verification:** The block form restricts the bundle to `with`'s operand set plus optional `with` steps (spec.md:146-149), and task 1.2 lists the grammar tests I suggested. The top-level form still says "an expression of type `env.Update`" (spec.md:191-192). There is no body after it, so it can be parsed, but using the same operand wording would keep the two forms alike. This is optional. Follow-up: the top-level form now takes the same operand and optional `with` steps (environment-values spec.md:51-53).

### ✅ Verified - RF11 Editor, highlighting and migration specs and tasks are missing
- **Severity:** Medium
- **Confidence:** High
- **Evidence:**
  - openspec/specs/editor-language-service/spec.md:611-614: a hover scenario on `provide brand.theme`,
    which becomes a syntax error;
  - openspec/specs/editor-syntax-highlighting/spec.md:567-589: `provide` keyword scoping, with nothing yet
    for `with`;
  - the TextMate grammar: src/vscode/syntaxes/nx.tmLanguage.json:3191 (`provide-definition`);
  - src/vscode/test/grammar/environment.test.ts:45 (`provide brand.theme`);
  - crates/nx-language-service/src/environment_tests.rs:175;
  - crates/nx-syntax/tests/parser_tests.rs:301;
  - crates/nx-types/tests/environment_values.rs:156 and 172;
  - runtime/typescript/src/index.ts:1038 and specs/future.md:2376, which mention `provide bundle`.

  tasks.md 1.3 covers only the tree-sitter queries.
- **Recommendation:** Add editor-language-service and editor-syntax-highlighting deltas. These should cover:
  - the bundle hover on `provide env with …`;
  - `with` scoped as a keyword after an operand and as a property name in `{ with: int }` and `x.with`;
  - `with` inside markup text staying text (`<p>Pay with card</p>`).

  Add tasks for the TextMate grammar and its tests, and for migrating the listed tests and comments.
- **Fix:** Added editor-language-service (bundle hover on `provide env with`, `with` completion and hover) and editor-syntax-highlighting (`with` after an operand, `provide env`) deltas; the editor requirement moved out of with-expressions. tasks.md has a TextMate task (1.4), a migration task listing every `provide bundle` use (6.3), and the #75 conditionals are gone now that #75 merged.
- **Verification:** There are now editor-language-service and editor-syntax-highlighting deltas. The hover scenario uses `provide env with brand.theme`, and `with` is scoped as a keyword, a name and text. The `with` editor requirement moved, and tasks 1.4, 5.1 and 6.3 list the TextMate grammar and its test and every migration site I named. Two more mentions of the divergence go stale once task 6.4 removes it; they are RF20.

### ✅ Verified - RF12 The with-target rules don't settle optional left sides or derived records
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/with-expressions/spec.md:7-9, 82-86 and 100-101; openspec/specs/update-records/spec.md:47-49
  (`User.Update.Update` doesn't exist).
- **Details:** A `User.Update` is itself a record type, so it matches both the record rule (which needs
  `User.Update.Update`) and the update-record rule. The spec gives no order between them. A left side of
  type `User?` (for example `x?.author with { … }`), or a sequence of records, isn't classified either.
- **Recommendation:** Say that an update type, including `env.Update`, takes the update rule before the
  record rule. Also say that an optional or sequence left side is `with-target`, with a hint to use
  `?.`/`??` or `if`. Add a scenario for the optional case.
- **Fix:** `with-target` now also names optional and sequence left sides, with an unwrap hint and a scenario. Update types (`env.Update` included) are checked before records, so a derived update record is never treated as a record.
- **Verification:** Update types are now checked before records (with-expressions spec.md:92-93; task 2.2). Optional and sequence left sides are `with-target` with an unwrap hint, and the `<Book />.author with { … }` scenario expects `User?`. `<Book />.author` parses as a member access on an element.

### ✅ Verified - RF13 "A `T.Update` built from its entries" doesn't fit paths or overlapping entries
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/with-expressions/spec.md:8-9 compared with 40-44 and 57-60
  (`text=<Text …/> text.muted="silver"`); design.md:202 and 218-219.
- **Details:** Entries applied in order, including a whole field followed by a path into it, aren't one
  `T.Update` value. The first requirement contradicts the path requirement and the lowering in design
  decision 7.
- **Recommendation:** In the first requirement, describe a field list without paths as one `T.Update`, and
  defer to the path requirement for anything else. For example: "the value SHALL be what applying its
  entries in order gives".
- **Fix:** The first requirement now says the entries apply in order, each on the result of the one before; paths are allowed only on records and bundles (`with-path` on other update records); field lists hold plain entries only.
- **Verification:** The requirement now says the entries apply in order, and that a field list without paths means `apply(left, <T.Update …/>)` (spec.md:8-11). That is consistent with the path requirement and with design decision 7.

### ✅ Verified - RF14 The scenario for captured locals doesn't test a formula
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/environment-values/spec.md:268-271 (`<Row env.accent={item.color} />` reads no
  environment value, so the old and new rules give the same result).
- **Recommendation:** Make the value read both a local and the environment, for example
  `env.gap={env.space * item.scale}` with an inner `env.space` override. The scenario then pins capture
  semantics at the language level, as nx-ir-format's descriptor scenario does at the IR level.
- **Fix:** The captured-locals scenario is now `env.gap={env.space * item.scale}` with an inner `env.space` override, and pins that the local is captured and the environment read where read (Cells read 2 and 6).
- **Verification:** The scenario now uses `env.gap={env.space * item.scale}` with an inner `env.space=2`. It expects 2 and 6, which is correct under the new rule and would be 4 and 12 under the old one, so it does test capture together with read-time evaluation.

### ✅ Verified - RF15 One design line is far past the wrap width the other files use
- **Severity:** Low
- **Confidence:** High
- **Evidence:** design.md:164 is 149 characters, in the Risks "Cost" bullet. The rest of the change wraps at
  about 100.
- **Recommendation:** Rewrap that bullet. A few other lines run to 101-108 characters (proposal.md:71,
  specs/with-expressions/spec.md:78 and 120), mostly because of long code spans. AGENTS.md's 120-character
  target is written for NX source, not for these Markdown files, so those lines are optional to fix.
- **Fix:** Rewrapped prose at 100 characters in proposal, design and all spec deltas. The remaining lines over 100 are single long code spans in scenarios, left whole.
- **Verification:** The 149-character line is gone. The remaining lines are at most 103 characters.

## New Findings Discovered During 2026-11-24 05:05 Verification

### ✅ Verified - RF16 The encoding of a formula's fingerprint in `$env` isn't stated
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/nx-ir-format/spec.md:78-79 (`formula` holds `module`, `node`, `fingerprint`, `locals`);
  openspec/specs/nx-ir-format/spec.md:49-52 (fingerprints are 64-bit and must survive a JavaScript
  `number`); docs/nx-ir-format.md:346 and 374.
- **Details:** `$env` is canonical JSON that hosts store and compare. A 64-bit fingerprint written as a
  JSON number loses precision in the TypeScript runtime, so the two runtimes could disagree on whether a
  stored formula is stale.
- **Recommendation:** Specify the JSON form of `fingerprint` in `$env` and `$entries`, for example a decimal
  string as the existing lossless-fingerprint requirement allows, and extend task 3.2 to document it.
- **Fix:** `fingerprint` in `$env` and `$entries` is written in the lossless form program fingerprints use (nx-ir-format `$env` requirement, design decision 3, task 3.1).
- **Verification:** `fingerprint` in `$env` and `$entries` now uses the lossless program-fingerprint form. The nx-ir-format `$env` requirement (spec.md:80-81), design decision 3 and task 3.1 all say so.

### ✅ Verified - RF17 Generated formulas are closures, but nothing says how they are compared or serialized
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** specs/executable-code-generation/spec.md:11-16 (a formula is a function in `formula`,
  "meaningful only to the generated program that made it"); specs/environment-values/spec.md:263-267
  (bundles hold formula entries, so any `env.Update` value can carry one); tasks.md 6.4 (generated
  JavaScript must agree with the IR runtimes on the new programs).
- **Details:**
  - The corpus comparison (`crates/nx-api/tests/codegen/ir_corpus_tests.rs`) compares generated output
    with the IR runtime's canonical output. A descriptor or bundle holding a formula can't be equal under
    that comparison: one side holds a closure, the other `{ module, node, fingerprint, locals }`.
  - A generated program's output that holds a formula and is passed through `JSON.stringify`, for example
    stored or sent to another process, silently loses the entry. The IR runtimes, by contrast, keep it as
    data. A silent drop changes results without any diagnostic.
- **Recommendation:**
  - Say how the conformance comparison treats formula entries. For example, compare them by the value
    they give at a fixed read, or normalize them to a marker. Add that to task 6.4.
  - Say what serializing a generated formula does. Either refuse with a diagnostic, or give the closure a
    `toJSON` that throws, so it is never dropped quietly.
- **Fix:** Generated formulas also hold the runtime's `module` and `locals`; `JSON.stringify` of one throws naming the value (new scenario line); the conformance comparison matches formula entries by `value`, `path`, `module`, `locals` and the values reads give, not `node` or `fingerprint` (executable-code-generation delta, design decision 8, tasks 4.3 and 6.4). The RF5 note is added too: an entry that only names a top-level value keeps its load-time value in generated code.
- **Verification:** The executable-code-generation delta now does three things. It makes `JSON.stringify` of a generated formula throw, with a new scenario line naming `gap`. It makes the comparison match formula entries by `value`, `path`, `module`, `locals` and the values they give, leaving out `node` and `fingerprint`. And it adds the RF5 note about entries that only name a top-level value. Task 4.3, task 6.4 and design decision 8 match. One sentence in the delta now contradicts itself; that is RF21.

### ✅ Verified - RF18 The v1 IR can't be "what was emitted before" if every override gains a flag cell
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** specs/nx-ir-format/spec.md:8-11 (each override holds a reference, a path, a value node and
  a flag); specs/nx-ir-format/spec.md:57-61 (a v1-only module's IR "SHALL be what was emitted before
  this change"); docs/nx-ir-format.md:162 (`[ref, [str...], node]`).
- **Details:** If the flag is a fourth cell present on every override, a v1-only program's `envProvide`
  changes shape, which contradicts the scenario.
- **Recommendation:** State the encoding so that an unset flag is absent, for example an optional fourth
  cell present only on a formula, or the formula form as a separate shape. Then the scenario holds, and a
  v1 reader rejects only v2 modules.
- **Fix:** The formula flag is an optional fourth override cell present only on a formula, so a non-formula override's encoding is unchanged (nx-ir-format requirement, task 3.1).
- **Verification:** The flag is now an optional fourth override cell, present only on a formula (nx-ir-format spec.md:9-11; task 3.1). A non-formula override is still `[ref, [str…], node]`, so the v1 scenario holds. This applies to `envUpdate` overrides too, since they have the same shape.

### ✅ Verified - RF19 When a stale formula is refused differs between the deltas
- **Severity:** Medium
- **Confidence:** High
- **Evidence:**
  - specs/nx-ir-format/spec.md:111-114: refused when the host *enters* the stored descriptor;
  - specs/typescript-ir-runtime/spec.md:26-29: refused when the body *reads* that value;
  - specs/rust-ir-runtime/spec.md:9-14 and design.md:185-186: `enter` needs no program, so it can't check a
    fingerprint;
  - tasks.md 4.1: "validate them in `enter`".
- **Details:** These four places name three different moments (entering, reading, validating in `enter`).
  `enter` holds no program, so it can check only the shape, not the fingerprint. The result also differs
  between the moments: a stale formula for a value the body never reads either fails or doesn't.
- **Recommendation:** Pick one point and say it in every place:
  - `enter` checks the shape only, and the start of an evaluation that is given the environment checks
    every formula's module and fingerprint against its program;
  - or the check happens lazily at the read.

  Align the nx-ir-format scenario, both runtime deltas and task 4.1 with that choice.
- **Fix:** One point everywhere: `enter` checks only the shape of `$env`; a formula with a wrong module, fingerprint or node is refused with `nx-ir-boundary-type` at the read that evaluates it. nx-ir-format requirement and its scenario (entering succeeds, the read fails), both runtime deltas, design decision 3 and task 4.1 now say this.
- **Verification:** There is now one point everywhere. `enter` checks only the shape of `$env`, and a stale formula is refused with `nx-ir-boundary-type` at the read that evaluates it. nx-ir-format (spec.md:84-88, and the scenario at 115-119 where entering succeeds and the read fails), the rust-ir-runtime and typescript-ir-runtime deltas, design decision 3 and task 4.1 all agree. The TypeScript scenario ("the body reads that value") is consistent with it.

### ✅ Verified - RF20 Two notes about the `darkAccent` divergence aren't in the migration tasks
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/ir-conformance/README.md:147 and crates/nx-api/tests/codegen/ir_corpus_tests.rs:1283-1286
  (the doc comment on `KNOWN_GENERATED_DIVERGENCES`).
- **Details:** Task 6.4 removes the `darkAccent` divergence, and task 7.3 removes its note in
  `specs/future.md`, but these two places also describe it and would go stale.
- **Recommendation:** Add both to task 6.4. If the list becomes empty, remove the constant or leave an
  empty list with its comment updated.
- **Fix:** Task 6.4 now updates the `KNOWN_GENERATED_DIVERGENCES` doc comment (or removes the constant if empty) and `specs/ir-conformance/README.md`.
- **Verification:** Task 6.4 now updates the `KNOWN_GENERATED_DIVERGENCES` doc comment, or removes the constant if it ends up empty, and updates `specs/ir-conformance/README.md`.

## New Findings Discovered During 2026-11-24 05:40 Verification

### ✅ Verified - RF21 The generated-formula sentence first replaces `module` and `locals`, then keeps them
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/executable-code-generation/spec.md:12-15.
- **Details:** The sentence says a formula is a function "in place of the runtime's `module`, `node`,
  `fingerprint` and `locals`" and then that it "SHALL also hold the runtime's `module` and `locals`".
  design.md:194-195 says "beside the runtime's `module` and `locals`", which is the intended reading.
- **Recommendation:** Say "in place of the runtime's `node` and `fingerprint`", and keep the clause that
  says it also holds `module` and `locals`.
- **Fix:** The sentence now says a generated formula holds the runtime's `module` and `locals` in place of its `node` and `fingerprint`.
- **Verification:** executable-code-generation spec.md:12-14 now says the formula holds the runtime's `module` and `locals` in place of its `node` and `fingerprint`. That agrees with design decision 8 and with the comparison rule in the same paragraph.

### ✅ Verified - RF22 Design decision 5 misstates how an `env.` override through an empty optional field behaves
- **Severity:** Low
- **Confidence:** High
- **Evidence:** design.md:157-160 ("an `env.` override through an optional field applies only when the
  field has a value"); crates/nx-ir-runtime/src/environment.rs:609-619 (`set_path`).
- **Details:** When a step finds the field empty, `set_path` fails with `nx-ir-environment` ("Environment
  override of field '…' found the field empty"). It does not skip the override. The only exception is the
  last step of a plain (non-patch) override, which sets the field. Question 6's follow-up repeats the
  claim.
- **Recommendation:** Reword the reason. For example: "an `env.` override through an optional field fails
  at run time when the field is empty, while a `with` path is rejected statically, so it can't fail". Or
  drop the reason and keep only "stricter". The current main spec doesn't describe this runtime failure,
  but that predates this change and can be left as it is.
- **Fix:** Dropped the wrong reason; decision 5 now says only that `with` paths are stricter than the static rule for `env.` paths, which this change leaves as it is.
- **Verification:** design.md:157-159 now says only that the `with` rule is stricter than the static rule for `env.` paths, which the change leaves as it is; the inaccurate runtime reason is gone. The old wording now appears only in Question 6's earlier follow-up line in this report, which is a historical record and not part of the artifacts.

### ✅ Verified - RF23 Three new lines go well past the wrap width
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/nx-ir-format/spec.md:11 (151 characters), specs/environment-values/spec.md:53
  (132), and specs/rust-ir-runtime/spec.md:13 (108). Each was edited in place without rewrapping.
- **Recommendation:** Rewrap them to the ~100 characters the rest of the change uses.
- **Fix:** Rewrapped the edited paragraphs in nx-ir-format, environment-values, rust-ir-runtime and executable-code-generation.
- **Verification:** No line in design.md or the spec deltas is over 105 characters now. The rewrapped paragraphs in nx-ir-format, environment-values, rust-ir-runtime and executable-code-generation have the same wording as before, except the RF21 change.

## New Findings Discovered During 2026-11-24 06:27 Review of the "bundle or attributes, not both" change

The combined form is gone from every change file. The proposal, design (decisions 2, 3 and 4), tasks
1.2, 6.1 and 6.2, and the environment-values and nx-ir-format deltas all describe a block as holding
attribute-form overrides or one `env with` chain. No combined-form example or "the block's other
overrides" wording is left. The remaining "in the order written" sentence in "Values take precedence"
still holds for attribute-form overrides. The IR rule ("either attribute overrides or a single bundle
override, never both"), the lookup and the `$env` shape are unaffected, since a descriptor can still
receive attribute and bundle entries from different nesting levels.

The "section theme follows a mode set inside the section" scenario is still correct under the where-read
rule:

- **Main form:** `Banner`'s read of `color` reaches marketing's `color` formula. The formula reads
  `colorMode`, and the nearest source of that is `<Section env.colorMode=dark>`, so the result is `b`.
- **`with { colorMode=dark }` variant:** the formula's read of `colorMode` is a read of another value.
  It sees every source, the bundle's later `with` step included, so it also gives `b`.

"A `with` step after a bundle wins" follows from bundle entries being looked up last first.

### ✅ Verified - RF24 It isn't settled whether a field list may come right after `provide env with`
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/environment-values/spec.md:5-11. `bundle` is "an operand of the kind a `with`
  takes on its right", which is not a field list. But the next clause says "after `with`, a brace whose
  first entry is `name=` SHALL be a field list". design.md:72 ("`env with` takes the same operand").
- **Details:** `provide env with { colorMode=dark } { … }` matches the field-list sentence, because it
  comes after `with`. It does not match the operand rule. If it is accepted, it is a second spelling of
  `provide env.colorMode=dark { … }`, as a bundle. That changes which IR it emits (a bundle override,
  requiring `environment-v2`, versus a plain override at v1). If it is rejected, users need a
  diagnostic that points to the attribute form.
- **Recommendation:** Say which it is: "the first operand after `env with` SHALL NOT be a field list"
  (with a message suggesting `provide env.name=value`), or "a field list is allowed and means an inline
  bundle". Add a grammar test to task 1.2.
- **Fix:** A field list right after `env with` is an inline bundle: `provide env with { colorMode=dark } { … }` means `env with <env.Update colorMode=dark />` (environment-values requirement and a new scenario; design decision 2 notes it equals the `env.` form, which the style guide prefers for single values; task 1.2 grammar test; task 7.3 style rule).
- **Verification:** The rule is now settled. A field list right after `env with` is an inline bundle (environment-values spec.md:7-9, with the scenario "A field list after `provide env with`"), and design decision 2 and task 1.2 match. Because it is an inline bundle, it follows the bundle rules already in nx-ir-format. It emits a bundle override, which requires `environment-v2`, and `$env` holds a `bundle` entry. So "means the same as `provide env.colorMode=dark`" is about the values read, not the IR. That is consistent, and the style rule in task 7.3 steers single values to the `env.` form.

### ✅ Verified - RF25 The suggested fix for a mixed block changes the meaning when the attributes come first
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/environment-values/spec.md:10-11 ("suggests a `with { name=value }` step in place
  of the attribute-form overrides"); spec.md:46-49 (the only scenario puts the attributes after the
  bundle); tasks.md 1.2.
- **Details:** In the earlier draft of this change, a combined block applied its overrides in the order written. So in
  `provide env.radius=0 env with marketing { … }`, `marketing`'s `radius` won. The suggested rewrite,
  `env with marketing with { radius=0 }`, makes `radius=0` win instead.
- **Recommendation:** Say that the suggestion keeps the order. Attribute overrides written before the
  bundle become a leading inline bundle, such as `env with <env.Update radius=0 /> with marketing`;
  those written after become a trailing `with { … }` step. Alternatively, have the message say only
  "use one `env with` chain". Add a scenario or test for the attributes-first order.
- **Fix:** The suggestion keeps the written order: overrides after the bundle become a `with { … }` step after it, ones before it a field list before it (`provide env.radius=0 env with marketing` suggests `provide env with { radius=0 } with marketing`). New scenario; task 1.2 tests both orders.
- **Verification:** The suggestion now keeps the written order. Overrides after the bundle become a trailing `with { … }` step, and overrides before it become a leading field list (spec.md:12-15). There is a scenario for the attributes-first order (`provide env with { radius=0 } with marketing`), and task 1.2 tests both orders. A mix with overrides on both sides of the bundle follows from the same rule.

### ✅ Verified - RF26 The environment docs task doesn't mention the "not both" rule
- **Severity:** Low
- **Confidence:** High
- **Evidence:** tasks.md 7.2 ("`provide env with` for bundles, bundles in blocks, …").
- **Details:** `reference/syntax/environment.md` is where users will meet the rule, and the mixed form is
  the one they are most likely to try.
- **Recommendation:** Add the rule to task 7.2: a block holds `env.` overrides or one `env with` chain,
  and a bundle is patched further with `with { … }`. Include an example of the rewrite.
- **Fix:** Task 7.2 now covers the rule and how to rewrite a mixed block as a chain.
- **Verification:** Task 7.2 now covers the rule and how to rewrite a mixed block as a chain. Task 7.3 adds the style-guide preference for `env.x=` for single values and `env with` for bundles.

## Questions

1. A formula names "the module identity and node". Node indices are only stable within one image. Can a
   host that stores descriptors, such as a preview session reloading after an edit, or instance-tree
   reconciliation across a rebuild, still compare and evaluate them? Should a stored formula carry an
   image hash, or be refused after a rebuild?
   - Answer: a formula carries the module's fingerprint and is refused with `nx-ir-boundary-type` when it doesn't match; a host that keeps descriptors across a rebuild renders again (nx-ir-format `$env` requirement, scenario "formula from another build"; design decision 3).
   - Verification: accepted. RF16 (the fingerprint's JSON form) and RF19 (when the refusal happens) follow
     from it.
2. Should field lists accept the conditional entries that property lists allow
   (`x with { if c { a=1 } }`)? The spec says "attribute form" without saying whether that includes them.
   - Answer: no. Field lists hold plain entries only; conditional entries and spreads can come later (with-expressions first requirement; recorded as an open question in task 7.3).
3. Should the spec say outright that an element attribute `<T with=3 />` and a field-list entry
   `x with { with=3 }` keep working for a field named `with`? Today it names only `x.with`.
   - Answer: yes; the spec now says a field named `with` works as a field-list entry, an attribute and after `.`.
4. Should `provide brand.theme` get a targeted migration diagnostic ("write `provide env with brand.theme`")
   instead of a bare syntax error, given that #75 and existing docs use it?
   - Answer: yes, a syntax error that suggests `provide env with brand.theme` (RF9).
5. Should the editor requirement ("The editor completes and explains `with`") live in editor-language-service
   rather than in the new capability, as the environment editor requirements do?
   - Answer: yes, moved to editor-language-service.
6. design.md:205 says the required-field rule "already holds for `env.` paths". The main spec says only "a
   field of a record type" (openspec/specs/environment-values/spec.md:76-78). Should that requirement be
   tightened to "required", so the two rules match on paper?
   - Answer: leave the main spec as is; design decision 5 now says the rule matches `environment-path` rather than claiming it already says "required".
   - Verification: this is not quite settled. `environment_path_type` (crates/nx-types/src/infer.rs:7777-7817)
     steps through any record field, and the optional mark is kept apart from the field's type
     (`optional: bool`, crates/nx-types/src/ty.rs:332). So `env.` paths appear to allow an optional step,
     while `with` paths reject one (confidence: medium, from reading the code, not from a test). If so,
     design decision 5's "It matches" is inaccurate: either say the `with` rule is stricter, or tighten
     `environment-path` in the same change.
   - Follow-up: design decision 5 now says `with` paths are stricter than `env.` paths, which this change leaves as they are (an `env.` override through an optional field applies only when the field has a value; a `with` patch always gives one). The top-level `provide env with` wording now uses the block form's operand (RF10 note).
   - Verification (follow-up): this settles the question, since `with` paths are stricter and `env.` paths
     stay as they are. But the reason given, that an `env.` override through an optional field "applies only
     when the field has a value", doesn't match the runtime; see RF22.
7. For an override that reads the environment only on some branches (`env.x={ if c { env.y } else { 1 } }`),
   is it a formula? A static read set would say yes. Confirm that this is intended.
   - Answer: yes, intended. The flag comes from the static read set, so a value that reads the environment on any branch is a formula (design decision 3).

## Summary

All 26 findings, RF1–RF26, are ✅ Verified, and the latest fixes introduced no new issues.
`openspec validate add-with-expressions --strict` passes. The rule that a block holds `env.` overrides
or one `env with` chain is consistent across the proposal, design, tasks and specs. It now includes a
field list right after `env with` as an inline bundle, and a mixed-block suggestion that keeps the
written order.

## New Findings Discovered During 2026-11-24 08:17 Implementation Review

**Scope:** the implementation on `claude/add-with-expressions` against `origin/main` (`git diff
origin/main...HEAD`, through 75ba2ff9): grammar and validation (`crates/nx-syntax`,
`src/vscode`), lowering and checking (`crates/nx-hir`, `crates/nx-types`, `with_rewrite.rs`),
formula marking and the cycle check (`crates/nx-codegen/src/formulas.rs`,
`environment_cycles.rs`), NX IR (`crates/nx-ir`), the Rust and TypeScript runtimes, generated code
and its `runtime/environment.ts`/`.js`, the language service, the `with-expressions` and
`environment-formulas` corpus programs, tests and docs.

**How it was checked:** besides reading the code, about 20 small programs were compiled with
`nxlang codegen --target nx-ir` and `--target javascript` and run in all three engines: `nxlang run`
and a scratch host over `nx-ir-runtime` that renders component descriptors through
`Environment::enter`, the TypeScript runtime (`dist`, which matches a fresh `tsc` build), and the
generated JavaScript. Engines agreed on everything tried (section bundles, a bundle passed as a
prop, a mode set inside the section, captured locals in loops and component props, bundle path
entries with update-record patches, nested caps, `with` chains and precedence) except where RF31
says. Tests run: `cargo test -p nx-types --test with_expressions --test environment_values`,
`cargo test -p nx-language-service`, and the TypeScript `runtime.test`, `corpus.test` and
`emitted-ir.test`; all pass. The VS Code grammar tests were not run (no `node_modules` in
`src/vscode`).

### ✅ Verified - RF27 An inline bundle in a main-module `provide env with { … }` evaluates every entry at every read
- **Severity:** Medium
- **Confidence:** High (reproduced in all three engines)
- **Evidence:** `mark_formulas` (crates/nx-codegen/src/formulas.rs:119-135) visits only
  `module.declarations`, never `module.provides`. So the `envUpdate` that `provide env with
  { … }` or `provide env with { … } with { … }` builds at the top of the main module gets no formula
  flags. Its entries are plain values, computed as soon as the bundle is built, and the bundle is
  built at every read that reaches the provide. As a result, a read of one value evaluates the
  entries for every other value. Repro:
  ```nx
  environment radius: int = 2
  environment zero: int = 0
  environment space: int = 4
  provide env with { space={10 / env.zero} radius=5 }
  let root() = { env.radius }
  ```
  All three engines fail with `nx-ir-division-by-zero`. The same entries in a named bundle (`let b:
  env.Update = <env.Update …/>` then `provide env with b`) give `5`. This breaks "a read of a value
  `V` SHALL evaluate only the entries that set `V`" (environment-values) and "a read doesn't evaluate
  a bundle's entries for other values" (nx-ir-format, cost). It also breaks design decision 3's "Inline
  and named bundles behave the same". `nxlang ir explain` shows `env.space=(…)` with no `:=` in the
  `$provides` bundle.
- **Recommendation:** Also run `mark_formulas` over the values of `module.provides`, for example
  through a visitor that covers provides as `visit_declaration_expressions_mut` covers declarations.
  Add a corpus case with an inline top-level bundle whose entries read the environment, and check its
  operation count.
- **Fix:** `mark_formulas` (crates/nx-codegen/src/formulas.rs) now also walks the values of `module.provides`, so the inline bundle of a main-module `provide env with { … }` (and its `with` steps) gets formula flags. The repro now gives `5` in all engines. New corpus program `specs/ir-conformance/environment-inline-provide` pins it: `provide env with { space={10 / env.zero} radius=5 gap={env.radius * 2} }` with `radius()` = 5, `gap()` = 10 and `space()` under `env.zero=2` = 5, with operation counts.
- **Verification:** `mark_formulas` now walks `module.provides`. The RF27 repro gives `5` in Rust IR, TypeScript IR and generated JavaScript. So does a step chain, `provide env with t with { space={10 / env.zero} } with { radius=7 }`, which gives `7`. `environment-inline-provide` pins it in both IR runtimes, with operation counts. Generated JavaScript skips that program (see RF29), but the fix itself is in the shared formula marking, and the manual run agrees.

### ✅ Verified - RF28 The static `environment-cycle` check misses an inline bundle in a main-module provide
- **Severity:** Medium
- **Confidence:** High (reproduced)
- **Evidence:** `environment_cycle_diagnostics` (crates/nx-codegen/src/environment_cycles.rs:73-98)
  collects override targets from `module.provides` only where `provide.target` is set, and from
  `Provide`/`EnvUpdate` nodes inside declarations. A top-level `provide env with { a={env.b} }` has
  no target, and its `envUpdate` sits in `module.provides`, so the check never sees it:
  ```nx
  environment a: int = 1
  environment b: int = { env.a + 1 }
  provide env with { a={env.b} }
  let root() = { env.b }
  ```
  This compiles, then fails at run time in every engine with `nx-ir-environment-cycle` ('b' → 'a' →
  'b'). The same entry in a named bundle is rejected at compile time with `environment-cycle`, which
  is what the spec requires for "a bundle entry or a main-module provide of a value `V` that reads a
  `W` whose default reads `V`". The test
  `an_override_that_reads_a_value_whose_default_reads_it_is_an_environment_cycle`
  (crates/nx-api/tests/codegen/tests.rs:3854) covers only `provide env.a = …` at the top level.
- **Recommendation:** Walk the `EnvUpdate` overrides inside `module.provides` values, as is done for
  declarations, and add the inline top-level bundle form to that test.
- **Fix:** `environment_cycle_diagnostics` now also walks the `Provide`/`EnvUpdate` overrides inside bundle provides of `module.provides`. The repro is now `environment-cycle` at compile time. `an_override_that_reads_a_value_whose_default_reads_it_is_an_environment_cycle` (crates/nx-api/tests/codegen/tests.rs) gains `provide env with { a={env.b} }` and `provide env with theme with { a={env.b} }`.
- **Verification:** The repro is now `environment-cycle` at compile time, at the entry. An inline bundle with a step inside a block, `provide env with { space=1 } with { a={env.b} } { … }`, is caught too. `an_override_that_reads_a_value_whose_default_reads_it_is_an_environment_cycle` passes with the two new top-level forms.

### ✅ Verified - RF29 The corpus lacks three cases that task 6.1 lists as done, and no test reaches `nx-ir-environment-cycle`
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** Task 6.1 (tasks.md:33) lists "a mode set inside the section", "passed as a prop" and
  "the runtime cycle error" for `environment-formulas`. The program (specs/ir-conformance/
  environment-formulas/main.nx) has none of them:
  - `darkAccent` sets the mode under the main-module theme, not inside a section bundle.
  - `passed` passes a bundle as a function argument, not as a component prop.
  - No entrypoint fails with `nx-ir-environment-cycle`.

  A search of every Rust, TypeScript and JavaScript test finds no test of `nx-ir-environment-cycle`
  in any engine. The error paths in crates/nx-ir-runtime/src/environment.rs:752,
  runtime/typescript/src/index.ts:5061 and crates/nx-codegen/src/runtime/environment.js:294 are
  never run. Task 6.4's "a theme that varies by mode in a section" isn't in the
  generated-JavaScript comparison either. The spec's cost scenario "A formula is charged at each
  read", where a component reads `env.gap` twice through a descriptor formula, has no case: `card()`
  only returns the descriptor. RF28's repro is a ready-made runtime cycle case, and it gives the
  same message in all three engines today.
- **Recommendation:** Add the missing cases to `environment-formulas`, with a `diagnostics.json`
  entry for the cycle case. For the section-mode and prop cases, use a component that applies `provide env
  with theme` to a body that sets `env.colorMode=dark` and reads a mode-dependent entry. Add a
  lifecycle, or a case that renders `Card`, so the per-read charge of a descriptor formula is
  recorded. Otherwise, uncheck the items.
- **Fix:** New corpus program `environment-inline-provide`: a component `Section` takes a bundle prop (`look: env.Update = {theme}`), applies it with `provide env with look`, and reads the mode-dependent accent twice, the second time in a section set to dark mode (lifecycle result `blue`, `navy`, with its operation count, so a formula's per-read charge is recorded); and a failing case `cycle#throughB`, which passes `readB` as a function value so the static check can't see the loop, recorded in `diagnostics.json` as `nx-ir-environment-cycle`. Rust IR, TypeScript IR and generated JavaScript run it (334 parity tests and the TS suite pass). The README describes the program.
- **Verification:** Only partly fixed. Both IR runtimes run the new program and record all of its cases: `corpus_results_match_the_rust_ir_runtime`, the operation-count test and the TypeScript `corpus.test` pass, including `cycle#throughB` and the `Section` lifecycle. But the **Fix** note is wrong that generated JavaScript runs it. `cycle` calls a function-typed value (`<read />`), so `nxlang codegen --target javascript` refuses the whole program ("calls of function-typed values are not supported by executable source codegen yet"). `generated_javascript_agrees_with_the_recorded_results` then lists `environment-inline-provide` as skipped. So the inline top-level bundle (RF27), the bundle prop and the dark section are still not compared in generated JavaScript, which is what task 6.4 asks for. Also, the lifecycle mounts `Section` with no props, so `look` is its default `{theme}`: no caller passes a bundle as a prop. **Still to do:** move `cycle`/`readB` to their own program, or to another one that generated code already skips, so this program runs in generated JavaScript. Add a component that renders `<Section look={…} />` with a bundle other than the default.
- **Fix (2):** `cycle`/`readB` moved to their own program, `specs/ir-conformance/environment-cycle` (with a passing `plain()` entrypoint beside `cycle#throughB`, recorded as `nx-ir-environment-cycle`), so `environment-inline-provide` no longer has a call of a function-typed value and generated JavaScript now generates and runs it (`nxlang codegen --target javascript` accepts it; 335 parity tests pass). `environment-inline-provide` gains a component `Branded look: env.Update` with a required bundle prop, mounted by a lifecycle whose `props` pass a bundle (`accent` = `green`), so a caller passes a bundle other than the default. The README describes both programs and why the cycle case is apart.
- **Verification:** (2) `environment-inline-provide` no longer calls a function-typed value, and `nxlang codegen --target javascript` accepts it. `generated_javascript_agrees_with_the_recorded_results` passes, and its skipped list now names `environment-cycle` but not `environment-inline-provide`, so `radius`, `gap` and `space` (RF27's inline top-level bundle) are compared in generated JavaScript. `environment-cycle` records `cycle#throughB` as `nx-ir-environment-cycle` beside a passing `plain()`. `Branded` is mounted with a host-passed bundle (`green`). The corpus tests and the TypeScript `corpus.test` pass. The generated-JavaScript comparison runs entrypoints only, not lifecycles, so `Section` and `Branded` are still checked only in the IR runtimes; see RF37. A manual render in TypeScript IR and generated JavaScript of `<Section />`, `<Section look={green} />`, `<Branded look={green} />` and `<Branded look={theme with dark} />` agreed (`blue`/`navy`, `green`/`green`, `green`, `navy`).

### ✅ Verified - RF30 `specs/future.md` drops "Generated values are evaluated at load", which is still true
- **Severity:** Medium
- **Confidence:** High (reproduced)
- **Evidence:** The change removes the entry at specs/future.md:~2376. Generated modules still
  evaluate top-level values once at load, and RF31 shows the result.
  specs/ir-conformance/README.md:152 now says no entrypoint is skipped. That holds only because no
  corpus program reads a top-level value that reads the environment under an override.
- **Recommendation:** Restore the limitation, narrowed: a top-level value that reads the
  environment, whether read directly or named by an override or bundle entry, keeps its load-time
  value in generated code. Link it from the README sentence. The fix itself could be a formula-like
  thunk for such values, which is later work.
- **Fix:** Restored "Generated values are evaluated at load" in specs/future.md, narrowed to top-level values that read the environment, including when an override or bundle entry names one, with RF31's example. specs/ir-conformance/README.md now says that limitation remains and that no program names such a value under an override.
- **Verification:** The restored specs/future.md entry is narrowed as recommended and includes RF31's example. The README sentence links it. Nit: the edited README line ("No other entrypoint is skipped, though …") runs to about 125 characters, past the file's wrap width.

### ✅ Verified - RF31 Generated JavaScript gives a different result when an override names a top-level value that reads the environment
- **Severity:** Low (the executable-code-generation delta allows it: "an entry that only names a
  top-level value gets that value as it was at load")
- **Confidence:** High (reproduced)
- **Evidence:**
  ```nx
  environment space: int = 4
  environment gap: int = 0
  let tv: int = { env.space * 3 }
  let b() = { provide env.gap={tv} { provide env.space=1 { env.gap } } }
  ```
  Rust IR and TypeScript IR give `3` and generated JavaScript gives `12`. `env.gap:=tv` is flagged
  as a formula, so the IR runtimes evaluate `tv` again at the read, while the generated closure
  returns the module-load value. The spec allows the divergence, but nothing records it (RF30), and
  the corpus doesn't pin it as a known divergence.
- **Recommendation:** Record it as in RF30, and add the case to `KNOWN_GENERATED_DIVERGENCES` with a
  corpus entrypoint. Or make generated top-level values that read the environment lazy thunks, so
  all three engines agree.
- **Fix:** Recorded, as the spec allows: the limitation and its example are in specs/future.md ("Generated values are evaluated at load") and the README points to it. Making generated top-level values that read the environment lazy is left as later work and raised with Bret; not adding a known-divergence entrypoint, so the comparison stays free of a skip list.
- **Verification:** Rerun: still `3` in the IR runtimes and `12` in generated JavaScript, as recorded. The executable-code-generation delta allows this ("an entry that only names a top-level value gets that value as it was at load"). Recording it in specs/future.md, with the fix as later work, is one of the two options recommended, so it is acceptable for this change. Without a known-divergence entrypoint, nothing would flag the day the engines start to agree, or the day the gap widens. That is a reasonable trade for keeping the comparison free of a skip list.

### ✅ Verified - RF32 The `with-target` hint for optional and sequence left sides suggests syntax that doesn't exist
- **Severity:** Low
- **Confidence:** High
- **Evidence:** crates/nx-types/src/infer/with.rs:121-122 suggests ``test it with `if x is { some }` ``.
  NX has no `some` pattern; the docs use `if x? { … }` (expressions.md:365, if.md:245). The same hint
  is given for a sequence (`User+`), where `(x ?? fallback) with { … }` doesn't help: `??` doesn't
  turn a sequence into one record.
- **Recommendation:** For `T?`, suggest ``if x? { x with { … } }`` or ``(x ?? fallback) with
  { … }``. For a sequence, suggest patching each item: ``for item in x { item with { … } }``.
- **Fix:** The hint now depends on the left side: for `T?` it suggests `if x? { x with { … } }` or `(x ?? fallback) with { … }`; for a sequence, `for item in x { item with { … } }`, with no `??`. Both forms compile. Tests in crates/nx-types/tests/with_expressions.rs check each.
- **Verification:** `u.home with { … }` on `A?` now suggests `if x? { x with { … } }` or `(x ?? fallback) with { … }`. `u.tags with { … }` on `A+` suggests `for item in x { item with { … } }`, with no `??`. Both suggested forms compile and run. `with_on_an_optional_suggests_unwrapping_it` and `with_on_a_sequence_is_with_target` pass. ("but this is a A?" keeps the older article wording; not part of this finding.)

### ✅ Verified - RF33 `provide brand with { … }` at the top of a module gets a suggestion that drops the bundle
- **Severity:** Low
- **Confidence:** High (reproduced)
- **Evidence:** For `provide marketing with { radius=1 }`, `lower_provide_bundle`
  (crates/nx-hir/src/lower.rs:3589-3598) reports "`environment-path`: … write `env` in place of
  'marketing'". Following that advice gives `provide env with { radius=1 }`, which loses
  `marketing`. The spec asks for a syntax error that suggests `provide env with bundle`, which here
  is `provide env with marketing with { radius=1 }`. The code `environment-path` is also the wrong
  one for this. A plain `provide marketing` gets the right syntax error and suggestion from
  validation.
- **Recommendation:** Report it as the same syntax error that `provide brand.theme` gets, and suggest
  `provide env with <name> with …`, keeping the written operand and steps.
- **Fix:** `provide brand with { … }` (top level or in a block) is now the syntax error `provide brand.theme` gets, from validation (`PROVIDE_BUNDLE` whose head is not `env`), suggesting `provide env with brand with { … }` with the written operand and steps. The `environment-path` diagnostic in `lower_provide_bundle` is removed. Test `test_provide_bundle_with_steps_but_no_env_suggests_env_with` (crates/nx-syntax/tests/parser_tests.rs).
- **Verification:** `provide marketing with { radius=1 }` is now a syntax error that suggests `provide env with marketing with { radius=1 }`. This holds at the top level, with a second step (`… with marketing`), and in a block. The `environment-path` text is gone from lower.rs. `parser_tests` passes (207). A call operand still gets a generic error; see RF36.

### ✅ Verified - RF34 The TextMate rule scopes `with` as a keyword in a type's field list after another field
- **Severity:** Low
- **Confidence:** Medium (from reading the regex; the grammar tests were not run here)
- **Evidence:** The rule at src/vscode/syntaxes/nx.tmLanguage.json:403 matches
  `(?<=[A-Za-z0-9_)\]}"'>])\s+\K\bwith\b(?!\s*=)`. `record-body` and `record-property` include
  `#operators`. In `type Opts = { a:int with:int }`, `with` follows `int` and a space and is
  followed by `:`, so it matches as `keyword.operator.with.nx`. The match starts at the space,
  before any property-name match at `with`. The same applies to `component <C a:int with:int />`.
  with.test.ts covers only `{ with: int }` as the first field. The editor-syntax-highlighting spec
  says `with` as a property name keeps the name scope.
- **Recommendation:** Add `:` to the negative lookahead (`(?!\s*[=:])`), and add a grammar test with
  `with` as a second field.
- **Fix:** The rule's lookahead is now `(?!\s*[=:?])`, so `with:` and `with?:` stay names. New grammar test in src/vscode/test/grammar/with.test.ts covers a later type field, an optional one and a component prop; it fails with the old rule and passes now (314 grammar tests pass).
- **Verification:** The lookahead is now `(?!\s*[=:?])`. No valid operator `with` is followed by `:` or `?`, so the change has no false negatives. The grammar suite passes: 314 tests, including the new later-field, optional-field and component-prop cases.

### ✅ Verified - RF35 No test checks that a `with` program emits the same image as its `apply`/`merge` form
- **Severity:** Low
- **Confidence:** High
- **Evidence:** Task 3.2 says "Check that a program written with `with` emits the same images as its
  `apply`/`merge` form", and design decision 7 promises it. No test compares images. The corpus pair
  `moved`/`applied` compares only results. A manual check here, with field lists, a path through a
  name, and update-record `merge`, gave explain output identical except for spans and the
  fingerprint. So the behavior holds today but isn't pinned.
- **Recommendation:** Add an emitter test that compiles both forms and compares the stripped images'
  node and declaration tables, ignoring spans and fingerprint.
- **Fix:** New test `a_with_emits_the_image_of_its_apply_or_merge_form` (crates/nx-api/tests/codegen/tests.rs) compiles a field list, an update operand and an update-record field list beside their `apply`/`merge` forms and compares the explained stripped images, without the fingerprint line.
- **Verification:** `a_with_emits_the_image_of_its_apply_or_merge_form` passes. It compares stripped images, which carry no spans, so only the fingerprint line is dropped. It covers a field list, an update operand and an update-record field list. The path form (`address.city=…`) isn't among the cases, but my earlier manual check found it identical to its nested `apply`.

## Questions (2026-11-24 08:17 review)

- Should the generated-code divergence in RF31 be fixed in this change, by making top-level values
  that read the environment lazy, or only recorded? The spec allows it, but it is the same kind of
  divergence the change set out to remove for `darkAccent`.

## Summary (2026-11-24 08:17 review)

The implementation matches the spec closely, and the three engines agree on every formula, bundle
and `with` case tried, including several scenarios the corpus doesn't hold. The real defects come
from one gap: the inline bundle of a main-module `provide env with { … }` is skipped by formula
marking (RF27) and by the static cycle check (RF28). Beyond that, the corpus is missing cases that
task 6.1 claims (RF29), and `specs/future.md` drops a limitation that still applies (RF30, RF31).
The rest are Low: diagnostic wording, a TextMate edge case and a missing image-equivalence test.

## New Findings Discovered During 2026-11-24 09:05 Verification

### ✅ Verified - RF36 A top-level `provide` of a call without `env with` gets a generic syntax error
- **Severity:** Low
- **Confidence:** High (reproduced)
- **Evidence:** With `let m(): env.Update = { marketing }`, both `provide m()` and `provide m() with
  { radius=1 }` at the top of the main module give only "Syntax error / Check the syntax and try
  again". A name or element operand (`provide marketing`, `provide <env.Update … />`, `provide
  marketing with { … }`) gets the suggestion for `provide env with …`. The spec says "A top-level
  `provide bundle` without `env with` SHALL be a syntax error whose message suggests `provide env
  with bundle`". The docs list a call among the forms a bundle may take. Inside a block,
  `provide marketing { … }` and `provide m() with { … } { … }` give "Invalid function definition".
  The spec doesn't require that form, but it is the one a user moving a bundle into a section is
  likely to write.
- **Recommendation:** Have the validation that handles `PROVIDE_BUNDLE` also recognize a call (and
  a braced expression) after `provide`, top level and in a block. Suggest `provide env with
  <operand> …`, keeping the steps. Add parser tests for the call form.
- **Fix:** The grammar's `legacy_bundle` now also admits a call operand (`alias($.bundle_call_expression, $.call_expression)`; parser regenerated), so top-level `provide m()` gets the `provide env with m()` suggestion; for `provide m() with { … }` validation carries the `with` steps left in the error node after the definition, suggesting `provide env with m() with { radius=1 }`. A braced expression was already admitted. Parser tests added to `test_provide_bundle_with_steps_but_no_env_suggests_env_with`.
- **Status (block forms):** Not changed: inside a block, `provide marketing { … }` and `provide m() with { … } { … }` still give "Invalid function definition". `provide name {` is ambiguous with the start of other forms in the block grammar, and the spec doesn't require it; a dedicated recovery rule could be added later if it proves common.
- **Verification:** (2) `provide m()` at the top level is fixed: it suggests `provide env with m()`, also when declarations follow. The step form is not. In `provide m() with { radius=1 }` followed by `let root() = { env.radius }`, the error node that validation copies runs to the end of the file. So the suggestion is "write `provide env with m() with { radius=1 }\nlet root() = { env.radius }`", and it takes in every declaration after the provide; with `provide k(3) with marketing` and three lines after it, it quotes all three. A generic "Syntax error" at the `with` is still reported before it, and the declarations swallowed by the recovery are lost to the module. The new parser test passes only because the provide is the last line of its source (parser_tests.rs:409). A name operand with steps (`provide marketing with { … }`) is fine. **Still to do:** let `legacy_bundle` take `with` steps after a call operand in the grammar, as it already does after a name, instead of copying them from the error node. Then drop the extra "Syntax error". Add a parser test with a declaration after the provide. The block forms are left as recorded under Status, which is acceptable since the spec doesn't require them.
- **Fix (2):** The grammar now gives a call operand its own `provide_definition` branch, `legacy_bundle` (the call) then `with` steps as `legacy_step` fields, so `provide m() with { radius=1 }` parses as one definition with no error node; the suggestion is the definition's own text after `provide` (`provide env with m() with { radius=1 }`), and no generic "Syntax error" is reported. The parser test now has a declaration after the provide and checks the suggestion ends at the steps and that no "Syntax error" appears.
- **Verification:** (3) The grammar now parses `provide <call> with …` as `legacy_bundle` plus `legacy_step`s, and validation suggests the provide's own text after `provide`. I checked `provide m() with { radius=1 }`, `provide k(3) with marketing` (with a comment and two declarations after it) and `provide k(3) with { radius=1 } with marketing`, plus a trailing `// comment`. Each now gives one diagnostic with exactly the written chain, and the following declarations are untouched. The name form and the valid `provide env with k(1) with { … }` / `provide env.radius = 7` are unchanged. All `nx-syntax` tests pass, including the parser test that now has a declaration after the provide.

## New Findings Discovered During 2026-11-24 09:40 Verification

### ✅ Verified - RF37 The bundle-prop and dark-section cases are not compared in generated JavaScript
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `generated_javascript_agrees_with_the_recorded_results`
  (crates/nx-api/tests/codegen/ir_corpus_tests.rs:1356) runs a program's entrypoints and never its
  lifecycles. In `environment-inline-provide`, the bundle passed as a prop, the section set to dark
  mode and the host-passed `Branded` bundle are all lifecycles, so only the IR runtimes check them.
  Task 6.4 asks for "a theme that varies by mode in a section" in the generated-JavaScript
  comparison. A manual render agrees today (see RF29's verification), but nothing pins it.
- **Recommendation:** Add plain entrypoints that cover the same paths, such as `let darkSection() =
  { provide env with theme { provide env.colorMode=dark { env.accent } } }` and a function that
  takes an `env.Update` argument and applies it. Alternatively, extend the generated-JavaScript
  comparison to lifecycles' initial render.
- **Fix:** Added plain entrypoints to `environment-inline-provide` that generated JavaScript runs: `sectioned()` applies the mode-dependent theme with `provide env with theme` and reads it in a section set to dark mode (`navy`), and `shown()` returns `<Branded look={theme} />`, a descriptor whose bundle prop carries a formula entry, compared by module and locals. 335 parity tests (including `generated_javascript_agrees_with_the_recorded_results`) and the TypeScript suite pass.
- **Verification:** `sectioned()` (`navy`: a theme that varies by mode, under a mode set inside the section) and `shown()` (a `Branded` descriptor carrying `theme` as its prop) are now entrypoints. `generated_javascript_agrees_with_the_recorded_results` passes with 167 entrypoints compared, 2 more than before, and `environment-inline-provide` isn't skipped. The corpus tests and the TypeScript `corpus.test` pass. `shown()` compares the descriptor, with the bundle's formula entry, rather than a rendered read of the prop. That covers passing a bundle as a prop, and my manual render already showed the engines agree on applying it.
