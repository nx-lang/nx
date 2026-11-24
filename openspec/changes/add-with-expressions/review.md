# Review: add-with-expressions (proposal)

Reviewed: proposal.md, design.md, tasks.md, specs/with-expressions, and the deltas to environment-values and
nx-ir-format, against the main specs (environment-values, nx-ir-format, update-records,
component-instance-tree, executable-code-generation, rust-ir-runtime, typescript-ir-runtime,
editor-language-service, editor-syntax-highlighting), `crates/nx-syntax/grammar.js`,
`docs/nx-ir-format.md` and the current implementation. `openspec validate add-with-expressions --strict`
passes. Every requirement has SHALL on its first line. The MODIFIED blocks repeat the full main requirement,
all of its original scenarios included, and the REMOVED header matches the main spec's name exactly.

## Findings

### 🟡 Fixed - RF1 Precedence 125 is already taken by `??`, which is right-associative
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

### 🟡 Fixed - RF2 Formulas bring back reads that have no source left, and the cycle rules don't cover them
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

### 🟡 Fixed - RF3 A bundle held in a local, prop or loop item is still evaluated where it was built
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

### 🟡 Fixed - RF4 The NX IR encoding of the new forms is missing from the format spec
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

### 🟡 Fixed - RF5 Generated code can't produce the `$env` formulas that the runtimes produce
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

### 🟡 Fixed - RF6 The runtime specs and the host `enter` API aren't updated
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

### 🟡 Fixed - RF7 Read tracking in the instance tree doesn't cover formulas and bundles
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

### 🟡 Fixed - RF8 What evaluating a bundle for one value means is underspecified, and so is its cost
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

### 🟡 Fixed - RF9 `env with marketing` outside a `provide` would parse, and the existing error isn't a syntax error
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

### 🟡 Fixed - RF10 The bundle after `provide env with` is "any expression", but the grammar can't take one there
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

### 🟡 Fixed - RF11 Editor, highlighting and migration specs and tasks are missing
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

### 🟡 Fixed - RF12 The with-target rules don't settle optional left sides or derived records
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

### 🟡 Fixed - RF13 "A `T.Update` built from its entries" doesn't fit paths or overlapping entries
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

### 🟡 Fixed - RF14 The scenario for captured locals doesn't test a formula
- **Severity:** Low
- **Confidence:** High
- **Evidence:** specs/environment-values/spec.md:268-271 (`<Row env.accent={item.color} />` reads no
  environment value, so the old and new rules give the same result).
- **Recommendation:** Make the value read both a local and the environment, for example
  `env.gap={env.space * item.scale}` with an inner `env.space` override. The scenario then pins capture
  semantics at the language level, as nx-ir-format's descriptor scenario does at the IR level.
- **Fix:** The captured-locals scenario is now `env.gap={env.space * item.scale}` with an inner `env.space` override, and pins that the local is captured and the environment read where read (Cells read 2 and 6).

### 🟡 Fixed - RF15 One design line is far past the wrap width the other files use
- **Severity:** Low
- **Confidence:** High
- **Evidence:** design.md:164 is 149 characters, in the Risks "Cost" bullet. The rest of the change wraps at
  about 100.
- **Recommendation:** Rewrap that bullet. A few other lines run to 101-108 characters (proposal.md:71,
  specs/with-expressions/spec.md:78 and 120), mostly because of long code spans. AGENTS.md's 120-character
  target is written for NX source, not for these Markdown files, so those lines are optional to fix.
- **Fix:** Rewrapped prose at 100 characters in proposal, design and all spec deltas. The remaining lines over 100 are single long code spans in scenarios, left whole.

## Questions

1. A formula names "the module identity and node". Node indices are only stable within one image. Can a
   host that stores descriptors, such as a preview session reloading after an edit, or instance-tree
   reconciliation across a rebuild, still compare and evaluate them? Should a stored formula carry an
   image hash, or be refused after a rebuild?
   - Answer: a formula carries the module's fingerprint and is refused with `nx-ir-boundary-type` when it doesn't match; a host that keeps descriptors across a rebuild renders again (nx-ir-format `$env` requirement, scenario "formula from another build"; design decision 3).
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
7. For an override that reads the environment only on some branches (`env.x={ if c { env.y } else { 1 } }`),
   is it a formula? A static read set would say yes. Confirm that this is intended.
   - Answer: yes, intended. The flag comes from the static read set, so a value that reads the environment on any branch is a formula (design decision 3).

## Summary

The proposal is coherent in intent, and the three artifacts agree with each other on the user-facing
syntax. It validates under strict mode, and its MODIFIED and REMOVED blocks are well formed. The weaknesses
are in the new evaluation rule and the layers below it:

- `with` collides with `??` at precedence 125 (RF1).
- The "computed where read" rule creates reads with no remaining source, which the cycle rules don't cover
  (RF2).
- The rule doesn't hold for a bundle passed as a value (RF3).
- Five capabilities need deltas that the change doesn't have:
  - the IR encoding of bundles in blocks, `base`, the formula flag and `environment-v2` (RF4);
  - generated-code `$env` (RF5);
  - the runtime and `enter` APIs (RF6);
  - instance-tree read tracking (RF7);
  - editor and highlighting (RF11).

Count: 6 High, 5 Medium, 4 Low, plus 7 questions.
