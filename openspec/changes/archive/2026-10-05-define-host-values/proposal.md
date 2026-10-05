## Why

NX has one value model: the empty value, booleans, numbers, strings, sequences and records, with
the function values and action handlers a runtime makes. A host holds such a value in its own
language's data, a plain object and an array in JavaScript, an `NxValue` in Rust, and passes it as
it is, whether it read the value from JSON or computed it a moment ago. Canonical JSON is the
encoding of the same values for a wire or a store. That is how both runtimes are used, and no
spec says it as one rule: the specs call the boundary "JSON", and "a canonical value" is defined
only in passing, inside the rule for the input size.

The TypeScript runtime is where the missing rule costs something, because JavaScript can hold much
that is not a value. The runtime has no one answer to what a host value is. Its input measure
enters a list and a plain object and counts anything else as one value. Its check enters any
object where a record is expected, a class instance included, and passes anything at all through
a site typed `object`. Its result writer walks whatever it is handed. So a 5 MB typed array
measures 8 and passes every input limit, a function or a `Date` in an `object` field comes back
in a result, and a member a host set to `undefined`, which TypeScript accepts for any optional
field, is refused as a value of the wrong type. `@nx-lang/agent` grew a copying helper to paper
over the last of these for one record, took four attempts to get it right, and still cannot cover
a class instance below the top level, because it has to enter exactly what the measure enters and
the check enters more.

## What Changes

- `nx-ir-format` says what a host value is: a canonical value, in a form each runtime defines in
  its host's language and takes with no encoding step. Canonical JSON is one encoding of it and
  not something a host has to produce. A form may spell a value in a way JSON does not, where the
  language makes that natural. What is not a canonical value is refused where it is passed and
  never converted by guessing. `docs/nx-ir-format.md` gains the table of forms: each kind of
  value as canonical JSON, JavaScript and Rust hold it.
- The TypeScript runtime reads its input once, when a call is entered, in its JavaScript form:
  - A value the host built is the value. Nothing is encoded, and nothing is copied unless a
    member has to be left out.
  - A member of a plain object that is `undefined` is absent. It is not counted by the input
    measure, a default applies or an optional field is empty where the check would have seen it,
    and the program is not given it.
  - **BREAKING**: a value that is not canonical is refused, with `nx-ir-boundary-type` and the
    path, wherever in the input it is: a class instance, a typed array, a `Date`, a `Map`, a
    function, a symbol, a big integer. That includes a value inside a site typed `object`, which
    nothing looks at today, and a plain object that holds itself. The handler and function values
    the runtime makes for its own use are not refused.
  - A refusal inside an argument of `callFunction` or `evaluateFunction` names the argument, as
    every other failure in a value a host passed does.
- The input size of `nx-ir-format` gains one rule (an `undefined` member is not a member) and its
  sentence about a value that is not canonical says what now happens to one.
- `@nx-lang/agent` drops its copy of the context record. It passes the record's own members with
  `callId` set, as it did at first, and the runtime does the rest. **BREAKING** for a host that
  passes a class instance below the top level of its context: it is refused where it was entered,
  and reported as `invalid-context`.
- The Rust runtime does not change: `NxValue` is its form, and it can hold nothing else.

No operation count changes, no NX IR format change, no compiler change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `nx-ir-format`: what a host value is, as a new requirement; and the requirement *A call's host
  input has a size*, for an `undefined` member and for what a runtime does with a value that is
  not canonical.
- `typescript-ir-runtime`: the runtime's JavaScript form and how it reads it, as a new
  requirement.
- `agent-host-package`: the paragraph about the members of the context record in *A function tool
  runs its function under the evaluation budget*, and one scenario. The delta is a copy of the
  requirement as the main spec has it today, after `name-the-argument-in-boundary-diagnostics`,
  with those two edits.

## Impact

- `runtime/typescript/src/index.ts`: a reading of the input at the entry of every evaluation API,
  beside the measure, and the input measure itself (`measureValue`); the check is not changed.
  With the committed `dist`, and `runtime/typescript/README.md`.
- `docs/nx-ir-format.md`: *Values*, for the table of forms, and *Input size*.
- `crates/nx-ir-runtime/README.md` and the doc comment of `NxValue`: a sentence each, naming
  `NxValue` as the Rust form. No code.
- `packages/agent/src/execute.ts` and `src/json.ts` (`withoutUndefinedMembers` is removed), their
  tests, and `README.md`.
- `specs/future.md`: the entry *A host value that is not JSON is measured as one value, then
  walked* is resolved by this change and removed.
- `src/vscode/CHANGELOG.md`.
- Hosts of the TypeScript runtime that pass something other than plain data as props, state,
  arguments or a batch: the playground, the DrawnUI fiddle and ReachMe are to be checked. A value
  read from JSON, or built as object literals and arrays, is unaffected.

This change was drafted as `treat-host-values-as-json`, and the archived changes
`add-agent-host-package` and `name-the-argument-in-boundary-diagnostics` refer to it by that name.
