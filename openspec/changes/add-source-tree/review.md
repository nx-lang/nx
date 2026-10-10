# Review: add-source-tree

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/source-tree/spec.md, specs/language-protocol/spec.md, tasks.md  
**Reviewed code:** the committed diff `133d2cf..HEAD`, limited to this change: `crates/nx-language-service/src/{source_tree.rs,source_tree_tests.rs,lib.rs}`, `bindings/wasm/**`, `bindings/node/**`, `packages/language-{protocol,core,http,client}/**`, `packages/monaco/test/**`, `sites/playground/src/{language/worker.ts,worker/session.ts}`, `src/vscode/CHANGELOG.md`. The value-origin files were out of scope.  

**Checks run:**
- `cargo test -p nx-language-service --lib source_tree`: 34 passed. The measurement test reports 1509 nodes and 380 KB of JSON for question-flow `main.nx`, in 59.6 ms warm in a debug build, so the 100 ms release budget is met.
- `openspec validate add-source-tree --strict`: valid.
- `language-protocol` (4), `language-core` (16), `language-http` (20), `language-client` (16) and `monaco` (18): all passed.
- `sdk-node` vitest: 37 passed.
- `sdk-wasm` `language-service.test.ts` and `parity.test.ts`: 39 passed.
- Scratch probe outside the repo: it calls `WorkspaceSnapshot::source_tree` on hand-written and fuzzed documents and runs the tests' own `violations()` check against each tree. The fuzzed documents are prefixes of a real file, and real files with 3 bytes deleted, taken from question-flow and `examples/nx/*.nx`.

## Findings

### ✅ Verified - RF1 A member modifier other than `content` is dropped, so any identifier the parser reads as a modifier breaks the token rule
- **Severity:** Medium
- **Evidence:**
  - The grammar reads any identifier before a member's name as `modifier` (`crates/nx-syntax/grammar.js:428`, `:465`). Lowering accepts only `content` there and reports "Unsupported property modifier" for anything else (`crates/nx-hir/src/lower.rs:942-946`).
  - `member()` looks at the modifier only to test for `content` (`crates/nx-language-service/src/source_tree.rs:661-666`). Any other modifier token gets no node, and the member does not carry it.
  - Example: `action Completed = { respondentring answered:int }` gives a `field` node `Completed.answered` that directly owns `respondentring`. The tests' checker reports "`respondentring` belongs to Field `Completed.answered`, which does not say what it is".
  - This shape is very common mid-edit. Deleting 3 bytes at points in `specs/ir-conformance/question-flow/library/questions.nx` produced about ten distinct violations. Examples: `exts` in `Phone.defaultCountry`, `ends` in `Number.min`, and `Question` absorbed into `Consenxtends.statement` after a broken `extends`.
  - The repository conformance test only sees files that parse and compile, so it cannot catch this.
- **Recommendation:**
  - When the modifier is not `content`, emit an `unparsed` node for it (the analysis already reports it as an error), or carry it on the member.
  - Add a role test for a misspelled modifier.
  - Consider extending the coverage test with a small deterministic edit corpus, such as truncations and short deletions of the question-flow program, so the token rule is checked on documents that do not parse. The spec says such documents still have a lossless tree.
- **Fix:** `member()` now emits an `unparsed` child for a modifier other than `content` (lowering already reports it), and keeps the `content` flag for `content`. Covered by the new `a_stray_modifier_is_unparsed` test (`action Completed = { respondentring answered:int }`). The new `every_token_of_an_edited_nx_file_belongs_to_a_node` test checks the token rule on 178 deterministic edits of `question-flow/library/questions.nx`, `examples/nx/component.nx` and `examples/nx/types.nx`: every eleventh token deleted, and a cut before every forty-first. Against the builder as committed, it reports this finding's violations and RF3's. Scratch runs that deleted every token, and 3-byte windows, of every file under `examples/nx`, the playground examples and question-flow found two more holes of the same kind, which are fixed too:
  - An occurrence suffix on a case (`mode=dark?`, or `= answers?` mid-edit) was owned by the `case` node. It is now an `unparsed` child, as in the new `a_suffix_on_a_case_is_unparsed` test. The checker rejects the suffix there, and `T=int?` still reads as a type.
  - An element tag spelled with whitespace (`<SearchBox. value=…>` mid-edit) was rejected by the tightened checker. The checker now splits a qualified name at whitespace as well as at dots.
  - The scratch runs (about 50,000 edits, the syntax fixtures included) now report no violation. The committed edit test takes about 5 s in a debug build.
