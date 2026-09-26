---
title: 'Textual Content'
description: 'Typed text elements, raw blocks, and embedded expressions.'
---

Text nodes in NX are typed, so you can opt into different text processors (e.g., markdown) or keep raw text when needed.

## Default text

```nx
<p>Hello, world.</p>
```

Plain elements without a text type use the host’s default (often plain UI text).

## Text at a `string` content property

When a type declares a `string` content property, a body of text and braced values binds to it as
one string: each run of text as written, and each braced value in its text form.

```nx
type Label = { content text:string }

let <Total count:int /> = <Label>Total: {count}</Label>
let <FullName first:string last:string /> = <Label>{first} {last}</Label>
```

`<Total count=3 />` binds `text` to `"Total: 3"`, and `<FullName first="Ada" last="Lovelace" />`
binds it to `"Ada Lovelace"`. A body can also be a single braced value, so `<Label>{count}</Label>` binds `"3"`.
A braced value must be a `string`, a number or a `boolean`, the same values `+` joins to a string;
a record, a sequence or a value that may be empty is a type error naming the type. The numbers print in the
[canonical text forms](/reference/syntax/expressions#canonical-text-forms), so `{1.0}` reads `1`.

A body under a tag with no declaration, such as `<p>`, is not type checked and is unchanged by this
rule, as is a body that binds to an `Element` or list content property.

### Line breaks are layout

A body means the same thing whether it's written on one line or across several. Where a line ends
and how far the next one is indented depend on how the code is formatted, so they aren't part of
the text:

- Whitespace that contains a line break, together with the indentation after it, reads as **one
  space**. That holds between two pieces and inside a run of text.
- Whitespace within a line is **kept as written**.
- The whitespace at the start and end of the body is **removed**.

```nx
type Label = { content text:string }

let <TaskSummary count:int first:string last:string /> =
  <Label>
    {first}
    {last}
    has {count}
    open  tasks
  </Label>
```

`<TaskSummary count=3 first="Ada" last="Lovelace" />` binds `"Ada Lovelace has 3 open  tasks"`, the same string as
writing the body on one line. Moving the element into a deeper container, or reformatting it,
never changes the text.

Only text is laid out this way. A braced string is a value and keeps its text exactly, so `{"  a "}`
keeps its spaces wherever it stands, and a braced string literal that spans two lines puts a real
line break in the text. Raw text (`<Label:raw>`) is also kept exactly as written.

A typed text body (`<Label:markdown>`) keeps its line breaks, because they are the text processor's
to read: a blank line separates two Markdown paragraphs, and a line that starts with `-` is a list
item. Only the indentation the source is written at comes off — the indentation every line shares,
so a nested list keeps the rest — along with the line break after the open tag and the blank line
before the close tag. Each `@{}` value is joined as a braced value is.

```nx
type Note = { content text:string }

let <TaskNote count:int /> =
  <Note:markdown>
    # Tasks

    You have @{count} open.

    - one
    - two
  </Note>
```

`<TaskNote count=3 />` binds `"# Tasks\n\nYou have 3 open.\n\n- one\n- two"`, which a Markdown processor reads
as a heading, a paragraph and a two-item list.

#### How this compares with JSX, HTML and XAML

- **JSX** drops whitespace that contains a line break. Written on separate lines, `{first}` and
  `{last}` read `"AdaLovelace"`, and code has to add `{" "}` to get the space back. NX reads the
  same line break as one space, the way the words read on screen, so splitting a long body across
  lines never glues two words together.
- **HTML and XAML** turn every run of whitespace into one space, including spaces within a line.
  NX changes only whitespace that contains a line break. Spaces typed on one line are something the
  author chose, and a formatter never introduces them.
- **No layout at all**, keeping every character, would make the text depend on indentation. The
  same label would read differently at a different nesting depth.

## Explicit text types with `:TextType`

```nx
<Note:markdown>
  **Bold** and _italic_ text.
</Note>
```

- `:markdown` (or any identifier) selects a text processor.
- Attributes can still appear on typed text elements.

## Raw text

```nx
<Snippet:raw>
  {"literal braces stay untouched"}
</Snippet>
```

Use `raw` to prevent interpretation of braces or entities.

## Embedded expressions in text

```nx
let balance = 42

<p:text>
  Account balance: @{balance} credits
</p>
```

- `@{}` interpolates expressions inside typed text content, such as `<p:text>` above.
- A body without a text type, such as a plain `<p>`, embeds a value with `{balance}` alone; there
  `@` is ordinary text.
- Escape `@` as `\@` when you need a literal at-sign.

## See also (Reference/Grammar)
- Reference: [Elements](/reference/syntax/elements)
- Reference: [Expressions](/reference/syntax/expressions)
- Grammar: [nx-grammar.md – Elements/Text](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements)
