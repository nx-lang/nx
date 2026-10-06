## Context

See `proposal.md` for the motivation. What shapes the approach:

- What a host value should be, as the project means it: one data model for every host language,
  with an exception only where a language needs one; like JSON, and easy to read from it and
  write to it, but not JSON, since a host as often computes a value as reads one; and as cheap as
  it can be, cheapest where the host and the runtime are the same language.
- That model exists already and has a name. `docs/nx-ir-format.md` (*Values*) and the
  `nx-ir-format` spec (*NX IR preserves canonical NX value encoding rules*) describe "canonical
  values"; the Rust runtime takes them as `NxValue` and the TypeScript runtime as
  `NxCanonicalValue`. What is missing is the statement that a host value is one, in the host's
  own data, and that JSON is an encoding of it. The first draft of this change called the model
  "canonical JSON", which is the encoding.
- `docs/nx-ir-format.md` (*Input size*) and the `nx-ir-format` spec already name the canonical
  values of a JavaScript host: `null`, a boolean, a number, a string, an array, a plain object,
  and the objects the runtime made for its own use. They say a value that is none of these "is one
  value and is not read further", and that what a runtime then does with it, "which is mostly to
  refuse it, is unchanged" (`add-ir-runtime-input-limit-and-cost-tests`, design decision 2). The
  second half is not true of the code, which is where this change starts.
- The measure (`measureValue`, with `isPlainInput`) enters arrays and plain objects, by their
  prototype, and counts every member of an object, one that is `undefined` included. It runs only
  when the host sets `maxInputSize`.
- The check (`normalizeValue` and what it calls) is not only for host input. It is the same code
  that checks a value the program computed, at an inner call's parameter, a record's field or a
  result. Where a record is expected it accepts any object that is not an array (`requireObject`,
  `isObject`). At a site typed `object` it returns the value as it is, unread. It reads a
  record's fields by `hasOwnProperty`, so a member that is `undefined` is present and is checked
  as a value, which fails.
- The check builds a new record for every record it checks, and the result writer builds a new
  value for everything it writes. So today the only values the runtime shares with the host are
  the ones held at a site typed `object` while a call runs. A result never holds a host's object.
- An argument by name that is `undefined`, and a positional argument that is `undefined` or a
  hole, already have a meaning (not supplied; the empty value). Only a member inside a value does
  not.
- `name-the-argument-in-boundary-diagnostics` is applied. `invokeFunction` knows when it was
  called by the host (`fromHost`) and gives a boundary failure raised while it binds a host's
  argument the member `argument`. `@nx-lang/agent` classifies a failure by that member:
  `invalid-context` or `invalid-input` when every diagnostic names an argument,
  `evaluation-failed` otherwise.
- `@nx-lang/agent` carries `withoutUndefinedMembers` (`packages/agent/src/json.ts`): a bounded
  copy of the context record that leaves `undefined` members out of plain objects, written to
  enter exactly what the measure enters. Its review (`add-agent-host-package`, RF11 and RF17 to
  RF20) is the record of how hard that is to do from outside the runtime.
- TypeScript generated from NX declares records as interfaces, so a host that follows the
  generated types builds object literals.

## Goals / Non-Goals

**Goals:**

- One statement of what a host value is, for every runtime: a canonical value in the host's own
  form, with JSON as an encoding of it.
- One definition of that form in the TypeScript runtime, applied by the measure, by what is
  refused and by what the program holds.
- A member that is `undefined` means the same as a member left out, at any depth.
- The input limit bounds everything the runtime accepts, and a result holds only canonical
  values.
- No cost to a host that passes plain data: nothing encoded, nothing copied, one pass that
  allocates nothing.
- `@nx-lang/agent` has no copy of its own.

**Non-Goals:**

- Accepting host objects that are not canonical values by converting them (a `Date` to a string,
  a `Map` to a record, a class instance to its members). A host converts what it has; the runtime
  does not guess. Decision 1 says what would add one to the form.
- Numbers JSON cannot spell (`NaN`, an infinity) and integers outside the safe range. They have
  their own diagnostics today and keep them.
- A host object built to answer differently each time it is read, by an accessor or a proxy, or
  one with members that are not enumerable. The rule is for data. A host is not defended against
  an object it built to mislead the runtime it called.
- The helpers that take no runtime options (`applyUpdate`, `mergeUpdates`, `diffRecords`). They
  are given values the runtime returned.
- Making the check or the result writer cheaper. Both copy what they are given, which costs more
  than anything this change adds; `add-ir-runtime-performance-harness` is where that is measured.
