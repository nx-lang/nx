# Plan: Improve NX Parser Error Messages with "Expected" Tokens

Enhance the NX CLI error reporting to show what tokens were expected when a syntax error occurs, transforming generic "Syntax error" messages into specific "expected '>', found '1'" messages using Tree-sitter's `LookaheadIterator` API.

**Status:** Implemented

## Background

Currently, parse errors produce generic messages like:

```
error hello.nx:2:1: Syntax error
    2 | <div: class="greeting" 1 2>
      | ^^^^^^^^^^^^^^^^^^^^^^^^^^ unexpected syntax here
note: Check the syntax and try again
```

This doesn't tell users **what** was expected, just that something was wrong.

## Goal

Transform error messages to be specific:

```
error hello.nx:2:22: Unexpected token '1'
    2 | <div: class="greeting" 1 2>
      |                        ^ expected '>', '/>', or '}'
```

## Steps

### 1. Thread `Language` into error collection ✅

**File:** `crates/nx-syntax/src/validation.rs`

Pass the Tree-sitter `Language` object into the error walker so we can use `LookaheadIterator` for expected tokens.

```rust
pub fn collect_enhanced_errors(
    tree: &tree_sitter::Tree,
    source: &str,
    file_name: &str,
    language: &tree_sitter::Language, // NEW
) -> Vec<Diagnostic>
```

### 2. Implement `expected_symbols` with safe parse-state lookup ✅

**File:** `crates/nx-syntax/src/validation.rs`

Add a helper that gathers likely parse states (parent, node, previous sibling, first child, previous leaf) and pulls expected tokens via `Language::lookahead_iterator`. Filter out internal symbols (`_` prefix), dedupe, and cap at 5 items; append `"..."` if truncated. Return the first non-empty lookahead set.

```rust
fn expected_symbols(node: &tree_sitter::Node, language: &tree_sitter::Language) -> Vec<String> {
    let mut candidate_states: Vec<u16> = Vec::new();
    // add parent/node/sibling/child/previous-leaf states, deduped
    for state in candidate_states {
        if let Some(mut lookahead) = language.lookahead_iterator(state) {
            let mut symbols: Vec<String> = lookahead
                .iter_names()
                .filter(|name| !name.starts_with('_'))
                .map(|s| s.to_string())
                .collect();

            symbols.sort();
            symbols.dedup();

            if symbols.len() > 5 {
                symbols.truncate(5);
                symbols.push("...".into());
            }

            if !symbols.is_empty() {
                return symbols;
            }
        }
    }
    Vec::new()
}
```

### 3. Human-friendly symbol formatting ✅

**File:** `crates/nx-syntax/src/validation.rs`

Convert grammar symbol names into readable text (quote punctuation, expand common names like `identifier`, `string`, `number`, and treat empty/EOF as `"end of file"`). Add `format_expected` to join the list with commas and `or`.

### 4. Rework `walk_and_collect_errors` messaging ✅

**File:** `crates/nx-syntax/src/validation.rs`

Use the expected-symbol helper when building diagnostics:

- For `MISSING` nodes, prefer `"Expected <symbol>"` and keep the existing suggestion note.
- For `ERROR` nodes, extract a concise `found` token from the source slice (trim whitespace; strip delimiters; fall back to `node.kind()` if empty). If expected symbols exist, emit `"Unexpected '<found>': expected <list>"`. If not, fall back to `analyze_error_context` for message/note.
- Add lightweight heuristics to ensure obvious closing delimiters (`>`, `/>`, `}`, `)`, `]`) appear in expectations when the slice suggests an unclosed opener.
- Preserve the existing diagnostic code and label ranges.

### 5. Update `lib.rs` to pass `Language` ✅

**File:** `crates/nx-syntax/src/lib.rs`

Modify `parse_str` to pass the language when calling `collect_enhanced_errors`.

```rust
// In parse_str function:
let mut errors = validation::collect_enhanced_errors(&tree, source, file_name, &language());
```

### 6. Add targeted tests for expected-token messaging ✅

**File:** `crates/nx-syntax/src/validation.rs`

Add tests that assert:
- Unexpected literal in an element reports the literal and expected symbols (colon element syntax).
- Unclosed start tag reports expected `>` or `/>`.
- Missing closing delimiter still uses the existing suggestion note but now includes the expected symbol in the main message.

```rust
#[test]
fn test_error_shows_expected_tokens() {
    let source = "<div: class=\"test\" 1>";  // '1' is unexpected
    let result = parse_str(source, "test.nx");
    
    assert!(!result.errors.is_empty());
    let msg = result.errors[0].message();
    assert!(msg.contains("expected"), "Should mention what was expected: {}", msg);
    assert!(msg.contains("1"), "Should mention the unexpected token: {}", msg);
}

#[test]
fn test_unclosed_element_shows_expected_close() {
    let source = "<div:";
    let result = parse_str(source, "test.nx");
    
    assert!(!result.errors.is_empty());
    let msg = result.errors[0].message();
    assert!(msg.contains(">") || msg.contains("/>"), 
        "Should expect closing bracket: {}", msg);
}
```

