# Review: add-tracked-reactivity

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/reactivity/spec.md,
specs/nx-ir-format/spec.md, specs/component-instance-tree/spec.md,
specs/executable-code-generation/spec.md  
**Reviewed code:** crates/nx-ir-runtime/src/{environment.rs, eval.rs, component.rs, tree.rs,
stored.rs, value.rs}, runtime/typescript/src/index.ts (environment, instance, dispatch),
crates/nx-codegen/src/runtime/environment.js, docs/nx-ir-format.md, openspec/specs/{nx-ir-format,
component-instance-tree, component-action-handlers, environment-values, rust-ir-runtime,
typescript-ir-runtime, value-origin}/spec.md, specs/future.md (instance tree, reactive model),
specs/ir-conformance/README.md  

The change is spec-only, so it is reviewed against the code it will change. Design line numbers
are those of `design.md` itself.

## Findings

### ✅ Verified - RF1 A kept result's validity never checks that a read it took from the root is still taken from the root at the new use, so a light palette can be reused in a dark section
- **Severity:** High
- **Confidence:** High
- **Evidence:** design.md:206-226 and specs/component-instance-tree/spec.md:27-31 classify a kept
  result's inputs once, when it is evaluated, into root reads (checked by revision) and
  override-supplied reads (checked by value). Which class a read is in depends on where the result
  is used. Each node's environment is its own stack of `$env` layers (`Node.environment`,
  tree.rs:66-69, `Environment::enter`). So `color`'s default, kept after a node with no override
  read `colorMode` from the root, records `colorMode` as a root read. A node under
  `<div env.colorMode=dark>` that later asks for it finds the root revision unchanged and the
  result "valid" (spec:28-31), with no override-supplied read to compare, and reuses the light
  palette. The scenario "Light and dark sections keep a variant each" (spec:50-53) covers only the
  case where both sections override.
- **Recommendation:** Make variant selection and validity check every recorded read's *current
  supplier* at the use: a read recorded from the root is valid only if no layer at the use
  supplies or caps that value, and an override-supplied read only if the same kind of override
  supplies it. In practice that means a variant key built from what supplies each read at the use,
  not only from the override values, computed by the same stack walk that gives tokens. Add a
  scenario: a result kept from an unoverridden node and then used under `env.colorMode=dark` is
  evaluated again.
- **Fix:** Each read in a record is classified at the use by whether a layer there sets or caps the value (free, from the walk that gives tokens), and a read classified otherwise than recorded does not hold, at any depth, since an edge holds only when its variant is valid at the same use. Variants are told apart by those classes and the override-supplied values. design.md Decisions 8 and 11; reactivity model requirement; component-instance-tree "The tree keeps derived results across renders", with the scenario "A result kept where nothing overrides is not used under an override". Refinement: a use tries variants in the order kept, each by the in-order rule of RF3, rather than computing a key up front, so that selecting a variant never looks up a value a fresh run would not read.
- **Verification:** Verified. Decision 11 "Validity at a use" and the kept-results requirement classify each read at the use, and a read classified otherwise than recorded does not hold. The new scenario "A result kept where nothing overrides is not used under an override" pins the light/dark case. Variants are tried in the order kept, each by the in-order rule, which is deterministic.

### ✅ Verified - RF2 The node's record is flattened leaf reads in Decision 8 but direct inputs in Decisions 11–12, and the flattened record defeats the equality cutoff
- **Severity:** High
- **Confidence:** High
- **Evidence:** Decision 8 (design.md:158-164) and the requirement "The instance tree records the
  reads of a reused result" (specs/reactivity/spec.md:83-86) keep the node's read list as every
  leaf read, `locale` under `direction` included, checked by value through
  `environment_reads_hold` (environment.rs `environment_reads_hold`, used by `visit` at
  tree.rs:560-564 and by `change_environment` at tree.rs:248-253). Decision 11/12
  (design.md:206-208, 240-243) and the MODIFIED root-value requirement
  (component-instance-tree/spec.md:107-111) give the node direct inputs (root values, kept-result
  edges) with revisions. With a flattened list, the node is a `locale` reader in `readers`, and on
  a `locale` change `environment_reads_hold` finds `locale` changed and renders it. That
  contradicts "An equal result stops a change from spreading" (reactivity/spec.md:120-124) and
  "A node whose inputs come back equal is left alone" (component-instance-tree/spec.md:72-75). The
  design also never gives a node its own `verified` revision, which "renders only when one of its
  direct inputs changed" needs, and never says which record `visit`'s skip check uses. The
  scenario at reactivity/spec.md:120 also sits under the dispatch requirement after a stray blank
  line, so it is attached to the wrong requirement.
- **Recommendation:** Choose one node record. Direct inputs plus a per-node `verified` revision is
  the one that allows the cutoff. Rewrite Decision 8, the reactivity "records the reads of a
  reused result" requirement and the MODIFIED root-value requirement to match, state how `visit`'s
  skip check validates the new record, and move the cutoff scenario under the kept-results
  requirement.