- Any change to the Rust runtime's code, to operation counts or to the NX IR format.

## Decisions

### 1. A host value is a canonical value in the host's own form, and JSON is one encoding of it

The new requirement of `nx-ir-format` says it in four parts.

- **The model.** The empty value, a boolean, a number, a string, a sequence, a record with an
  optional type name, and the function values and action handlers a runtime makes. This is what
  NX evaluates to, and it is the same in every host.
- **The form.** Each runtime defines one way to hold each kind of value in its host's language,
  out of that language's own data, and takes and returns values that way with no encoding step.
  So the standard is the model, and the same value passed to two runtimes is the same input.
- **JSON.** Canonical JSON encodes the same values. What a host reads from it is in the form, so
  a host that has JSON passes what it parsed, and a host that computed a value passes that. A
  form may hold a spelling JSON lacks: `undefined` for a member left out in JavaScript, a 64-bit
  integer in Rust. Each has one meaning and one size.
- **What is not a value.** Refused where it is passed, with its path, and not converted.

| A canonical value | Canonical JSON | JavaScript | Rust |
| --- | --- | --- | --- |
| The empty value | `[]`, or the key left out | `[]`, or the member left out or set to `undefined` | `NxValue::Array` of no items, or the property left out |
| A boolean | `true`, `false` | A boolean | `NxValue::Bool` |
| An integer | A number. Outside JavaScript's safe range, the `nx.int` record | A number. The runtime cannot hold one outside the safe range | `NxValue::Int`, or `NxValue::Int32` |
| A float | A number | A number | `NxValue::Float`, or `NxValue::Float32` |
| A string, a constant union case | A string | A string | `NxValue::String` |
| A sequence | An array | An array | `NxValue::Array` |
| A record | An object, with `$type` where the site needs it to tell the type | A plain object, with a `$type` member likewise | `NxValue::Record`, with its `type_name` likewise |
| A function value, an action handler | The record that names it | That record, or, where a value is held, the value the runtime returned in a state | That record |

A host's `null`, `NxValue::Null` in Rust, is read as the empty value where a site admits none.

This is the table of `docs/nx-ir-format.md` (*Host values*), as task 4.2 checked each cell
against the runtime it describes.

**Why refuse and not convert.** A conversion has to choose: a `Date` as ISO text or as
milliseconds, a `Map` with keys that are not strings, a class instance with or without the
members it keeps for itself. Whatever is chosen, the value means something the host did not
write, the result holds something the host did not pass, and the choice can never be changed
without changing what a working host gets. A refusal names the path and costs the host one line.

**What adds a type to a form.** A host's type joins a form as the spelling of a kind of value NX
has, and not before. A big integer joins the JavaScript form when the TypeScript runtime can hold
an integer outside the safe range. A `Date` or a byte array joins when NX has a date or a bytes
type to mean. Refusing them until then is what keeps that open: accepting tomorrow what is
refused today breaks no host, and changing what a guessed conversion meant would break every host
that relied on it.

**An instance of a host's class** is the one case where the kind exists already, since its
members could be read as a record. It stays outside the JavaScript form, for three reasons. The
input-size rule already defines a plain object as the record of JavaScript, and the measure and
the check have to agree. An instance holds what its class keeps for itself, a cache, a connection,
a parent, and reading its members passes that to the program and, at a site typed `object`, back
out in a result. And the generated TypeScript types are interfaces, so the values a host is led
to build are literals. A host with an instance spreads it, as `@nx-lang/agent` does for the top of
the context record. If hosts turn out to need it, accepting instances later breaks nothing.

Alternatives considered:

- *Say host values are JSON, as the first draft did.* It is nearly the same code, and the wrong
  rule: it reads as if a host had to produce JSON, it makes `undefined` and the runtime's own
  handler objects exceptions to explain, and it leaves no way to add a spelling JSON lacks.
- *Give JavaScript hosts a value type of the runtime's own to build, as Rust hosts build
  `NxValue`.* One more type between a host and its data, and a copy of every value on the way in.
  Rust needs `NxValue` because Rust has no such data of its own. JavaScript has.

### 2. Host input is read once, at the entry, and that is where it is made canonical

Every evaluation API already starts by measuring its input when a limit asks for it. This change
adds a second step in the same place, which always runs: **read the input**. It walks every value
the measure covers and does two things.