- **Verification:** Verified. `member()` now emits an `unparsed` child for a modifier other than `content` (`source_tree.rs:672-678`). `a_stray_modifier_is_unparsed` passes. My `respondentring` reproduction and the `questions.nx` deletions that failed before are now clean. I reran my scratch fuzzer over all 53 `.nx` files under `examples/nx`, question-flow and `sites`, using the fixed checker: a prefix cut and a 3-byte deletion at every 7th byte. It found no violations. The new edited-file test passes.

### ✅ Verified - RF2 A region that did not parse is reported as the empty value `{}`, and inside a pattern it breaks source order and sibling overlap
- **Severity:** Low
- **Evidence:**
  - `braced()` counts only `Part::Item`s (`source_tree.rs:837-846`). A brace pair that holds only an error region becomes an `empty` node with an `unparsed` child. For example, `let f(k:M) = { if k is { @ => 1 } }` answers `f.body` as `empty`, even though the source is not `{}`.
  - `pattern()` places trivia under the arm while it walks the pattern's children. It then pushes an `empty` fallback node that spans the whole pattern (`source_tree.rs:1098-1113`), so the `unparsed` sibling comes first and lies inside its later sibling.
  - Example: `<Panel if k is { light => { title="a" } } />` gives node 29, `unparsed` [226..236], followed by node 30, `empty` [224..237], both with the arm as parent. The tests' checker reports "is out of order" and "overlaps its sibling". This contradicts the requirement that nodes are in start order and that two siblings' ranges do not overlap.
- **Recommendation:**
  - In `pattern()`, push the fallback node before walking the children, or do not push one when the pattern's only content is an error region, so the `unparsed` node takes its place.
  - In `braced()`, do the same when there are no items but there is an error region: emit `unparsed` alone, or a sequence or wrapper role, rather than `empty`.
  - Cover both cases with `tree_for` tests, which already run the invariant checks.
- **Fix:** In `braced()`, braces that hold no item but do hold an error region now act like `{x}`: the `unparsed` node takes the value's place under the parent, which owns the braces, so no `empty` node is produced. `pattern()` decides before walking whether the pattern is `{}`. If it is, it pushes the `empty` node first and places any comments inside it under that node. A pattern that is only an error region is that `unparsed` node, with no fallback. New `tree_for` tests `braces_around_an_unparsed_region_are_not_empty` and `a_pattern_that_did_not_parse_is_unparsed` cover both examples and a `{ /* none */ }` pattern. `tree_for` checks order and sibling overlap.
- **Verification:** Verified. `braced()` no longer produces `empty` for braces that hold only an error region. `pattern()` pushes the `empty` node before its comments, and does not push one when the pattern is only an error region. My two reproductions (`if k is { light => { title="a" } }` and `{ if k is { @ => 1 } }`) now produce no order, overlap or token violations. The fix opened a narrow new hole, reported as RF9: the braces now belong to the parent, and a member access does not own braces.