- **Fix:** One record form for nodes and kept results: direct inputs in read order (root reads with the root's revision and an edge when a provide or default supplied the value, kept results used as edges, override-supplied reads with values; reads under the render's own overrides keep today's by-value handling), plus props, state fields and a per-node `verified` revision. `set_environment`, the visit skip check and the dispatch all check it by the kept-results rule, with revisions and equality cutoff. Rewrote Decision 8, replaced the reactivity requirement with "The instance tree's records hold direct inputs" (the cutoff scenario now sits there), and rewrote the MODIFIED root-value requirement.
- **Verification:** Verified. Decision 8 and the reactivity requirement "The instance tree's records hold direct inputs" give nodes and kept results one record form (direct inputs in read order, edges in place of inner reads) plus a per-node `verified` revision, and task 5.1 replaces the leaf-read `environment_reads_hold` check in `visit`, `set_environment` and dispatch. The cutoff scenario now sits under that requirement. The rule for edges still needs a fix for the cutoff to work; that is RF17, a new finding.

### ✅ Verified - RF3 Recursive validation has no defined order and no early stop, so it can evaluate an input the new run would not read, adding operations or raising an error a fresh evaluation would not
- **Severity:** High
- **Confidence:** Medium
- **Evidence:** design.md:209-213 and component-instance-tree/spec.md:29-31 say only that "the
  used results [are] validated first, recursively". Suppose `K` reads `flag` and then, only when
  `flag` is true, `K3`, and `flag` changes to false. If validation brings every recorded input up
  to date before evaluating `K` again, it evaluates `K3`. That costs operations a fresh render
  would not, and can raise `K3`'s error (a cycle, a failed check, a budget charge) where a fresh
  evaluation succeeds. That breaks "No result changes" (design.md:35). Salsa's deep verify and
  Adapton avoid this by checking inputs in the order they were read and stopping at the first one
  that changed, then running the query again. The order also decides the operation count, which
  must be a function of the tree's state.
- **Recommendation:** Specify the order: inputs are checked in the order they were first read,
  each one validated (recursively) before the next, and at the first that changed the result is
  evaluated again without checking the rest. Add a scenario where an input that only a branch no
  longer taken read fails when evaluated, and the render still succeeds.
- **Fix:** Inputs are checked in the order they were read, each validated before the next, stopping at the first that does not hold; the result is then evaluated again without checking the rest (Salsa's rule). design.md Decision 11 states why this never evaluates a branch or raises an error a fresh run would not, and that the order fixes the count. Scenario "An input only a branch no longer taken read is not checked".
- **Verification:** Verified. Inputs are checked in read order with an early stop, followed by evaluation again (Decision 11, kept-results requirement, scenario "An input only a branch no longer taken read is not checked"). The wording of the edge rule introduced RF17.

### ✅ Verified - RF4 Ignoring the caps pushed for an evaluation is unsound for a bundle provide, which is cached by index but evaluated under a cap of whichever value is being read
- **Severity:** High
- **Confidence:** Medium
- **Evidence:** design.md:88 caches "an entry-module provide's value, a bundle provide's bundle
  included" by provide index. design.md:112-116 says the token ignores "the layers and caps `S`'s
  own evaluation pushed", and that ignoring caps "can only make a token change more often". In the
  code, a bundle provide (`target: None`) is evaluated inside `self.capped(key, Below::Provide(index), …)`,
  with `key` the value being read (environment.rs `provided_or_default`, the `None =>` arm at
  :1091 of the file). So one bundle expression is evaluated under `Cap{gap}` for a read of `gap`
  and under `Cap{scale}` for a read of `scale`. If the bundle expression reads `env.scale` (for
  example `provide env with makeTheme(env.scale)`, which also sets `scale`), the two evaluations
  give different bundles. The cached one has `scale` recorded with token *none*, because the cap
  is ignored, so a later read of `scale` reuses the bundle from the `gap` read. That is a wrong
  value. Treating the cap as a token does not fix it either: each read pushes a fresh cap with a
  fresh sequence number, so nothing that reads the capped value would ever be reused. Nor does the
  design say whether `S` "begins" before or after its own cap is pushed. Both runtimes must agree
  on that, or their counts differ.
- **Recommendation:** Identify a cap by what it means, `(key, below)`, not by a sequence number.
  Include in a provide's cache key the cap it was evaluated under (a bundle provide is keyed by
  `(index, key read)`), or do not cache bundle provides across keys. Pin where `S` begins
  (before its cap is pushed) in both runtimes, and correct the "only more often" argument. Add a
  corpus case with a bundle provide that reads a value it sets, read for two different keys.
- **Fix:** A bundle entry-module provide is keyed by (provide index, value being read), in design.md Decision 2, the reactivity within-call requirement and generated code. Decision 3 now defines where an evaluation begins (the stack when the result is asked for, before its own cap), takes tokens over the layers below that, explains that the caps above it are fixed by the result's identity and that caps below are ordinary layers, drops the "only more often" argument, and walks through `provide env with makeTheme(env.scale)` read for `gap` and `scale`. Added a reactivity scenario, an nx-ir-format corpus item and tasks 1.2/1.4/6.1.
- **Verification:** Verified. A bundle provide is keyed by (index, value read) in Decision 2, the within-call requirement and generated code. Where an evaluation begins (before its own cap) is pinned, and caps below the start are ordinary numbered layers. I traced `makeTheme(env.scale)` through `provided_or_default`: the `scale` entry starts above `Cap{gap}`, records `scale` with token none and gives the default-built bundle, and the `gap` entry gives the bundle built from the provided `scale`. Each matches a fresh evaluation. The "only more often" argument is gone.

### ✅ Verified - RF5 Reads made by capturing a frame are not reads a body makes at run time, so formulas, prop defaults and handlers can depend on a prop or state field the record misses
- **Severity:** High
- **Confidence:** High
- **Evidence:** Props and state are frame slots (component.rs: the dispatch frame is built from
  `instance.props` then `working`, :1340-1356). A render records the props and state fields it
  read (design.md:166-170, 183-186), which in practice means `slot` nodes evaluated in the body.
  Three other things read slots:
  - `capture_formula` reads `frame.get(slot)` directly for the slots of a formula's node
    (environment.rs:857-870), without evaluating those nodes. With
    `<Row env.gap={env.space * scale}>`, a change of the prop or state field `scale` is not
    recorded, so the prop skip or the dispatch skip keeps a `$env` formula that captured the old
    `scale`.
  - A prop's default can read another prop: `normalize_fields` binds the frame as it goes, and
    design.md:94 says parameter defaults "can read parameters". With
    `title: string = label`, a body that reads only `title`, and a descriptor that changes
    `label`, both descriptors omit `title`, so the raw comparison over recorded props
    (component-instance-tree/spec.md:5-8) finds them equal and keeps a stale `title`.
  - Handlers capture the whole frame (value.rs `Handler.captured`, specs/future.md:1069-1074). The
    design covers this with `slots_read` for props only.
- **Recommendation:** Define a read as any use of a slot's value during the render: a `slot` node
  evaluated, a formula captured (its `slots_read` slots), a handler created (the prop slots its
  body reads), and a prop default evaluated. Compare *normalized* props (after defaults), or count
  the props a default read as reads of the defaulted prop. Add scenarios for a formula override
  that captures a prop or state field, and for a defaulted prop that reads another prop.
- **Fix:** A read now includes slots `capture_formula` captures (a captured prop or state field is a read of the enclosing render) and the prop slots of each handler node the body binds (`slots_read`); props are compared after defaults resolve, so a defaulted prop that reads a changed prop counts. design.md Decisions 9 and 10; reactivity dispatch requirement; component-instance-tree prop requirement with scenarios for a formula-captured prop, a formula-captured state field and a defaulted prop. Refinement: a handler's *state* slots are not reads, because in the tree every handler runs against its creator with live state (Decision 9); counting them would make the `pressedAt` case render.
- **Verification:** Verified. Reads now cover `slot` nodes evaluated, slots `capture_formula` captures, and handler prop slots (`slots_read`). Props are compared after defaults resolve. Scenarios cover a formula-captured prop, a formula-captured state field and a defaulted prop. Excluding handler state slots is sound in the tree, because `dispatch_chain` only sends token invocations to the creator, where `invoke_handler` binds every state slot live (component.rs:1277-1278, 685-699).

### ✅ Verified - RF6 Keeping a render under a new descriptor keeps the parent's old handler allocations in its output, and the instance's old props, with no rebinding
- **Severity:** High
- **Confidence:** High
- **Evidence:**
  - Today an equal descriptor read from a new parent instance goes through `Across::descriptors`
    and `rebinding`/`rebind` (tree.rs:584-608, 667-779), so the kept output names the parent's new
    handler allocations. `creator` finds a handler's owner by identity (tree.rs:935-961), and the
    main spec requires that identity (component-instance-tree "A handler runs against the instance
    whose body bound it"). The prop skip (component-instance-tree/spec.md:3-10) keeps the output
    when an *unread* prop differs, but `Across::descriptors` returns false whenever any prop
    differs. So no rebinding happens. The kept output, for example a `Stack`'s content with a
    `Button` handler equal by value, then holds the parent's retired allocation. A tap then either
    fails with `nx-ir-handler-token` after a pass or, once the node's `through` names the new
    parent, resolves to the child as the creator and patches the wrong instance.
  - "Takes the new descriptor" is not enough. A later dispatch builds the body's frame from
    `instance.props` (component.rs:1342-1349), so the instance needs the new normalized props.
    Producing them means a normalization with charged checks, which no API offers without
    rendering.
  - "Compared by value" over raw descriptors would compare handler tokens by text, which the main
    spec forbids ("The text of a token SHALL NOT count").
  - "charged as `same_descriptor` is charged today" (design.md:189) means free:
    `same_descriptor` is unmetered (tree.rs:1327). Only `Across` meters handler comparisons.
- **Recommendation:** Specify the prop skip as an extension of the rebinding path: compare the
  recorded props with `Across` semantics (handlers through `before`/`now`, recorded in a
  `Rebinding`), rebind the kept output and the nodes initialized through it, and produce a new
  instance whose props are the new descriptor's normalized props, with that cost stated. Say
  exactly what the comparison is charged. Add a scenario: a `Stack` whose unread prop changes
  keeps its output, and a tap on its content's button still patches the parent.
- **Fix:** The prop skip now normalizes the new props (an internal entry, charged as initialization charges normalization, which the instance needs anyway), compares the recorded props with `Across` semantics under `Program::metered` with a `Rebinding`, rebinds through `rebinding`/`rebind` when the parent's instance or a handler changed, and gives the instance the new props without rendering. design.md Decision 10; component-instance-tree prop requirement with the `Stack` scenario.
- **Verification:** Verified. The prop skip normalizes the new props through an internal entry, compares the recorded props with `Across` semantics under `Program::metered`, rebinds via `rebinding`/`rebind`, and gives the instance the new props. Two issues the fix introduced are recorded as new findings: the `Stack` scenario cannot hold while handlers capture the whole frame (RF19), and a prop skip changes the part of the instance that `is` compares (RF20).

### ✅ Verified - RF7 The dispatch skip has nothing to return: neither runtime's instance holds its rendered output, and the spec does not say what happens to generations, tokens, written-output charges or origins
- **Severity:** High
- **Confidence:** High
- **Evidence:**
  - `InstanceData` (component.rs:27-45), the TypeScript `NxComponentInstance` (index.ts:1354-1366)
    and the stored form `Stored` (stored.rs:42-52) hold no rendered output. "Return the previous
    rendered output" (reactivity/spec.md:98-101) needs a new field in both runtimes and in storage,
    which tasks 4.1-4.2 do not list.
  - Tokens are `h<generation>-<n>`, and `InstanceData::handler` accepts only the instance's
    generation (component.rs:403-415). The spec does not say whether a skipped dispatch keeps the
    generation. If it increments, the old output's tokens stop resolving.
  - The main specs require the opposite. rust-ir-runtime (spec.md:207-209: "one re-render after the
    batch, and fresh tokens on every lifecycle render") and typescript-ir-runtime (spec.md:262-265:
    "the body SHALL be re-rendered once … its handlers SHALL receive fresh tokens") have no delta.
    Corpus lifecycles record each batch's rendered output with literal tokens
    (ir-conformance/README.md:104-112), so `results.json` changes, which contradicts "No result
    changes" (proposal.md:72). Task 6.2 regenerates only `operations.json`.
  - Writing the rendered output for the host is charged ("every value an evaluation returns is
    written"). The spec does not say whether a skipped dispatch writes the old output again, and
    both runtimes must count it the same way.
  - With an origins report, the output's origins are collected while it is written. A skip that
    returns stored output must still report them, or the preview loses its links (value-origin).
  - "A stored instance SHALL carry its record" (reactivity/spec.md:100) is of no use unless the
    output is stored as well.
- **Recommendation:** State that a skipped dispatch keeps the generation, the token numbering and
  the very handler allocations. Store the rendered value on the instance in both runtimes, or
  simpler, have a restored instance always render. Say whether the output is written and charged
  again, and that origins are reported. Add MODIFIED deltas for rust-ir-runtime and
  typescript-ir-runtime, regenerate `results.json`, and pin the skip with a corpus lifecycle case,
  since the corpus does have lifecycles.
- **Fix:** The dispatch skip is restricted to the instance tree, which holds rendered output. `dispatch_component_actions` and the TypeScript dispatch keep rendering every batch, so `rust-ir-runtime`, `typescript-ir-runtime` and the corpus's `results.json` are unchanged and need no delta. In the tree a skip keeps rendered output, tokens and generation, writes no output and reports no origins for it; it also requires the node's environment record to hold. The tree uses one internal Rust lifecycle call that applies the handlers and renders only when needed, so the budget still covers the whole dispatch (a refinement of "an entry that returns state and effects without rendering"). Instance records are in memory only; the stored form is unchanged. Updated proposal, design.md Decision 9, the reactivity requirement (with a bare-lifecycle scenario) and tasks 4.x/5.3.
- **Verification:** Verified. The skip is restricted to the tree, which holds rendered output (`Node.rendered`). The bare APIs and the corpus are unchanged, so no runtime deltas are needed. Generation, tokens and handler allocations are kept, no output is written and no origins are reported, and an internal entry keeps one budget per dispatch (tasks 4.3, 4.5, 5.3). The stored form is unchanged, which answers the restore question.

### ✅ Verified - RF8 A dispatch that keeps its render returns a new instance, and the tree uses instance identity to mean "this render"
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `Node.through`, `visited`, `read_from_current`, `walked_anew`, `rebinding` and
  `creator` all compare `ComponentInstance::is` (pointer identity, tree.rs:379-399, 556-560,
  706-712, 773-778). A non-rendering dispatch must return a new instance, since it carries new
  state, while its output is unchanged. After it, every child's `through` names an instance the
  parent no longer holds. So a later visit takes the metered rebinding path (`!through_same`,
  tree.rs:584) instead of the free equal-descriptor path, and a node visited earlier in the same
  pass counts as "read from output it no longer holds". Its output is the same, so this costs
  operations rather than giving wrong output, but counts then depend on whether a skip happened,
  and nothing specifies that.
- **Recommendation:** Give a render an identity of its own, such as the rendered allocation or a
  render id, and compare that wherever the tree means "the same output", or have a skipped dispatch
  update the children's `through`. Specify it in the component-instance-tree delta.
- **Fix:** A dispatch that keeps its render keeps the instance's render identity: the instance's render part (handlers, generation, tokens) is shared, and `ComponentInstance::is` compares it, so `through`, `visited`, `creator` and the rest still match and nothing is rebound; only the state changes. design.md Decision 9, the reactivity dispatch requirement and task 4.2.
- **Verification:** Verified. Task 4.2 splits the instance into a shared render part and makes `ComponentInstance::is` compare it, so after a kept dispatch `through`, `visited`, `read_from_current`, `walked_anew`, `rebinding` and `creator` still match. See RF20 for the same question after a prop skip.

### ✅ Verified - RF9 "The state a handler captured is never read" is false for handlers run without live state, and the dispatch skip ignores a changed environment
- **Severity:** Medium
- **Confidence:** High
- **Evidence:**
  - design.md:177-181 argues that keeping the old handlers is sound because handlers read state
    live. But a handler whose owner is not the dispatched instance runs with `live = None` on its
    captured frame: parent-bound `handler_props` for an emitted-action entry (component.rs:1335),
    and a handed-down handler named by token (component.rs:1277-1278). This matches
    typescript-ir-runtime spec.md:255-261 ("every other captured value SHALL be the value
    captured"). A host using the bare API that dispatches an emit at a child after the owner's
    dispatch was skipped gets results computed from the owner's pre-dispatch state, and with the
    skip nothing ever refreshes those handlers.
  - `dispatch_component_actions` renders in the environment its options give. The skip decides
    from state reads only (reactivity/spec.md:94-101), so a bare-API dispatch under different
    roots or `$env` returns output from the old environment. Only the tree renders again eagerly on
    `set_environment`.
- **Recommendation:** Either record the state slots that non-live handlers in the output capture
  and read as reads, or limit the claim to owned handlers and state the consequence. Make the skip
  also require `environment_reads_hold`, or the kept-result validity, for the dispatch's
  environment. Add bare-API scenarios for both.
- **Fix:** Resolved by RF7's restriction: the bare APIs keep rendering, so a bare-API handler run on its captures is never left stale. In the tree, every handler, emits included, runs against its creator with live state (design.md Decision 9). The tree's skip also requires the node's environment record to hold.
- **Verification:** Verified. Both problems are gone. The bare APIs keep rendering, so a non-live handler there never sees captures a skip left stale. In the tree, every handler runs against its creator with live state, and the skip also requires the node's environment record to hold (Decision 9).

### ✅ Verified - RF10 The equality cutoff keeps the old value under `==`, which equates values that are written differently and ignores origins
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** design.md:214-218 keeps the old value when the new one is equal "charged as a
  comparison of values". `values_equal` treats `Int(1)` and `Float(1.0)` as equal and compares
  floats with `==`, so `0.0` equals `-0.0` (value.rs:241-243). Origins "SHALL NOT take part in
  value equality" (value-origin spec). So an untyped or `object` result, or a record that a branch
  now builds at another element, can be kept with a representation or origin that a fresh
  evaluation would not give. That changes written output or the origins report, which contradicts
  "No result changes" (design.md:35).
- **Recommendation:** Cut off only on an equality that implies identical written output and
  origins: same representation, same origins when origins are reported. Or keep the new value
  while leaving `changed` alone, provided dependents' outputs are then provably identical. State
  which, and add a scenario with `1` versus `1.0`.
- **Fix:** The cutoff uses the identity a host would see in written output: same number kind, `-0.0` distinct from `0.0`, NaN never the same (no cutoff), and equal origins when the host collects them, charged as a comparison. design.md Decision 11, component-instance-tree kept-results requirement, and a reactivity scenario with `1` versus `1.0`.
- **Verification:** Verified. The cutoff compares by written identity: number kind, `-0.0` versus `0.0`, NaN never equal, and origins when collected. It is charged as a comparison, and the `1` versus `1.0` scenario was added.

### ✅ Verified - RF11 Dropping variants that no render used during a pass throws away the palette after almost every pass
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** design.md:223-225 and component-instance-tree/spec.md:34-35 drop a variant "no
  render used during a pass" at `finish`. After the first pass most passes render nothing (settled
  nodes are skipped), and dispatch and `set_environment` renders happen between passes, not
  "during" them. So the palette the first click reused (scenario at spec:39-43) is dropped by the
  next `finish`, and the second click evaluates it again. That defeats the change's purpose for
  design tokens.
- **Recommendation:** Tie a kept result's lifetime to the records that reference it: drop it when
  no live node, and no kept result reachable from one, has an edge to it. The reverse edges of
  Decision 12 already give this, as a reference count. Add a scenario of two dispatches separated
  by a pass that renders nothing.
- **Fix:** A variant is dropped when no node record and no kept-result record references it (reference counts over the edges), checked when the tree operation ends, not when a pass did not render it. design.md Decision 11 "Lifetime"; scenario "A kept result survives passes that render nothing"; task 5.6.
- **Verification:** Verified. Variant lifetime now follows the records that reference it, dropped at the end of the tree operation, with the scenario "A kept result survives passes that render nothing" and task 5.6.

### ✅ Verified - RF12 "Override-supplied reads compare by value" and "taken from the tree into the call's cache" are underspecified for formulas, field paths, bundles and the render's own overrides
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:**
  - Getting the value an override supplies at the use means evaluating it. A `$env` formula
    (`env.gap={env.space * 2}`), a field-path override on a base, or a bundle entry all evaluate,
    and their reads would themselves use kept results. The charging "as the skip check's reads"
    (design.md:219-222) is then circular, because `environment_reads_hold` runs
    `environment_value`, which the tree would answer from kept results.
  - Formulas are identified by "the layer that holds it" (design.md:89). Layers are numbered per
    call, so a `$env` formula has no identity across renders. A formula in an entry-module bundle
    provide is evaluated by `source_value` with `Below::Provide` and has no layer at all
    (environment.rs `provided_or_default`, bundle arm).
  - Results evaluated under the render's *own* `envProvide` layers (position ≥ `base`) are neither
    root reads nor overrides "above the node". The design does not say whether the tree keeps them.
  - "A result taken from the tree enters the call's own cache" (design.md:227-229) needs tokens
    for the current call, and the spec does not say how they are derived.
  - The tree calls `Program::initialize_component`/`dispatch_component_actions`, which take no
    cache, so a new API parameter is needed and neither the specs nor the tasks list it.
- **Recommendation:** Specify which results the tree keeps (for example only defaults, entry
  provides and values whose reads were all root or `$env`-value supplied), how a formula or bundle
  entry is identified across calls (formula data plus key, or provide index plus entry index), how
  an override-supplied value is obtained and charged, how tokens are assigned to a tree result
  entering a call, and the API through which the tree passes results to a call.
- **Fix:** Specified in design.md Decision 11 and the kept-results requirement: an override-supplied read at a use is the value the environment lookup gives there, evaluating any formula and taking any needed result from the tree by the same rule (it cannot need the variant being validated), charged as the skip check charges; formulas and in-render bundles are reused within a call only; results with a read under a layer the render pushed are not kept; a tree result enters the call's cache as if evaluated at the call's start, with tokens over the host layers taken at the use; the tree passes its store through an internal runtime option, not a public API.
- **Verification:** Verified for the parts listed. Formulas and in-render bundles are reused within a call only. Results with a read under a render-pushed layer are not kept. Override-supplied values come from the lookup, and the lookup cannot need the variant being validated. The tree passes results through an internal option. One gap the fix introduced is RF18: deciding whether a result can be taken into a render, and giving it tokens, needs its transitive reads, which the direct-input record does not hold.

### ✅ Verified - RF13 All-or-nothing changes do not cover the new tree state
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `atomically` snapshots and restores `nodes`, `visited`, `inert`, `roots` and
  `readers` (tree.rs:781-800). `set_environment` and `dispatch` are all or nothing (main spec "A
  change to the tree is all or nothing", and MODIFIED requirement at
  component-instance-tree/spec.md:113-114). A failed render after kept results were evaluated
  again, revisions bumped, reverse edges rewritten or the change set appended would otherwise leave
  them describing a tree that was rolled back. Neither the design nor tasks 5.2-5.4 mention it.
- **Recommendation:** Add the kept results, the revision counter, the reverse indexes and the
  change set to `atomically`'s snapshot, and add a task and scenario: a `set_environment` whose
  second render fails leaves kept results and `take_changes` as they were.
- **Fix:** `atomically` snapshots and restores the kept results, the revision counter, the reverse indexes and the change set with the nodes (design.md Decision 11, MODIFIED "A change to the tree is all or nothing", the kept-results requirement with scenario "A failed change leaves the kept results as they were", task 5.7).
- **Verification:** Verified. `atomically` now snapshots the kept results, revision counter, reverse indexes and change set (Decision 11, MODIFIED "A change to the tree is all or nothing", task 5.7, and a scenario).

### ✅ Verified - RF14 Main specs and the kept sentences of modified ones contradict the change, and no deltas update them
- **Severity:** Medium
- **Confidence:** High
- **Evidence:**
  - The nx-ir-format delta keeps "a function of the linked images and the values the host supplies
    alone" (specs/nx-ir-format/spec.md:4-5), while tree renders now depend on held state
    (component-instance-tree/spec.md:37).
  - reactivity/spec.md:43 says "Results SHALL NOT be kept from one call to the next", which the
    tree does.
  - environment-values "Every environment value is computed where it is read" ("SHALL be computed
    at each read", spec.md:352) has no delta.
  - component-instance-tree "Props flow down and state stays with the node" requires that "a node
    visited with any other descriptor … SHALL be initialized again" (spec.md:88-92), which the
    prop skip contradicts, and it is not MODIFIED.
  - rust-ir-runtime and typescript-ir-runtime require a re-render after every batch (RF7).
  - specs/future.md:1069-1074 says a handler captures the whole frame. Task 7.3 rewrites that
    section but does not say this premise of Decision 10 stays.
- **Recommendation:** Add MODIFIED deltas for environment-values, component-instance-tree "Props
  flow down…", rust-ir-runtime and typescript-ir-runtime. Qualify the cost requirement's "alone"
  and reactivity:43 with "outside the instance tree".
- **Fix:** Added MODIFIED deltas: environment-values "Every environment value is computed where it is read" (results unchanged; a runtime may reuse an earlier result as reactivity defines) and component-instance-tree "Props flow down and state stays with the node" (the prop-read skip). Edited the already-MODIFIED nx-ir-format cost requirement's first sentence (a function of the images, the input and, in the tree, its held state) and changed reactivity's "Results SHALL NOT be kept" to "except as the instance tree keeps them". No rust-ir-runtime or typescript-ir-runtime delta is needed after RF7. Beyond the list, also MODIFIED component-instance-tree "The Rust runtime exports an instance tree", "A pass drops the nodes…", "A handler runs against the instance whose body bound it", "An emitted action is carried…" and "A change to the tree is all or nothing", whose text or scenarios assumed every dispatch renders (for example the "event from output a dispatch replaced" scenario now has the body read the patched field). Task 7.3 now says the "handler captures the whole frame" premise stays.
- **Verification:** Verified. There are MODIFIED deltas for environment-values (reuse allowed, same value) and for component-instance-tree "Props flow down" plus the other affected requirements. The nx-ir-format cost sentence now names the tree's held state, and reactivity's "SHALL NOT be kept" makes an exception for the tree. No runtime deltas are needed after RF7. Task 7.3 keeps the whole-frame capture premise.

### ✅ Verified - RF15 The change set's semantics are ambiguous for replaced nodes, dropped subtrees and reused keys
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** design.md:248-256 and component-instance-tree/spec.md:77-84 cover added then
  removed, added then rendered, and rendered twice. They do not cover:
  - a node replaced at the same key by another component or parent (`visit` at tree.rs:612-655
    drops what is under it and puts a new node), which could be listed as removed and added,
    rendered, or nothing;
  - nodes removed by `drop_under`, by `finish`, or by `remove` under a removed node;
  - a key removed and then added again before the host takes the set.
  Nodes that take a new descriptor without rendering (prop skip, rebinding) are also not reported,
  although what the host holds for them, the descriptor, changed.
- **Recommendation:** Define the set as a per-key diff between the last `take_changes` and now
  (absent→present = added, present→absent = removed, present→present with another render
  identity = rendered, replaced = removed and added), and add scenarios for replacement and for a
  dropped subtree.
- **Fix:** The change set is a diff by key since the last `take_changes`: a node replaced at its key is removed and added; every node dropped with a removed or replaced subtree is listed as removed; a key removed and visited again is removed and added; a node whose output changed in any way, rebinding of what its tokens name included, is rendered; a node that only took a new descriptor or state is not listed. design.md Decision 13, the change-set requirement with scenarios for replacement, a dropped subtree and a rebound node.
- **Verification:** Verified. The change set is a diff by key, covering replacement as removed and added, every dropped node, a reused key, rebinding as rendered, and a descriptor or state change alone not listed. Scenarios and task 5.8 match.

### ✅ Verified - RF16 Smaller gaps: within-call reuse changes depth-limit failures, the cost wording is ambiguous, and tests are missing
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:**
  - A default is evaluated at the reader's `cx.depth` (environment.rs `provided_or_default`,
    `depth: cx.depth`). Reusing a result first evaluated shallowly can turn a read that hits a
    depth or native-stack limit today into a success. That is a result change in edge cases.
  - Decision 9 charges the state comparison "as `diff` compares fields" but does not say whether
    over all state fields or only patched ones (`patch_state` already knows the patched fields,
    component.rs:815-822). Both runtimes must agree.
  - Replaying a reused result's reads into an enclosing derived result needs each token computed
    again against *that* result's start (design.md:112-113, 128-131). The design calls tokens free
    but never says they are recomputed rather than copied.
  - In the tree, which node pays for a shared kept result depends on host visit order, so per-node
    budget failures move with it. That is acceptable but should be stated.
  - The generated-code scenario "evaluated once" (executable-code-generation/spec.md:3-6) cannot be
    observed without a hook, and task 3.2 does not say how it is tested.
- **Recommendation:** State these rules in design.md and the deltas. Pin the depth case or accept
  it explicitly, choose all fields or patched fields for the dispatch comparison, and describe the
  instrumentation the generated-code test uses.
- **Fix:** Documented in design.md (Decisions 4, 7, 9, 11, Goals and Risks) and the deltas: reuse can keep a call under a depth or operation limit a fresh evaluation reaches, alike in both runtimes (reactivity within-call requirement, proposal); the tree's state comparison is charged as `diff` charges a field, over the patched fields the render read; tokens are computed again, not copied, when a reused result's reads are added to an enclosing record; a shared kept result is paid for by the render that evaluates it first, so counts depend on host call order as held state does; the generated-code test uses a test-only evaluation counter in the generated helpers (executable-code-generation delta, tasks 3.2-3.3). Refinement: the comparison covers only patched fields the render read, since an unread field cannot cause a render.
- **Verification:** Verified. Each point is stated: limits may be avoided alike in both runtimes, the comparison covers only patched fields the render read, tokens are recomputed on replay, a shared result is paid for by the first render that evaluates it, and generated code has a test-only evaluation counter (tasks 3.2-3.3). This finding's Fix line had been appended after the Summary; it has been moved here.

## Questions
- Is the intent that the tree's kept results include anything evaluated under a render's own
  `envProvide` layers, or only results whose reads come from roots and `$env` values (RF12)?
- Should a restored instance ever skip a dispatch, given that the stored form would have to carry
  rendered output (RF7)?
- Would it be simpler to ship decisions 1-8 (within-call reuse, which is what `design-tokens-theme`
  needs) separately from the tree's revision scheme, which carries most of the open soundness
  questions (RF1-RF3, RF10-RF13)?
  - **Answer:** No. The product owner wants the whole model in one change so it can be judged as
    a whole; the within-call part stays in this change, and the tree findings are fixed here.
- **Answers to the first two:** the tree keeps only results none of whose reads came from a layer
  the render pushed; results under the render's own `envProvide` are reused within the call only
  (RF12). A restored instance never skips a dispatch: the skip exists only in the tree, and the
  stored form is unchanged (RF7).

## Summary
- Within-call reuse by layer tokens (decisions 1-7) is sound for defaults, targeted provides,
  values and layer formulas, because the stack only changes at the top. The exceptions are caps
  and bundle provides (RF4). The rest of the change is less settled. The kept-results scheme
  classifies reads by use site incorrectly (RF1), uses two incompatible node records (RF2) and
  leaves validation order open (RF3). The state and prop skips miss reads made by frame capture
  (RF5), skip the handler rebinding the tree relies on (RF6), and need rendered output that
  instances do not hold (RF7). Several main specs contradict the change and have no deltas (RF14).
  The design should be revised before implementation.
- Verification (2026-11-24): RF1-RF16 are verified. The revised fixes introduced four new
  findings: RF17 (the edge rule defeats the equality cutoff), RF18 (taking a result into a render
  needs its transitive reads), RF19 (a scenario contradicted by whole-frame capture) and RF20 (a
  prop skip changes what `is` compares, and normalization may be charged twice).

## New Findings Discovered During 2026-11-24 18:29 Verification

### ✅ Verified - RF17 The edge rule treats an invalid used result as a changed input, so the equality cutoff never takes effect
- **Severity:** High
- **Confidence:** High
- **Evidence:** design.md:348-349 and specs/component-instance-tree/spec.md:62-63 say an edge holds
  only when "the variant it names is valid at the use … and its `changed` is no later than this
  variant's `verified`", and that checking stops at the first input that does not hold, after
  which the result is "evaluated again without checking the rest" (design.md:356;
  spec.md:66-67). Take the `direction` example. `locale` changes, so `direction`'s variant is not
  valid. The node's edge to it therefore does not hold, and the node renders, without `direction`
  being evaluated again and compared. That contradicts "An equal result stops a change from
  spreading" (reactivity/spec.md:122-126), "A node whose inputs come back equal is left alone"
  (component-instance-tree/spec.md:138-141), and Decision 12's own "evaluating again only those
  that are not valid, with equality cutoff" (design.md:416-418). Salsa's deep verify handles an
  input query that is not valid by executing it again and then comparing its `changed` revision.
  Doing so is safe here for the reason RF3's fix gives: the edge was read after inputs that all
  hold.
- **Recommendation:** Rewrite the edge rule: when the used variant is not valid at the use,
  evaluate it again there (with the cutoff), and the edge holds when its `changed` is then still no
  later than this record's `verified`. Use the same wording in Decision 11, the kept-results
  requirement and Decision 12.
- **Fix:** The edge rule is now Salsa's deep verify: the used variant is validated first at the use and, when not valid, evaluated again there with the equality cutoff; the edge holds when its `changed` is then no later than the user's `verified` (an evaluation that gives another variant counts as changed). Same wording in design.md Decisions 11 and 12, the kept-results and notification requirements of component-instance-tree, and the reactivity "direct inputs" requirement; added the scenario "A used result is evaluated again before its edge is judged", and the `direction` walk-through in Decision 11. Task 5.5.
- **Verification:** Verified. Decision 11 now checks an edge the way Salsa's deep verify checks an input query: the used variant is validated at the use, evaluated again there with the cutoff when it is not valid, and the edge holds when its `changed` is no later than the user's `verified`. An evaluation that gives another variant counts as changed. Decision 12, the kept-results and notification requirements and the reactivity requirement use the same wording. The `direction` walk-through and the scenario "A used result is evaluated again before its edge is judged" match. Evaluating the used result is safe because the edge was read after inputs that all still hold.

### ✅ Verified - RF18 Taking a kept result into a render needs its transitive reads, but the record holds only direct inputs
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:** design.md:386-391 takes a tree result into a render "when no layer the render
  pushed touches a value the result read", and enters it into the call's cache "its reads with the
  tokens they have over the call's host layers". Decision 4's reuse within the call and replay into
  enclosing records also need every read with its token (design.md:155-159). But Decision 8 records
  only direct inputs and says reads inside a kept result "are not copied … the edge stands for
  them" (design.md:218-224). For `button` → edge to `color` → root read of `colorMode`, a render
  inside `provide env.colorMode=dark` that checks only `button`'s direct inputs finds nothing
  touched and takes the light `button`. Validity at a use classifies reads by the place's host
  layers only, so it does not catch this either.
- **Recommendation:** State that both checks walk edges transitively, which is bounded by the
  program's declarations and free of operations. Alternatively, keep with each variant the
  flattened set of values it read, used only for this check and for tokens. Add a scenario: a
  render that overrides `colorMode` itself and reads `env.button` evaluates `button` and `color`
  again.
- **Fix:** Each variant keeps, beside its direct inputs, a flattened set of its leaf environment reads at any depth (through edges) with their supplier class. It is used only for the "no layer the render pushed touches its reads" check and for the tokens a taken result enters the call with (and so the replay into enclosing records); validation still uses direct inputs. design.md Decisions 8 and 11, the kept-results and reactivity requirements, the scenario "A render that overrides a value a kept result read below an edge evaluates it again", task 5.4.
- **Verification:** Verified. Each variant keeps a flattened set of its leaf reads (through edges, with supplier class), used only for the check against layers the render pushed and for the tokens a taken result enters the call with. Validation still uses direct inputs. The `button` → `color` → `colorMode` case is in Decision 11 and has a scenario.

### ✅ Verified - RF19 The scenario "A kept render under a new parent instance is rebound" cannot hold while handlers capture the whole frame
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** In specs/component-instance-tree/spec.md:43-48, `Page` renders
  `<Stack note={note}><Button onTapped=<State.Update taps={taps + 1} /> /></Stack>` again with
  another `note`, and `Stack`, which reads only its content, is expected to keep its render. The
  content's `Button` handler captures `Page`'s whole frame (`Handler.captured`, value.rs:90; the
  premise task 7.3 keeps), `note` included. The `Across` comparison of the content prop compares
  handlers as `==` does, capture slot by slot (`handlers_equal`, value.rs:361-381), so the new
  handler is unequal, the content prop differs, and `Stack` renders again. specs/future.md:1069-1074
  records exactly this limitation.
- **Recommendation:** Change the scenario to one where `Page`'s frame is unchanged apart from what
  `Stack` does not see. For example, the parent is initialized again from equal props, or the
  changed value is a prop of `Stack` that no handler's frame holds, such as one `Page` gets from
  its own parent through a field of a record. Or narrow captures to `slots_read` in this change,
  which future.md says touches the image format and both runtimes.
- **Fix:** Handlers now capture only the slots their node reads (`slots_read` of the handler node, computed by the runtime from the image at no charge, cacheable per node), in both IR runtimes, with no image format change. New ADDED requirement in a component-action-handlers delta ("A handler captures only the slots its body reads", with scenarios); the nx-ir-format cost bullet ("A handler captures the slots its node reads") and its handler-comparison scenario are edited in the already-MODIFIED requirement. No main requirement says a handler captures the whole frame, so no other MODIFIED delta was needed. design.md Decision 10 notes that this changes handler comparisons and so operation counts, which the corpus regeneration covers (task 6.2). The `Stack` scenario is kept and now holds. Tasks 1.6, 2.2 and 7.3 (future.md's "What a handler captures" is now marked done, replacing the earlier "keep the whole-frame premise").
- **Verification:** Verified. Handlers now capture only the slots their node reads (component-action-handlers delta, and the nx-ir-format comparison bullet and scenario), so the `Stack` scenario holds: the content handler reads only `taps`. One consequence of the narrowing is recorded as RF21.

### ✅ Verified - RF20 A prop skip changes the part of an instance that `is` compares, so the children's `through` stops matching again
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** Task 4.2 puts "handlers, generation, tokens, props and records" in the shared
  render part that `ComponentInstance::is` compares (tasks.md:45-47). A prop skip gives the
  instance new normalized props, and its own handlers too when rebinding replaced them, while it
  keeps the output (design.md:314-316). With props in the compared part, `is` is false. The
  children's `through` and the pass's `visited` record then name a different instance, and the
  next visit of a child takes the metered rebinding path. That is RF8's problem again, for prop
  skips. Separately, when a recorded prop differs, the props are normalized for the comparison and
  then again by `initialize_component`, so normalization is charged twice, and nothing says
  whether that is intended.
- **Recommendation:** Keep props out of the part `is` compares, or have a prop skip without
  rebinding update the children's `through`. State whether initialization after a failed prop
  comparison reuses the normalized props, or charges normalization again.
- **Fix:** Props, handler props, state and records are outside the render part that `ComponentInstance::is` compares; the identity is the render (its generation's handlers and tokens), so a prop skip that rebinds nothing keeps children's `through` and the pass's record matching (design.md Decision 10, prop requirement, task 4.2). A node whose recorded prop differs is initialized from the props already normalized for the comparison through an internal entry, so normalization is charged once (design.md Decision 10, prop requirement, task 4.4).
- **Verification:** Verified. Props, handler props, state and records are outside the part `is` compares, so a prop skip that rebinds nothing keeps children's `through` matching, while a rebinding still changes the handlers and so the identity. A node whose recorded prop differs is initialized from the props already normalized for the comparison, so normalization is charged once (Decision 10, task 4.4).

## New Findings Discovered During 2026-11-24 18:34 Verification

### ✅ Verified - RF21 Narrowing handler captures changes what `==` on handlers returns, contradicting "Results do not change", and leaves the order of captured slots unspecified
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** Under the component-action-handlers delta ("A handler captures only the slots its
  body reads"), two handlers from one node that differ only in a slot the handler does not read now
  compare equal. Today `==` finds them unequal (`handlers_equal` compares the whole captured frame,
  value.rs:361-381). `==` on handlers is part of the language (the nx-ir-format comparison rules),
  so a program that compares handlers, or stores a comparison's result, can give a different
  result. design.md Decision 10 nonetheless says "Results do not change", and task 6.2 expects
  `results.json` to be unchanged. The comparison also stops "at the first slot that differs", so
  its operation count depends on the order of the captured slots. The delta requires the runtimes
  to "capture alike" but does not fix that order. Rust's `slots_read` returns them in the order a
  depth-first pop visits them (environment.rs `slots_read`).
- **Recommendation:** State in design.md and the component-action-handlers delta that handler
  equality now ignores unread slots, as a deliberate change of results, and let task 6.2 accept
  any `results.json` diff this causes. Specify the capture order, for example ascending slot index,
  so both runtimes compare in the same order.
- **Fix:** Stated as a deliberate change of results: two handlers made by one node that differ only in a slot the node does not read are now equal under `==`, and what a handler does when it runs does not change. This is in design.md (Goals, Decision 10, Risks), proposal.md (handler bullet and cost-model bullet, where "No result changes" became "Reuse changes no result", with handler equality named as the one intended change) and the component-action-handlers delta. Captured slots are held and compared in ascending slot index in both runtimes, whatever order `slots_read` finds them in (component-action-handlers delta with a new scenario, the nx-ir-format comparison bullet, tasks 1.6 and 2.2). Task 6.2 now accepts `results.json` diffs from programs that compare such handlers and lists them in the PR description.
- **Verification:** Verified. The equality change is now stated as deliberate, in design.md (Goals, Decision 10, Risks), proposal.md and the component-action-handlers delta, which also says what a handler does when it runs is unchanged. Captured slots are held and compared in ascending slot index in both runtimes, with a new scenario and matching wording in nx-ir-format and tasks 1.6 and 2.2. Task 6.2 accepts and lists the `results.json` diffs this causes. The main value-equality spec says nothing about handler captures, so no further delta is needed. No new issues.

## New Findings Discovered During 2026-11-24 21:49 Review

**Reviewed artifacts:** proposal.md, design.md, tasks.md, all six delta specs.  
**Reviewed code:** `git diff origin/main...HEAD`: `crates/nx-ir-runtime/src/{reuse.rs,kept.rs,environment.rs,eval.rs,component.rs,stored.rs,tree.rs}`, `runtime/typescript/src/index.ts`, `crates/nx-codegen/src/runtime/environment.js`, `crates/nx-codegen/src/emit.rs`, `crates/nx-api/tests/codegen/ir_instance_tree_tests.rs`, `docs/nx-ir-format.md`.  
**Method:** Each finding marked "confirmed" was reproduced with a throwaway instance-tree test, not committed. Where a finding says "regression", the same test passes on `origin/main` (744aa50a).

### ✅ Verified - RF22 A kept variant is reused at another place when an override-supplied value only compares equal under `values_equal`, so the reused result differs from a fresh evaluation
- **Severity:** High
- **Confidence:** High (confirmed)
- **Location:** `crates/nx-ir-runtime/src/kept.rs` `lookup_equals` (line 553), used by `input_holds` for `Input::Override` and `Input::Beneath`
- **Evidence:** Validation compares an override-supplied read with `values_equal`. That function equates `0.0` with `-0.0`, equates `Int(1)` with `Float(1.0)`, and ignores origins. The equality cutoff uses `same_written` because RF10 found exactly this problem. The validity check was left on `values_equal`, and it decides more than the old per-node skip did: a variant kept at one place is now taken into renders of other nodes at other places. Failure scenario: `environment boxed: Box = { <Box v={env.z} /> }`, and `<Card>` renders `<Num Value={env.boxed.v} />`. A page renders `<Stack env.z={0.0}><Card /></Stack>` and then `<Stack env.z={-0.0}><Card /></Stack>`. The second card takes the first card's variant and writes `Value: 0.0`. A fresh evaluation of the same card writes `-0.0`. The same thing happens for an `object`-typed value overridden with `1` in one section and `1.0` in another, and for records equal in their fields but built at different nodes when origins are collected. The root-change detection in `change_environment` (`tree.rs:350`) also uses `values_equal`, so changing a root value from `0.0` to `-0.0` renders nothing (confirmed). That comparison was already there before this change, but it contradicts the new rule that a root value that changes takes a new revision.
- **Recommendation:** Compare override-supplied and beneath reads with `same_written`, including origins when the call collects them, and detect root changes the same way. Add a tree test with two sections that override a value to `0.0` and to `-0.0`.
- **Fix:** `lookup_equals` (`kept.rs`), used for `Input::Override` and `Input::Beneath`, now compares with `same_written`, origins included when the call collects them, so a variant kept under `env.z={0.0}` is not taken under `env.z={-0.0}`, nor `1` for `1.0`. `change_environment` (`tree.rs`) detects a changed root value with `same_written` too, so `0.0` to `-0.0` takes a new revision and renders its readers. Tests: `a_kept_result_is_not_taken_where_an_override_supplies_a_value_written_otherwise`, `a_root_value_changed_to_one_written_otherwise_is_a_change`. Spec: the kept-results requirement (root changes and override-supplied reads by the cutoff's rule, new scenario), the reactivity records requirement, design Decisions 11 and 14, `docs/nx-ir-format.md`.
- **Verification:** Verified. `lookup_equals` and the root-change check in `change_environment` now use `same_written`. My probes pass: the second card under `env.z={-0.0}` writes `-0.0`, as a fresh evaluation does, and a root change from `0.0` to `-0.0` renders `-0.0`. The new tests cover both cases.

### ✅ Verified - RF23 A read reached through a formula entry of an entry-module bundle no longer depends on what the formula reads, so a root change leaves the node stale (regression)
- **Severity:** High
- **Confidence:** High (confirmed; passes on `origin/main`)
- **Location:** `crates/nx-ir-runtime/src/reuse.rs` `end_read` (the `parent.kind == TrackKind::Read` return at line 277, and `Input::Root` at line 309); `kept.rs` `keys_of` (line 232)
- **Evidence:** Decision 2 makes a formula among an entry-module bundle's entries "evaluated at each read", not a derived result of its own. A read of `gap` that the bundle supplies records `Input::Root { key: gap, edges: [bundle variant] }`. The formula's own reads, such as `env.space`, end in a `Read` track. That track passes only its edges up, so they become neither inputs nor index keys. `keys_of(Root)` is `[gap]` only. Failure scenario: `provide env with <env.Update gap={env.space * 2} />` in the entry module, and a `Card` renders `env.gap`. After `set_environment(space = 10)`, `tree.environment_value("root/0", "gap")` gives `20`, but the node still shows `8`. Nothing reaches it through `readers`, and its record would hold anyway. On `origin/main` the node renders `20`. The same gap affects a kept result that reads such a value.
- **Recommendation:** Record a bundle formula entry's reads as direct inputs of the read that used it, and index them in `readers`. Alternatively, make the entry a derived result keyed by provide and value, kept as a variant. Add the scenario as a tree test.
- **Fix:** Every track, a read's included, now holds direct inputs. A read nested in a read (a bundle's formula entry evaluated inside the read that found it) gives its own input to the outer read, and the outer root read adds its nested inputs after itself, so a read of `gap` records `Root { gap, [bundle] }` then `Root { space }`, indexed under `space`. The same applies to results the tree does not keep: `Tracked` now carries the direct inputs of a formula or a result evaluated under the render's own overrides, and its use gives them to the enclosing track in place of an edge (`reuse.rs` `add_use`, `end_read`, `derived`). This also fixed a formula in a body's own `provide` (`provide env.gap={env.space * 2} { ... }`), whose reads were dropped the same way. Tests: `a_read_through_a_bundle_formula_entry_follows_the_values_the_formula_reads`, `a_read_through_a_formula_the_body_provides_follows_the_values_the_formula_reads`. Spec: reactivity records requirement (sentence and scenario), design Decisions 2 and 8, `docs/nx-ir-format.md`.
- **Verification:** Verified. My probe now renders `20` after `set_environment(space=10)`. The outer root read forwards the nested `Root { space }`, which is indexed under `space`. The formula in a body's own `provide` is covered as well.

### ✅ Verified - RF24 A read beneath the render's own overrides is indexed only under the value read, so a root change of another value its source read is missed (regression)
- **Severity:** High
- **Confidence:** High (confirmed; passes on `origin/main`)
- **Location:** `crates/nx-ir-runtime/src/reuse.rs` `end_read` (`Input::Beneath { key, value: None }`, line 295); `kept.rs` `keys_of`; design.md Decision 14, "Reads under the render's own overrides"
- **Evidence:** `Input::Beneath` is indexed under its own key only. Decision 14 chose this so that a body that overrides `mode` and reads `loud` is not reached by a change of the root `mode`. But the source behind `loud` can read other values that the body does not override. Failure scenario: `environment loud: string = { env.mode + "!" + env.locale }`, and `<Card> = { provide env.mode="dark" { <Label Text={env.loud} /> } }`. After `set_environment(locale = "fr")` the node still shows `dark!en`, while a fresh render gives `dark!fr`. `readers[locale]` does not list the node. On `origin/main` it renders `dark!fr`, because the old record listed every read at any depth.
- **Recommendation:** Index a `Beneath` input under every value its source read beneath the overrides: the leaves of the re-read in `end_render`, less the values the body overrides wholly. Add the `locale` case beside the existing cascade scenario.
- **Fix:** A source evaluated under the render's own overrides is not kept, and now gives its direct inputs to the read that used it (same mechanism as RF23), so the body that overrides `mode` and reads `loud` records `Root { loud }` and `Root { locale }`, not `mode`. The read is then a plain root read; `Input::Beneath` remains only for a read that a layer of the place sets while a source was evaluated under the render's own overrides, where it sits beside those nested inputs. This is precise as well as sound: a `Beneath` value compared alone could miss a conditional source (`if env.mode == "dark" { env.locale } else { "x" }` looks unchanged beneath the override when `locale` changes). The test `a_read_that_fails_beneath_the_bodys_override_still_renders` changed: the record no longer holds a failed `Beneath`, so an equal visit and a root change of `divisor` keep the render. Test: `a_source_read_beneath_the_bodys_override_follows_the_other_values_it_reads` (root `mode` change keeps the render, `locale` change renders `dark!fr`). Spec: reactivity records requirement (sentence and scenario), design Decisions 8 and 14.
- **Verification:** Verified. My `loud` probe now renders `dark!fr` after a `locale` change, and a root `mode` change still keeps the render. The changed test `a_read_that_fails_beneath_the_bodys_override_still_renders` is right: the body overrides `divisor` itself, so a fresh render also gives `5`, and keeping the render is more precise than rendering again on every visit. One related case is still wrong: an override-supplied read inside a source evaluated under the render's own overrides. That is new finding RF30.

### ✅ Verified - RF25 The equality cutoff fails when a result has several variants and the first stale one belongs to another place, so nodes whose result came back equal render again
- **Severity:** Medium
- **Confidence:** High (confirmed)
- **Location:** `crates/nx-ir-runtime/src/kept.rs` `keep` (the `target` selection at line 631) and `validate`/`input_holds` (they stop at the first input that is `Stale` before later inputs are classified)
- **Evidence:** The spec says a re-evaluated result is compared with "the first variant of the result whose reads are classified at the place as they were recorded and that was found not valid there". `keep` instead takes the first variant whose memo is `Stale`. `validate` returns `Stale` at the first root read that changed, without classifying the inputs after it. So a variant kept for another place counts as replaceable here. Failure scenario: `color` reads `env.locale` and then `env.colorMode`. A light card and a card under `<Stack env.colorMode=dark>` each keep a variant, and the pull happens to check them in the order opposite to the order they were kept. After `set_environment(locale = "fr")`, which leaves both colors unchanged, the first node's evaluation overwrites the other place's variant and is compared with that variant's value. The values differ, so the overwritten variant takes a new revision, and `resolved != edge` makes the first node's edge `Other`. Then the second node does the same to the first node's variant. Both nodes render again (`take_changes().rendered == ["root/0", "root/1"]`), though neither output changed. Whether the cutoff works depends on key order. No test covers a cutoff with two variants.
- **Recommendation:** When validation stops at a stale input, still classify the remaining inputs, at no cost, before treating the variant as replaceable. Or choose the replacement target by classification alone. Add a two-variant cutoff test that checks keys in both orders.
- **Fix:** When `validate` stops at an input that is `Stale`, it classifies the inputs after it, at no cost (`classified_as_recorded`: a root read where no layer of the place sets the value, an override-supplied read where one does), and one classified otherwise makes the variant `Other`, so `keep` never picks another place's variant as its replacement target. Override values after the stopping input are not compared, since that would cost a lookup; two places that differ only in an override value can still pick each other's stale variant, which costs a render but not correctness (the other place's check then finds its variant `Other` and renders). Test: `an_equal_result_stops_a_change_whichever_variant_is_checked_first` (both page orders, both key orders). Spec: kept-results requirement (sentence and scenario), design Decision 11, `docs/nx-ir-format.md`.
- **Verification:** Verified. My probe that checks the keys in the order opposite to the order kept now leaves both cards unrendered, with an empty change set, in both page orders. The remaining case the fix note describes, two places that differ only in an override value, costs a render and never a wrong result, which is acceptable.

### ✅ Verified - RF26 Re-evaluating a kept result while a record is checked is charged to no reported call and runs under its own budget, contradicting the cost rules and the docs
- **Severity:** Medium
- **Confidence:** High (confirmed)
- **Location:** `crates/nx-ir-runtime/src/environment.rs` `Program::record_holds` (line 1568, "runs under its own budget and reports no usage"); callers in `tree.rs` (`visit`, `change_environment`, `dispatch_chain`); `docs/nx-ir-format.md:837`
- **Evidence:** A node's record check runs in a machine of its own. Through `edge_holds` that machine evaluates stale kept results again, which can be the expensive work such as rebuilding a palette. The render that follows takes those results for free. Failure scenario: `color` costs about 6,000 operations, and one `Card` reads it. `set_environment(colorMode = dark)` re-evaluates `color` in the check and renders the card. The usage reported is 7 operations. A fresh visit of the same card reports 6,037. The spec says a kept result "SHALL be paid for by the render that evaluates it first", and the docs say "evaluating a kept result again costs what the evaluation costs ... So a render's count is a function of its images, its input and what the tree holds". In practice no reported call pays for it. Also, the check and the render each get the full `max_operations`, so one operation can spend about twice the budget.
- **Recommendation:** Run the check in the render's or dispatch's machine, or carry its spending into that call's meter and usage. Otherwise state the actual rule in the spec and in `docs/nx-ir-format.md`. Add a usage assertion to the `a_root_change_makes_kept_results_that_read_it_stale` family.
- **Fix:** Made the cost match the spec. `Program::record_holds` now returns the operations the check spent, and the tree passes them to the lifecycle call that follows at the node (visit initialization or prop normalization, the pull's render, the dispatch) through a new `TreeCall` (`component.rs`): `call_in` starts that machine with `take_on(spent)`, so the check and the call share one budget and the call's usage includes the check. A check that holds with no lifecycle call after it (kept render, rebinding) is reported as a call of its own (`report_check`); a check that fails reports what it spent. A record verified at the current revision in the same environment, or an empty one, runs no check and reports nothing, so `keeping_a_child_leaves_the_usage_report_as_the_last_call_wrote_it` still holds. Tests: `a_kept_result_evaluated_again_during_a_check_is_charged_to_the_render_that_follows` (reported usage weighs one `color` evaluation), `a_check_that_holds_is_reported_as_a_call_of_its_own`; `a_root_change_makes_kept_results_that_read_it_stale` still passes with 1.5 weights. Spec: kept-results requirement (sentence and scenario), the tree-export requirement's budget sentence, design Decision 11 Cost, `docs/nx-ir-format.md`.
- **Verification:** Verified. My probe now reports 6,040 operations for the `set_environment` that re-evaluates `color`, against 6,037 for a fresh visit, so the check's work is counted. The check and the call that follows share one budget through `take_on`. A failed check reports what it spent. A check that holds with no call after it is reported on its own. The empty-record and verified-at-this-revision shortcuts report nothing, so `keeping_a_child_leaves_the_usage_report_as_the_last_call_wrote_it` still holds. The rebinding and prop comparisons through `metered` still run under a budget of their own and report nothing, but that was already so before this change.

### ✅ Verified - RF27 Environment reads made by prop defaults are not in the node's record, so a node whose prop default reads `env` keeps a stale render
- **Severity:** Medium
- **Confidence:** High (confirmed; the same on `origin/main`)
- **Location:** `crates/nx-ir-runtime/src/component.rs` `initialize` (`normalize_props` at line 1329, which runs before `begin_render` pushes the render's track) and `normalize_component_props` (line 1423, called without a session); `tree.rs` `keep_node` (equal descriptor path)
- **Evidence:** Props are now sources compared after their defaults resolve. But a default's environment reads happen before the render's track exists, so `end_read` finds no parent and drops them. Failure scenario: `component <Shown extends Node label:string = { env.mode } /> = { <Label Text={label} /> }` is visited at the root. After `set_environment(mode = "dark")`, `environment_value` at the node gives `dark`, but the node still shows `light`, because nothing indexes it under `mode`. A visit whose only change is a new `$env` on the parent behaves the same way, since the equal-descriptor path keeps the render. This happened before the change too, but it contradicts the new `reactivity` requirement that a computation records the sources it read "at any depth", and the decision to compare props after defaults resolve.
- **Recommendation:** Start the render's track before props are normalized, or record the defaults' reads separately and add them to the node's record and index. Add a tree test.
- **Fix:** The render's track now starts before the props are normalized (`begin_render_track`), so what prop defaults read is the render's, first in its record; the state's defaults, evaluated only at creation, are evaluated with the track suspended. `normalize_component_props` runs tracked and returns what the defaults read (`NormalizedProps::record`), which seeds the record of an initialization from those props; a prop skip that takes new props joins it to the node's record and relinks the indexes. The node keeps its props' inputs apart (`Node::props_record`) and a dispatch, which renders from the instance's props, starts its record with them. Note: a parent's render fills in the defaults of the descriptors it writes, so in practice this matters for descriptors a host writes without the prop; the reviewer's probe saw the stale child because it ran no pass after the Page rendered again. Tests: `a_prop_default_that_reads_the_environment_follows_a_root_change` (also after a dispatch), `a_prop_default_that_reads_the_environment_follows_a_new_place`, `a_prop_default_a_new_descriptor_falls_back_on_joins_the_record`. Spec: reactivity records requirement (sentence and scenario), design Decision 10.
- **Verification:** Verified. The fix note is right that my probe was flawed. The parent fills in prop defaults when it writes `<Shown />`, so the Page renders again, and after a pass the child shows `dark`. The tracking added for host-written descriptors, for a new place and for a dispatch's seed is correct. The prop skip's joined record keeps the old record's prop inputs as well. That can only add renders, never miss one.

### ✅ Verified - RF28 Every tree operation clones the whole kept-result store and sweeps every variant, even on the free path, so a pass costs O(nodes × variants) in real time
- **Severity:** Low
- **Confidence:** High (code reading; not measured)
- **Location:** `crates/nx-ir-runtime/src/tree.rs` `session` (line 249: `Session::new(self.store.clone())`), called in `visit` before the same-descriptor check, in `initialize_node`, once per pull candidate and once per dispatched node; `Store::sweep` at the end of every `visit`, `initialize_node`, `dispatch_chain` and `finish`
- **Evidence:** `Store` holds `HashMap`s of variants, `by_derived`, `users` (with `BTreeSet`s) and `readers`. A visit that changes nothing still clones all of them and then scans every variant in `sweep`. With N nodes and V kept variants, a pass that renders nothing costs O(N·V). Operation counts don't show it, but hosts that pass large trees will.
- **Recommendation:** Clone the store lazily, or copy on write, only when a check needs to evaluate or keep something. Skip the session on the `through_same && equal` path once the record is known to hold. Sweep once per operation, starting from the variants that lost users rather than all of them.
- **Fix:** A `Session` now borrows the tree's store and copies it only at its first write other than a variant found valid (`Session::write`); variants found valid are held in an overlay of revisions that `Store::commit` applies in place. So a check that only validates, and every visit on the free path, copies nothing. `Store` keeps `orphans`, the variants kept or left without a user since the last sweep (`unlink` and `keep` add them), and `sweep` starts from those only. A render that keeps a new result still copies the store once; `change_roots` and `atomically` remain O(variants) per `set_environment` or dispatch, as `atomically` is already O(nodes). Not measured. Spec: design Decisions 11 (All or nothing) and 14 (Dropping variants).
- **Verification:** Verified by reading the code. `Session` borrows the store, and the free path never calls `write`. `commit` applies the overlay of revisions found valid. No path mutates `self.store` between a session's creation and its commit, so replacing the store with the session's copy loses nothing. `sweep` starts from `orphans`, which `keep` and `unlink` fill, and `unlink` of a swept variant cascades. Not measured.

### ✅ Resolved - RF29 The prop skip compares props without origins, while state fields and the cutoff compare by written identity, so a kept render can carry the origins of an old prop
- **Severity:** Low
- **Confidence:** Medium (code reading; not reproduced)
- **Location:** `crates/nx-ir-runtime/src/tree.rs` `Props::values` (around line 1586)
- **Evidence:** `Props::values` compares floats by bits and handlers through rebinding, but it never looks at record origins. `read_field_changed` and the equality cutoff use `same_written`, which compares origins when the call collects them. Failure scenario, with origins collected: a parent builds a `style` record prop at one node in one render and at another node in the next. The fields are equal, so the child keeps its render. Its output passes the record through, so it still reports the first node's origin, while a fresh render would report the second.
- **Recommendation:** Compare recorded props with the same written-identity rule, including origins when they are collected, or say in the spec that origins do not count for props.
- **Status:** Not a bug. A node's props come from a descriptor, an `NxValue`, which `from_host` converts to records with no origin, so a prop never carries an origin that could differ; a default record built by the child is built at the same node each time. A kept render also makes no lifecycle call, so it reports no origins, and the instance takes the new normalized props (`with_props`). No change.
- **Verification:** Agreed. Descriptors are `NxValue`s, `from_host` builds records without origins, and a default the child builds comes from the same node each time, so no prop's origin can differ.

## Questions
- RF26: should the check's work be part of the render's count and budget, as the spec and docs say, or is a separate budget intended? If separate, the cost rules need to say so.
  - Answered by the fix: it is part of the count and budget of the lifecycle call that follows it, and a check with no call after it is reported as a call of its own.

## Summary
- Eight new findings: three High (RF22–RF24), three Medium (RF25–RF27) and two Low (RF28, RF29). RF22, RF23 and RF24 give stale or wrong output that a fresh evaluation would not give, and RF23 and RF24 are regressions from `origin/main`. Within-call reuse in the Rust runtime, the TypeScript runtime and generated JS showed no new issue: the token rule is applied alike in all three, failures are never kept, and `atomically` snapshots every new piece of tree state.
- Verification (2026-11-24 23:10): RF22–RF28 are verified and RF29's resolution is accepted. All the original probes now pass, as do the `nx-ir-runtime` and `nx-api` test suites. One new High finding, RF30, is open: an override-supplied read inside a source evaluated under the render's own overrides is still checked by value at the place.

## New Findings Discovered During 2026-11-24 23:10 Verification

### 🔴 Open - RF30 An override-supplied read inside a source evaluated under the render's own overrides records the value it found there, and validation compares it with the lookup at the place, without those overrides
- **Severity:** High
- **Confidence:** High (confirmed with a throwaway tree test; not checked against `origin/main`)
- **Location:** `crates/nx-ir-runtime/src/reuse.rs` `end_read`, the `info.host` arm: an `Input::Override` is recorded whenever the parent is not the render track, and the derived track's inputs are then forwarded through `Tracked::inputs` to the render
- **Evidence:** After the RF23/RF24 fix, a source that is not kept because it read under the render's own overrides gives its direct inputs to the render. When such a source reads a value that a layer of the place sets, the read is recorded as `Override { value }`, where `value` is what the lookup found *with the render's own layers on the stack*. `Input::Beneath` covers this case only when the read is made directly by the render body. Validation compares the recorded value with a lookup at the place, *without* the body's overrides, so the two can agree while what the render would now read differs. Failure scenario: `environment loud: string = { env.mode + "!" + env.locale }`, `<Card> = { provide env.mode="dark" { <Label Text={env.loud} /> } }`, and the Card sits under `<Stack env.locale={ if env.mode == "dark" { env.region } else { "fixed" } }>`. The first render shows `dark!fixed` (with `region = "fixed"`). After `set_environment(region = "us")` and a pass, the node still shows `dark!fixed`. The `region` index reaches the node, but the lookup at the place, where `mode` is `light`, still gives `"fixed"`, equal to the recorded value. A fresh tree gives `dark!us`. The same body reading `env.locale` directly is handled correctly.
- **Recommendation:** Treat a host-layer read whose own evaluation met a layer the call pushed (`nested.is_some()`, or a read inside a derived evaluation that has `passed`) as not checkable by value. For example, record `Beneath { value: None }` semantics so that it always counts as changed, or forward the formula's own nested inputs instead of a value. Add the scenario as a tree test.
