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

### 🟡 Fixed - RF17 The edge rule treats an invalid used result as a changed input, so the equality cutoff never takes effect
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

### 🟡 Fixed - RF18 Taking a kept result into a render needs its transitive reads, but the record holds only direct inputs
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

### 🟡 Fixed - RF19 The scenario "A kept render under a new parent instance is rebound" cannot hold while handlers capture the whole frame
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

### 🟡 Fixed - RF20 A prop skip changes the part of an instance that `is` compares, so the children's `through` stops matching again
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