### ✅ Verified - RF3 Tokens the parser invented to recover become zero-width nodes with empty names
- **Severity:** Low
- **Evidence:**
  - `trivia()` skips a missing node only when the missing node is the child being placed (`source_tree.rs:1415-1417`). `expression()` handles an invented `?` (`:765-772`), but nothing else.
  - A missing identifier inside an `IDENTIFIER_EXPRESSION`, a `CONTEXTUAL_NAME` or a type becomes a real node:
    - `let b = <Card title= />` gives `case` `b.value.title.value` with `name: ""` at [30..30].
    - `let c = { 1 + }` gives `reference` `c.value.right` with `name: ""` at [47..47].
    - Truncating `type UserId = ` gives a zero-width `typeReference`.
  - Fuzzing five `examples/nx` files and question-flow produced zero-width `case` and `typeReference` nodes in each.
  - A renderer would show an empty reference or case for text that is not in the source.
- **Recommendation:** Skip a construct whose span is empty, or whose only leaf is missing, in `expression()`, `type_reference()` and `member()`. Add a test with an incomplete attribute value and an incomplete binary operand.
- **Fix:** No node is pushed for an empty span in `expression()`, `type_reference()` and `qualified_case()`, or for an invented import path, imported name, emitted action name or element body (`<Note:markdown></Note>` bound to a content property). An invented name leaves the node's `name` absent rather than `""`, through a new `written()` helper. This covers a member access (`x.`), an attribute (`={1}`), a union case (`type M = a | `), a declaration and a member. `violations()` now also reports a node that covers no source, so `tree_for`, the repository test and the edit test all enforce this. New test `what_the_parser_invented_has_no_node` covers the three examples above and the other shapes the scratch runs found.
- **Verification:** Verified. Empty-span constructs get no node, and invented names are absent rather than `""` (the `written()` helper). `violations()` now rejects zero-width nodes, so every test enforces this. My reproductions (`title= `, `{ 1 + }`, `type UserId = `) and the fuzz run produce no zero-width nodes. Extra probes also came back clean: `< title="x" />`, `for in xs`, `for x, in xs`, `let = 1`, `let f( =`, an empty `@{ }`, `f(,)`, `1 ..` and an empty `state { }`.

### ✅ Verified - RF4 The standard-library declaration test is vacuous: its source does not parse and it asserts nothing about the library
- **Severity:** Low
- **Evidence:**
  - `a_standard_library_declaration_is_described` (`source_tree_tests.rs:1019-1028`) uses `let a = { [1, 2].count }`, which is not NX.
  - The tree it produces is `declaration`, then `empty`, then `unparsed`. The table holds only `a`, so the loop over non-local entries never runs.
  - Task 1.3 and the spec's "the standard library included" are therefore not exercised by any test.
  - A probe with `import "@nx/agent"` and `let a = <Agent name="support">Be brief.</Agent>` shows the behavior itself works. It answers an `Agent` entry in module `@nx/agent/agent.nx` with no range, with its doc, and with `instructions` marked `content`.
- **Recommendation:** Replace the source with a standard-library import, as in the probe above or the existing `agent_hover` tests. Assert the entry's module, kind, absent range and its properties and flags.
- **Fix:** The test now uses `import "@nx/agent"`, an `<Agent>` with content and a `<FunctionTool>`; the source has no diagnostics. It asserts:
  - `Agent`'s module `@nx/agent/agent.nx`, its kind `record`, its absent range, its doc, and its `name`, `model` (optional) and `instructions` (content, documented) properties
  - that the element's content binds to an `instructions` attribute
  - that `FunctionTool`'s base `Tool` is a library entry with no range, and that its inherited `name` and `description` come first and are flagged `inherited`
- **Verification:** Verified. The test now imports `@nx/agent` and asserts the `Agent` entry's module, kind, absent range, doc, property types and flags. It also checks `FunctionTool`'s library base `Tool` with no range, and inherited properties listed first with the `inherited` flag. It passes, and it matches what my own probe saw.

### ✅ Verified - RF5 The coverage checker's `carries` test is loose enough to hide a forgotten construct
- **Severity:** Low
- **Evidence:** `carries()` (`source_tree_tests.rs:222-231`) has three loose checks:
  - It accepts `.` for any node with a name (`token == "."`).
  - It accepts a token when the node's value merely contains it as a substring (`value.contains(token)`).
  - It accepts any `.`-separated part of a name.

  So an operator valued `..=` passes a stray `.` or `=`, a literal `"1.5"` passes a stray `1`, and any named node passes stray dots. This is the test the spec relies on: "A forgotten construct fails the test".