## Further Considerations

### Compatibility

- Upgraded to `tree-sitter = "0.22"` to use `Language::lookahead_iterator` and parse-state APIs directly.

### Output Length

- Keep the first five expected symbols; if more exist, append `"..."` to indicate truncation.

---

## Review Findings

### Issues to Fix

#### 1. Snapshot test failure (HIGH) ✅ Done

The file `crates/nx-syntax/tests/snapshots/parser_tests__snapshot_error_diagnostics.snap` needs updating to match the new error message format.

**Fix:** Snapshot updated to current output (no `.snap.new` pending).

#### 2. Heuristics completely replace grammar-based results (MEDIUM) ✅ Done

In `walk_and_collect_errors` (lines 244-247):

```rust
let mut expected = expected_symbols(&node, language);
let heuristic_expected = heuristic_expected_tokens(error_text);

if !heuristic_expected.is_empty() {
    expected = heuristic_expected;   // ← COMPLETELY REPLACES grammar results
}
```

The heuristics (`heuristic_expected_tokens`) and grammar-based lookahead (`expected_symbols`) return **different things**:

| Source | Example output for `<div...>` |
|--------|-------------------------------|
| Grammar lookahead | `['!=', '&&', '(', ')', '*', ...]` (operators from confused parser state) |
| Heuristics | `['>', '/>']` (closing brackets based on text patterns) |

Currently heuristics **replace** grammar results entirely when non-empty. This is arguably correct behavior since heuristics often produce more intuitive suggestions when the parser's error recovery leads to confusing states. However, this design choice should be documented.

**Fix:** Heuristic expectations are now merged ahead of grammar results (not replacing them) so intuitive closers appear while preserving parser hints.

#### 3. `"..."` truncation indicator not humanized (LOW) ✅ Done

When symbols are truncated, `"..."` is added to the list but `humanize_symbol_name("...")` just returns `"..."`. This produces:
```
expected '!=', '&&', '(', ')', '*', or '...'
```

**Fix:** `"..."` is humanized to `more`.

#### 4. Label message incorrect for MISSING nodes (MEDIUM) ✅ Done

For MISSING nodes where the message says "Expected X", the label still says `"unexpected syntax here"` which is semantically incorrect.

**Fix:** Missing-node labels now use `"missing here"`.

#### 5. `extract_found_token` extracts wrong token for elements (LOW) ✅ Done

For `<div class="greeting">`, it extracts `/div` (from closing tag in error span) rather than the actual problematic token. The heuristic of taking the last whitespace-separated part doesn't work well for element syntax.

**Fix:** `extract_found_token` now prefers the last non-identifier token (e.g., the literal `1`) to avoid tag-name captures.

### Additional Improvements

#### Humanize more NX-specific symbols

**Fix:** Added NX-specific humanization for `element`, `property`, and `text_content`.

#### Consider deduplicating ERROR children

If an ERROR node has error children, we might report redundant errors for the same syntax issue.

**Fix:** Error collection now skips ERROR/MISSING nodes that have an ERROR ancestor. The rationale:

1. **Parent errors provide better context** - The parent ERROR node spans the entire problematic region, and `find_error_position` locates the specific error within that span.

2. **Child errors are often redundant** - When tree-sitter creates nested ERROR nodes, they typically represent the same underlying syntax problem. Reporting both produces duplicate messages at the same location.

3. **Sibling errors are preserved** - Independent ERROR nodes (siblings) are still all reported since they represent distinct syntax problems.

4. **Children are still traversed** - We recurse into children of skipped ERROR nodes to find any genuinely distinct nested errors that might exist deeper in the tree.

Example improvement:
```
Before (duplicate errors for { ??? }):
error test.nx:2:5: Unexpected '???': expected '<', ...
error test.nx:2:5: Unexpected '???': expected '!=', ...

After (single error):
error test.nx:2:5: Unexpected '???': expected '<', ...
```

### Line Number Accuracy ✅ Done

Tree-sitter's error recovery can capture large spans (e.g., `<main>\n</main>` as a single ERROR node starting at `<main>`). Without correction, errors would be reported at the start of the recovery span rather than the actual problem location.

**Fix:** Added `find_error_position()` function that:

1. For MISSING nodes, uses their position directly
2. For ERROR nodes with ERROR/MISSING children, reports from that child's position  
3. For ERROR nodes where all children are "valid" tokens, looks for newline boundaries between children and reports from the first child after a newline (where the error likely starts)
4. Falls back to the original span if no better position is found

Example improvement:
```
Before (wrong line):
error test.nx:4:1: Unexpected '/main': ...
    4 | <main>
      | ^^^^^^ unexpected syntax here

After (correct line):
error test.nx:5:1: Unexpected '/main': ...
    5 | </main>
      | ^^^^^^^ unexpected syntax here
```
