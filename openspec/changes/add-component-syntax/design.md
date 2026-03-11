## Context

NX currently represents reusable element-shaped declarations with `let <.../> = ...` under `function_definition`. That path works for props and a direct body expression, but it has no place to attach component-specific metadata like emitted actions or persistent local state. The current grammar also only allows a direct right-hand-side expression after `=`, so it cannot represent a body prologue such as `state { ... }` followed by a render expression.

This change is intentionally parser-scoped. The grammar needs to accept and preserve the new structure now, while HIR lowering, type construction for emitted actions, and interpreter/runtime state remain for later changes.

## Goals / Non-Goals

**Goals:**
- Add a dedicated top-level `component` declaration syntax.
- Parse an optional `emits` group in the component signature.
- Parse an optional `state` group in the component body.
- Preserve enough CST structure for later lowering to synthesize public emitted-action record types and private component state types.
- Update parser-facing tests, highlighting, and syntax documentation to cover the new grammar.

**Non-Goals:**
- Lower component declarations into HIR or runtime structures.
- Enforce visibility rules such as `SearchBox.state` being private.
- Synthesize `SearchBox.ValueChanged` or `SearchBox.state` record types.
- Add interpreter behavior for state persistence or event emission.
- Replace or migrate existing `let`-based functions/components.

## Decisions

### 1. Introduce a distinct `component_definition` grammar node

Add `component_definition` to the top-level module definition rather than extending `function_definition`.

Proposed shape:

```text
component_definition
  : 'component' component_signature '=' component_body
```

This keeps `let` semantics unchanged and gives downstream phases a reliable way to distinguish components from ordinary functions.

Alternative considered: treat `component` as another branch of `function_definition`.
Rejected because the `emits` and `state` substructures are not function-like, and a separate node will make later lowering and diagnostics simpler.

### 2. Keep the component signature element-shaped and add an optional `emits` clause

The signature should mirror the current element-style function form so component invocation still aligns with declaration syntax.

Proposed shape:

```text
component_signature
  : '<' element_name property_definition* emits_group? '/' '>'

emits_group
  : 'emits' '{' emit_definition* '}'

emit_definition
  : identifier '{' property_definition* '}'
```

Reusing `property_definition` keeps payload fields and props consistent with the rest of the language.

Alternative considered: model emitted actions as normal nested `type` declarations.
Rejected because the source syntax is inline to the component signature, and representing it directly avoids inventing parser sugar that later phases must reverse.

### 3. Require a block body for components

Components should use a dedicated body block so an optional state prologue has an unambiguous home.

Proposed shape:

```text
component_body
  : '{' state_group? value_expression '}'

state_group
  : 'state' '{' property_definition* '}'
```

Using `value_expression` for the trailing body keeps existing expression forms available, including direct elements and `if`/`for` expressions.

Alternative considered: allow `component ... = <Element />` and treat `state` as another expression form.
Rejected because it makes the presence and scope of `state` ambiguous and complicates both parsing and future lowering.

### 4. Add dedicated syntax kinds and keyword coverage

The grammar change should introduce explicit CST nodes and keyword tokens for:
- `component_definition`
- `component_signature`
- `component_body`
- `emits_group`
- `emit_definition`
- `state_group`
- `component`
- `emits`
- `state`

That requires regenerating tree-sitter outputs, updating `syntax_kind.rs`, and extending highlight queries so the new keywords and declaration names are classified correctly.

Alternative considered: rely on anonymous punctuation/keyword nodes where possible.
Rejected because later lowering needs stable named nodes for emitted-action and state sections.

### 5. Limit the surface area to parser-owned artifacts

This change should update only artifacts owned by parsing and syntax presentation:
- `crates/nx-syntax/grammar.js` and generated parser outputs
- `crates/nx-syntax/src/syntax_kind.rs` and node metadata
- `crates/nx-syntax/queries/highlights.scm`
- parser tests and fixtures
- grammar/reference docs that describe declaration syntax

Lowering and interpreter crates are intentionally left untouched in this change.

## Risks / Trade-offs

- Block-only component bodies diverge from existing `let` bodies -> document the distinction clearly in syntax docs and examples.
- New keywords may conflict with existing identifiers named `component`, `emits`, or `state` -> accept the reservation now and cover it in release notes or follow-up migration notes if needed.
- Separate CST nodes without lowering support could make the feature feel incomplete -> keep proposal, design, and task scope explicit that this is grammar-only groundwork.
- Reusing `property_definition` inside `emits` and `state` ties those sections to current field syntax -> acceptable because the example syntax already matches record-style fields and future lowering benefits from that consistency.
