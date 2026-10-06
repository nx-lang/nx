---
title: 'Declaration Schemas'
description: 'How the compiler SDKs describe an NX function or type as JSON Schema, with doc comments as descriptions.'
---

A host that hands an NX function to a system that speaks JSON Schema, such as a language model's
tool interface or an MCP client, asks the compiled program for the function's schema. The compiler
SDKs, `@nx-lang/sdk-wasm` and `@nx-lang/sdk-node`, answer from a program artifact with JSON Schema
(draft 2020-12) for the function's arguments and for its result, with the author's
[doc comments](/reference/syntax/comments#doc-comments) as descriptions:

```ts
const tool = artifact.functionSchema({ name: "findPlans" });
tool.inputSchema;  // the arguments, an object keyed by parameter name
tool.outputSchema; // the result
tool.description;  // the function's doc comment, as Markdown
```

`typeSchema` answers the same way for a declared type: a record, action, union, type alias,
`<Target>.Update` or `<Target>.Property`. Both queries first ship in release 0.6.0, together with
doc comments.

The schemas are derived when the program is compiled. An NX IR image holds no doc comments, no
alias targets and no generic type arguments, so a host that only runs images stores the schemas it
derived beside them.

## The contract

A schema describes the canonical JSON encoding of NX values, and nothing wider:

- A value that is valid against a function's input schema is accepted as its arguments by a
  runtime running the IR emitted from the same program.
- A value the function returns is valid against its output schema.

A runtime accepts more than the schema says, such as a single value where a sequence is expected,
or `null` for an absent optional. The schema leaves those spellings out, so a model or client is
asked only for the canonical one.

## The mapping

| NX | JSON Schema |
| -- | ----------- |
| `string` | `{ "type": "string" }` |
| `boolean` | `{ "type": "boolean" }` |
| `int`, `int64` | `{ "type": "integer" }` |
| `int32` | `{ "type": "integer", "minimum": -2147483648, "maximum": 2147483647 }` |
| `float32`, `float64` | `{ "type": "number" }` |
| `object` | `{ "not": { "type": "null" } }`: any value but `null`, which a runtime reads as no value |
| `T+` | `{ "type": "array", "items": T, "minItems": 1 }` |
| `T*` | `{ "type": "array", "items": T }` |
| A field or parameter marked `?` | The schema of its type, left out of `required`, never `null` |
| A function result `T?` | `{ "anyOf": [T, { "type": "null" }] }` |
| A record or action | A closed object (`additionalProperties: false`) with one property per field, inherited fields first, and its `$type` |
| A union of constant cases | `{ "type": "string", "enum": [...] }` |
| A payload case | A closed object whose `$type` is `"<Union>.<case>"` |
| A union with payload cases | `anyOf` the constant cases as a string `enum`, then each payload case |
| An abstract record | `anyOf` every concrete record and union case in the program that extends it |
| An applied generic record, `<Page T=Plan />` | Its own definition, `Page_Plan`, with the argument substituted |
| A type alias | The schema of its target |
| `<Target>.Update` | Every field optional; a field the target marks `?` also admits `null`, which clears it |

Records, unions, payload cases and applied generic records are entries of the document's `$defs`,
referred to by `$ref`, so a type that refers to itself is described once. A field is `required`
unless it is marked `?` or has a default. A default written as a literal, or as a constant case,
becomes the `default` keyword; a computed default is left out.

## Input and output

Where a value is read decides where `$type` is asked for.

- **Output**, a value a runtime returns: every record and payload case carries `$type`, because
  every runtime writes it. A field with a default is always present, so it is `required`.
- **Input**, a value a host supplies: a record at a site typed by that record needs no `$type`,
  because the runtime knows which record it is. `$type` is asked for only where the runtime needs it
  to choose a shape: at an abstract record, and for a payload case.

A function's input schema is written for input and its output schema for output. `typeSchema`
takes the direction as an option and answers for output by default.

## Descriptions

Each doc comment becomes the `description` of what it documents, as Markdown with every doc link
written as code: `[maxMonthlyPrice]` becomes `` `maxMonthlyPrice` ``.

```nx
/// A plan a team can buy.
type Plan = {
  name:string   /// The plan's display name.
}

/// Finds the plans that fit a team.
let findPlans(
  teamSize:int   /// Number of people who need a seat.
): Plan* = { <Plan name="Team" /> }
```

Here the function's documentation is the answer's `description`, with its first paragraph as
`summary`, and the parameter's is the `description` of the `teamSize` property. A record's, union's
and payload case's documentation describes its `$defs` entry, and a field's describes its property,
beside a `$ref` where the field's type is a record. JSON Schema has no place for a description per
`enum` value, so the documented constant cases of a union are listed at the end of the union's
description. Something with no doc comment has no `description`.

## Types with no JSON form

Some types have no JSON form. The schema that would mention one is left out of the answer, and a
`schema-inexpressible-type` diagnostic names the member that holds it. The other schema of the
function is still answered.

- A function type, and the function reference type `<function ... />: R`. A function value names
  code; a host supplies one, never a JSON value. That makes the agent library's `Tool` and `Agent`
  host configuration rather than data: a site typed by either has no JSON form.
- A component used as a type, and markup: an element such as `<div />`, or `Element`.
- An abstract record that nothing in the program extends.
- A result type the checker could not infer. Annotate the result.

Two shapes at one abstract-record site that share a `$type`, such as two records named `Card` in
two modules, are reported as `schema-ambiguous-discriminator`, since a runtime refuses such a value
as ambiguous.

A host can name types it supplies itself, such as the agent library's `ToolContext`, with the
`hostSuppliedTypes` option. A parameter declared with one of them, with a record that extends
one, or with a type alias of either, is left out of the input schema and marked as supplied by the
host.

Every parameter's entry names the record or union it is declared with, as `typeRef`. Through a
type alias it names what the alias denotes: with `type Context = ChatToolContext`, a parameter
`context:Context` has the `type` `Context` and the `typeRef` `ChatToolContext`. An alias of
anything else, such as `type Plans = Plan+`, has no `typeRef`.

A parameter whose type only holds such a type, as `contexts:ChatToolContext+` does, or a record
with a field of it, cannot be filled in by the host and stays in the input schema. Its entry names
the listed types it holds, as `hostSuppliedWithin`, so a host can refuse a function that would ask
a caller for values the host means to supply. A type derived from a listed type,
`ChatToolContext.Update` or `ChatToolContext.Property`, is another type and holds none of it.

## Large integers

An `int64` is described as `integer`. JSON numbers are exact only up to 2^53 in JavaScript, so an
`int64` beyond that range does not survive a JavaScript host exactly, as it does not in the
canonical encoding itself. `int` is exact over that range on every backend.