- **Recommendation:**
  - Match tokens exactly. A token is carried when it equals the node's name or text type, or is one of the lexemes of the node's spelled `value`. Leaf nodes that carry their whole text (`text`, `comment`, `unparsed`, `typeReference`, `literal`) can keep containment, checked by range.
  - Allow `.` only on roles whose names are qualified (`element`, `case`, `reference`, `member`).
- **Fix:** `carries()` now accepts a token only when it equals the name, equals one of the value's whitespace-separated lexemes, or equals the text type. Only `element`, `attribute`, `reference` and `emit` accept the parts of a qualified name and the dots between them; these are the roles the grammar gives qualified names to: tags, attribute names, imported names' aliases and qualified emits. `case` and `member` already list `.` as punctuation. The leaf roles that carry their whole text (`literal`, `typeReference`, `text`, `comment`, `docComment`, `unparsed`) keep containment. The repository test still passes. Tightening the checker exposed no builder bug in the repository's files. It needed the `attribute` and `emit` roles added, and the fixtures `component-emits-qualified-reference.nx` and `import-named-aliased.nx` show why.
- **Verification:** Verified. `carries()` now uses exact matches. Only `element`, `attribute`, `reference` and `emit` accept qualified-name parts and dots, and only leaf roles that carry their whole text accept containment. The repository test, the edited-file test and all 206 crate tests pass under the stricter rule. My fuzzer uses a copy of the new checker, and it still catches an injected hole (RF9).

### ✅ Verified - RF6 The spec's token rule says a node directly owns only punctuation and keywords, but the implementation lets it own what it carries
- **Severity:** Low
- **Evidence:**
  - specs/source-tree/spec.md says: "The tokens that belong to a node and to none of its children SHALL be only the punctuation and keywords of the node's role".
  - The implementation instead has nodes own their name, value and text-type tokens directly:
    - an attribute's name
    - a declaration's or member's name
    - an operator's token
    - an import alias
    - a selective import's `as` alias
    - an element's tag and its closing name
  - The Rust doc says so (`source_tree.rs:50-52`), and the test encodes it (`carries`, `source_tree_tests.rs:305`). The spec and the code disagree on the contract a renderer is promised.
- **Recommendation:** Amend the requirement to read "...only the punctuation and keywords of the node's role, and the tokens it carries as its `name`, `value` or `textType`". Keep the clause that a name, literal, operator, type or comment always belongs to a node that says what it is.
- **Fix:** The requirement in `specs/source-tree/spec.md` now reads as recommended, and keeps the closing clause. The rule quoted in design.md's coverage decision says the same. That decision also mentions the new edit test.
- **Verification:** Verified. The token-rule requirement in `specs/source-tree/spec.md` now includes "the tokens the node carries as its `name`, `value` or `textType`". design.md matches.

### ✅ Verified - RF7 The language-protocol delta keeps the header "defines four queries" while its body defines five
- **Severity:** Low
- **Evidence:** `specs/language-protocol/spec.md` modifies "Requirement: The protocol defines four queries and names the rest". The body now lists `hover`, `completions`, `diagnostics`, `documentSymbols` and `sourceTree`, and the proposal says "the protocol defines five queries". Archiving will leave a main-spec header that contradicts its own text (`openspec/specs/language-protocol/spec.md:58`).
- **Recommendation:** Add a `## RENAMED Requirements` entry, FROM the "four queries" header TO "The protocol defines five queries and names the rest". Then apply the MODIFIED body under the new name.
- **Fix:** The delta now has a `## RENAMED Requirements` section, with `FROM:`/`TO:` lines for the header, and the MODIFIED block uses the new name. `openspec validate add-source-tree --strict` passes. A trial archive of a scratch copy of `openspec/` applied it as "1 modified, 1 renamed". It left one requirement, "The protocol defines five queries and names the rest", with the five-query body, and no "four queries" header.
- **Verification:** Verified. The delta has a `## RENAMED Requirements` FROM/TO entry and a MODIFIED block under the five-query name. `openspec validate add-source-tree --strict` passes.