- It refuses a value that is not canonical, with `nx-ir-boundary-type` and the path.
- It leaves out a member of a plain object that is `undefined`. When it finds none, which is the
  usual case, the value goes on as the host passed it and nothing is allocated. When it finds
  one, the objects on the way down to it are copied without it and the rest is shared, so the
  host's own object is never changed.

After it, everything the check and the program see is canonical. The check, the writer and
equality need no change, and no later code has to know what `undefined` means.

It has to be at the entry, and cannot be in the check, for one reason: the check also runs on
values the program computed. A walk of a whole value at every `object` site would be work no
operation pays for, repeated each time a value crosses such a site inside a loop, and the
operation budget would stop bounding time. At the entry it is once per call and bounded by the
size of the input, which is the host's to limit.

Order at the entry: validate the options; measure the input and refuse it if it is over the
limit; read the input; then proceed. Input over the limit is therefore still refused by the
limit, as the spec for the limit requires, whatever it holds.

The arguments of a function are the one input read a step later: once the function the host
called is found, each by its parameter's name, all of them before the first is bound
(`readHostArguments`, `readNamedHostArguments`). That is where a parameter's name is known, see
decision 6. It is still before any argument is checked against its type and before
any node is evaluated, a default's included.

What it costs: one pass over the input that allocates nothing, before a check that allocates a
record for every record and a writer that does the same for the result. Task 2.2 measured it
straight after the reading existed, and the first version was not that. A walk with its own stack,
a set of the objects it is inside and a list of names for every object cost 60% of a call that
takes 10,000 records. So the reading is in two parts. `isReadAsPassed` answers the usual case, a
value with nothing to refuse and nothing to leave out, with no stack, no set and no list, by a
recursion that stops at 32 levels. Whatever it does not settle, a value to refuse, a member to
leave out, a value nested more deeply, which is every value that holds itself, goes to the full
walk, which keeps its own stack and builds the path and the copies. With it the reading costs
about 5% of that call (task 2.2 has the numbers), and the engine's stack is used to a fixed depth
whatever the input.

A call by name has one more thing to read: the record of arguments itself, which has to be a
plain object, and what it holds under a name the function does not declare. That is dropped, as
the subset rule allows, but the limit covers it and it is the host's value, so it is read, with no
argument named, since it is in none.

Alternatives considered:

- *Refuse a value that is not canonical where the check meets one, and look into `object` sites
  there.* Rejected for the reason above. It could be made to work by telling the check when a
  value came from the host, but every one of the eight entry points and each default would have
  to set and clear that, and the handful of readers that take host input without the check
  (a batch's entries, handler properties, content) would each need the rule again.
- *Leave such a value alone at an `object` site, as an opaque value the runtime never reads, and
  refuse it only where a typed site meets it.* The measure, the check and the writer could be
  made to agree on "opaque", and it would cost nothing. But a result would then not always be
  canonical, a host would learn of a `Date` it passed only when a function happened to return it,
  and "what is a host value" would still have two answers.
- *Fold the reading into the measure's walk.* With a limit set it would be one pass where this
  design makes two. But the measure stops at the limit and runs only when asked, the reading must
  finish and always runs, and a refusal found before the limit is passed would have to be held
  until the measure ended, so that the limit still came first. Two walks that each do one thing
  are simpler. If task 2.2 finds the second walk costs too much, this is the first thing to try,
  and it changes no spec.

### 3. An `undefined` member is a member that was left out

It is what `JSON.stringify` does with one and what TypeScript's optional properties produce, so
it is what a host means. Three consequences, each stated in the spec:

- A field with a default takes the default, and an optional field is empty.
- A required field with no default is reported missing, with the diagnostic a missing field has.
- A member whose name the type does not declare is not an unknown field. It is not there.

An `undefined` item of an array is not the same thing: an item cannot be left out without moving
the ones after it, and one is far more often a mistake in the host than a value it meant. The
positional arguments of a call keep the meaning they have (the empty value). Anywhere else such an
item is not canonical and is refused. Reading it as `null` later, as `JSON.stringify` writes it,
would break no host.

The measure follows: an `undefined` member is not counted, neither its name nor a value, so one
input has one size however the host spelled it, which is the rule the `nx-ir-format` requirement
already gives for every other spelling. The measure reserves room for an object's members when it
lists their names, and refuses an object with more members than the limit has room for without
reading them. With this change the members it counts are the ones that are not `undefined`, so it
looks at each member's value once when it lists the names. That is work in proportion to the
names listed, which is the one cost the limit already does not bound.

### 4. What is canonical is decided by the test the measure already uses

`isPlainInput` is the definition: an array, or an object whose prototype is `Object.prototype`,
of any realm, or nothing, and beside them the handler and function values the runtime made, known
by having made them. The reading uses the same function, so the measure and the reading cannot
drift apart. `requireObject` stays as it is: after the reading it can only be given a plain
object, a value the program built, or one of the runtime's own.

A plain object that holds itself has no finite size. With a limit set the measure refuses it, as
today. With none, the reading refuses it: it keeps the objects on the path from the root to where
it is, and an object found on its own path is refused with `nx-ir-boundary-type`. Only the path
is kept, not every object seen, so the same object held twice side by side is accepted, as it has
to be.

### 5. The measure still counts one for a value that is not canonical

`measureInputSize` is exported, needs no program and refuses nothing. It keeps counting one for
such a value and not entering it. In a call the value is refused straight after, so the count no
longer stands for anything the runtime goes on to read, which is what closes the hole: nothing
that measures 8 can cost five million.

### 6. A refusal inside an argument names the argument

The requirement *TypeScript runtime diagnostics name the argument a failure is in* already covers
"any other refusal of the value itself", so a value the reading refuses inside what the host
passed for a parameter carries `argument`, and one anywhere else carries none. Nothing is added
to that requirement. The code follows it by reading a function's arguments by their parameters
(decision 2) and naming the argument of a refusal with the function that already names the
argument of a boundary failure (`namingArgument`), so the rule for which failures are in an
argument is in one place. Each argument is read by its
parameter's name, so the path in the message begins as the check's does.

`@nx-lang/agent` needs this and nothing else. A class instance or a `Date` in the context record
is refused in the context argument, and so is `invalid-context`: the host's mistake, which the
model cannot correct. Without the name it would be `evaluation-failed`, which says the function
failed.

### 7. The agent package passes the record and nothing more

`execute` goes back to `{ ...context, callId }`. The spread is what makes the top-level record a
plain object whatever built it, which the package has always done and its spec keeps. Below the
top level the runtime's rule applies: a plain object's `undefined` members are absent, and a
class instance is refused by name. `withoutUndefinedMembers`, `isPlainRecord` and their tests are
removed. The package's size check of the context record (`maxContextSize`, by `measureInputSize`)
stays, and is what refuses a record that holds itself before the call.

The test that pins "an instance of a class below the top level is passed as it is, and its
`undefined` member is refused" changes to what the runtime now says: the instance is refused as
not a plain object, whatever its members hold, and the failure is `invalid-context`.

### 8. Nothing recorded moves

The reading charges no operation and the measure counts as before for input with no `undefined`
member, so the conformance corpus, whose cases are JSON, and the differential cost test, whose
values are generated as JSON, pass unchanged. That is the check that the change did what it says
and no more.

## Risks / Trade-offs

- [A second walk of the input on every call, limit or no limit] → It is one pass with no
  allocation in the usual case, before a check and a writer that each allocate. Task 2.2 measured
  a component with a large state and a function with a large argument before and after: about 5%
  of a call that does nothing but take 10,000 records, at the bound this design set and not
  clearly under it. It was not folded into the measure's walk: that would help only a call with a
  limit, which already pays more than three times as much for the measure. The harness of
  `add-ir-runtime-performance-harness` is where it should be watched.
- [A host passes something that is not plain data today and it works by accident: a class
  instance as props, a `Date` in state] → It is refused with a message that names the path. Task
  1.3 looks at the three hosts in reach (the playground, the DrawnUI fiddle, ReachMe) before the
  code changes.
- [A host with objects of its own classes has to spread them at each level] → Decision 1 gives
  the reasons, and the rule can be loosened later without breaking a host. It cannot be tightened
  later.
- [`runtime/typescript/dist` is committed] → Rebuilt and committed with the source.

## Migration Plan

A host that passes plain data, read from JSON or built as literals, sees one difference: a member
it set to `undefined` now means absent, where the call failed. A host that passes anything else
is refused at the call with the path to the value, and converts it before the call.

The entry of `specs/future.md` this change resolves is removed when it is applied.

## Open Questions

- **Should the helpers that take no options refuse a value that is not canonical too?** They are
  outside this change. A host gives them values the runtime returned, which are canonical once
  this lands. It can be decided when someone passes them something else.
- **Should an instance of a host's class be read as a record?** Not in this change (decision 1).
  No host in reach passes one (task 1.3): the playground does not call this runtime, ReachMe
  passes no values at all, and the DrawnUI fiddle passes literals and values the runtime returned.