### ✅ Verified - RF8 Keys embed spelled source text, so a key cannot be split back into its path segments
- **Severity:** Low
- **Evidence:**
  - Match arms are keyed `is <patterns>`, condition-list arms `when <test as spelled>`, and imports `import "<path>"` (`source_tree.rs:1016`, `:1083`, `:1376`, `:535-538`).
  - These segments contain dots, spaces, quotes and commas, giving keys such as:
    - `m.body.is Mode.dark.pattern[0]`
    - `n.body.is 1, 2.pattern[1]`
    - `t.body.is ui.Mode.light[0]`
    - `import "./extra0.nx"#2.Card`
  - A consumer such as the change view cannot recover "the arm" or "the slot" from a key without re-parsing NX. A `.` inside a segment is indistinguishable from the separator.
  - Uniqueness still holds because of the `#n` suffix, so this is a usability issue, not a correctness one.
- **Recommendation:** Pick one of the following:
  - Escape or bracket text-derived segments, for example `is[Mode.dark]`, `when[x > 1]` and `import["./ui.nx"]`.
  - Document in the protocol README and the spec that keys are opaque identifiers compared only for equality.
- **Fix:** Took the second option. Structure is already in `parent`, and a change view needs keys only to pair nodes, which equality does. Bracketing would add a second key syntax to the unstable contract for no consumer yet. The key requirement in `specs/source-tree/spec.md`, the `key` docs in `SourceNode` (Rust) and `@nx-lang/language-protocol` (TS), and the protocol README now say the same thing. A key is an identifier to compare for equality, not a path to split, because segments written from the source can hold dots and spaces. Read structure from `parent`.
- **Verification:** Verified. Keys are now documented as opaque identifiers compared for equality, with structure read from `parent`. The spec requirement, the Rust `SourceNode::key` doc, the TS `SourceNode.key` doc and the protocol README all say so. This is the second option I recommended.

## New Findings Discovered During 2026-10-10 09:26 Verification

### ✅ Verified - RF9 Braces that hold only an error region break the token rule when they are a member access's object
- **Severity:** Low
- **Evidence:**
  - The RF2 fix in `braced()` (`source_tree.rs:866-882`) passes braces with no item and an error region through like `{x}`. The `unparsed` node goes under the braced expression's parent, and that parent owns the `{` and `}`.
  - That works for parents whose role lists braces as punctuation. A member access's object is one place where it fails: `punctuation(SourceRole::Member)` is only `.` and `?.` (`source_tree_tests.rs:203`).
  - The grammar accepts a braced object syntactically but reads its contents as an error. `{b}.title` parses as `member_access_expression target: (values_braced_expression (ERROR (identifier)))`.
  - So `let c = { {b}.title }` and `let a = { { @ }.title }` produce "`{` belongs to Member `c.value`, which does not say what it is" and the same for `}`.
  - Before the fix, the old `empty` node owned those braces, so they were covered, though mislabeled. The edited-file test and my 53-file fuzz run do not reach this shape. A hand-written probe does.
- **Recommendation:** When braces with no item hold an error region and the parent role does not list braces as punctuation, keep a node that owns them. Either make the `unparsed` node cover the whole brace pair, or let the role take the braces as in the old `{x}` path. Covering the braces is simplest: in the `0 if unparsed` case, emit one `unparsed` node over `node.span()`. Add a `tree_for` test for `let c = { {b}.title }`.
- **Fix:** In `braced()`, braces that hold no item but do hold an error region are now one `unparsed` node over the whole brace pair, whatever the parent. Before, the region went under the parent and the parent owned the braces. So `{b}.title` gives an `unparsed` `{b}` under the member access, and the RF2 example gives `{ if k is { @ => 1 } }` under `f`. The `braces_around_an_unparsed_region_are_not_empty` test is updated for that. I checked whether other parents that do not list braces have the same gap. A scratch matrix put 11 wrapped fillers into 18 expression positions:
  - The fillers were `{b}`, `{}`, `{ @ }`, `{ b c }`, braces with comments, nested braces, `(b)` and `( @ )`.
  - The positions were member and optional-member objects, callee, argument, prefix and postfix operands, both binary operands, attribute value, `for` iterable, `if` test, match subject, embed, element content, member chain, range, `??` and condition-list arm, plus a parameter default.
  - No brace case remains. It found a related gap in valid NX: `(1 + 2).x` and `(Mode).dark` leave the parentheses with a `member` or `case` node, whose punctuation list had no `(`/`)`. The builder does what it does for every parenthesized operand, giving the parentheses to the parent and flagging the child `parenthesized`, as `operator` and `call` already allow. So I added `(` and `)` to the checker's `member` and `case` punctuation. That is a checker omission, not a builder change. New test `a_wrapped_object_of_a_member_access_is_covered` covers `{ {b}.title }`, `{ { @ }?.title }`, `(1 + 2).x` and `(Mode).dark`.
- **Verification:** Verified. `braced()` now emits one `unparsed` node over the whole brace pair when the braces hold no item but do hold an error region (`source_tree.rs:867`), so the braces belong to that node whatever the parent. My reproductions `let c = { {b}.title }` and `{ { @ }.title }` now give an `unparsed` `{b}` or `{ @ }` under the member access, with no violations. `a_wrapped_object_of_a_member_access_is_covered` and all 207 crate tests pass. My earlier reproductions for RF1-RF3 stay clean.

  Checker widening: letting `member` and `case` own `(` and `)` is correct.
  - It matches the convention every other role already uses. `wrapper()` passes a parenthesized object to the parent with the `parenthesized` flag, so the parentheses fall inside the member or case and outside the object.
  - `(1 + 2).x` and `(Mode).dark` are valid NX that the original checker would have flagged; no repository file used the form.
  - It cannot hide a missing construct. Every construct but the unit literal has non-parenthesis tokens that would still be flagged, and `().x` and `(()).x` both produce a `literal` `()` node, so that one is covered too.

  I also ran a 90-case matrix: 9 wrapped objects (`(c)`, `((c))`, `{c}`, `{ @ }`, `( @ )`, `(1 + 2)`, `(Mode)`, `{ (c) }`, `(xs)`) in 10 positions, covering member, `?.`, operand, prefix, attribute value, condition test, loop iterable, qualified case, case pattern and call argument. It found 0 violations. A denser fuzz sweep of all 53 `.nx` files under `examples/nx`, question-flow and `sites` (a prefix cut and a 3-byte deletion at every 5th byte) also found 0 violations.

## Questions
- `bindings/wasm/test/recursion-stack.test.ts` fails in this tree ("forLoop within the limit: expected 'trap' to be 'value'"). It concerns evaluation stack depth, not the language service, so it looks unrelated to this change. It may come from the value-origin commits or from the environment. Is it a known failure?
- The design leaves "used by" lists open. Is the expectation that the change view computes them from `declaration` indices? If so, a short note in the protocol README would help.

## Summary
- The query, its types, the protocol and the exposure through the Node SDK, wasm SDK, language-core, language-http, language-client and playground are complete. They are consistent with each other (the Rust and TS role and flag sets match) and well tested for documents that compile. All tasks are genuinely done, and the measured cost is well within budget.
- The weak spot is documents that do not fully parse, which is the normal state in an editor:
  - stray identifiers in member position break the token rule (RF1)
  - error regions are mislabeled `empty` and can break ordering (RF2)
  - invented tokens become phantom nodes (RF3)

  None of these is caught, because the coverage test only sees files that parse and its matcher is loose (RF5).
- The remaining findings are test and spec hygiene.
