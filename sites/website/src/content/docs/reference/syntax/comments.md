---
title: 'Comments'
description: 'Ordinary comments, and the `///` doc comments that document declarations and members.'
---

NX has three kinds of ordinary comment, which the compiler ignores, and one kind of doc comment,
which documents the declaration or member it is attached to. Editors show that documentation in
hover and completion, and `nxlang typegen` carries it into the generated C# and TypeScript. For the
grammar, see [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#lexical-structure).

## Ordinary comments
- `//` comments run to the end of the line.
- `/* */` and `<!-- -->` comments can span lines.
- A comment may appear anywhere whitespace may, except inside a string literal or text content,
  where `//` is just text.

```nx
// A line comment.
/* A block comment
   over two lines. */
<!-- An HTML-style comment. -->
let greeting = "Hello"   // A trailing comment.
```

## Doc comments
A line comment with exactly three slashes, `///`, is a doc comment. Four or more slashes make an
ordinary comment, so a banner such as `//////////` documents nothing, and a block comment is never
documentation.

A doc comment is written either above what it documents or after it on the same line.

- **Leading:** one or more `///` lines on their own documents the item that starts on the next line.
  When several items start there, as an `emits` entry and its first field do, the outermost is
  documented.
- **Trailing:** a `///` after code documents the item that both starts and ends on that line. It
  continues onto the `///` lines directly below it whose `///` starts at the same column, so a
  member's documentation can run to several lines and stay beside the member.

```nx
/// A text box that reports each keystroke.
///
/// Shows [placeholder] while the box is empty.
export component <SearchBox
  value:string = ""            /// The current text.
  placeholder:string = ""      /// Hint text shown while the box is empty,
                               /// and read aloud by screen readers.
  emits {
    /// Fired on every keystroke.
    ValueChanged { value:string }
  }
/> = {
  <span>{value}</span>
}

/// Where a search stands.
export type LoadState =
  | idle                       /// No search has run yet.
  | loading
  | failed { message:string }  /// The last search failed.
```

Everything that can be declared can be documented: type aliases, records, actions, unions, values,
functions, element-style functions and components, and within them record and action fields, union
cases and their payload fields, component properties, `emits` entries, `state` fields, and function
parameters. Type parameters, such as `T:type`, are the exception: a doc comment on one is an error.

Because a `///` line directly below a trailing doc comment continues it, a leading block for the
next item goes after a blank line:

```nx
type Shelf = {
  label:string     /// The label.

  /// Which way the shelf faces, as seen from the room.
  facing:string
}
```

The text after `///` and one following space is the documentation. It is
[CommonMark](https://commonmark.org), so `**bold**`, `` `code` ``, lists, and links work as they do in
any Markdown. Its first paragraph is the summary, which tools show where there is room for only a
line or two. A blank line ends the summary, and so does a list that starts right after it. There are no `@param` or `@returns` tags: a parameter or member is documented where it
is declared.

## Doc links
`[Name]` in documentation links to a declaration, and `[Type.member]` to one of its members: a
field, a union case, a case's payload field, or a component property. A name is looked up first
among the members of the declaration the documentation is on or inside, and then among the
module's top-level names, imports included, so a property can link to its sibling as `[value]`. A
markup-style member name links as written: `[aria-label]`.

```nx
type LoadState = idle | loading

/// The state a search starts in: [LoadState.idle].
let initial: LoadState = idle
```

Editors show a link that resolves as code, and hovering it reports what it names. As you type a
link's name, the editor suggests the names it can resolve to, in the same order, and after `Type.`
the members of `Type`; right after `[` or `.`, ask for them with Ctrl+Space. A link that names
nothing is a warning, never an error. Brackets inside code, as in `` `items[index]` ``, are not a
link, and neither is a Markdown link with a destination, `[the spec](https://nxlang.org)`.

## Attachment errors
A doc comment must document exactly one item, so each of these is an error:

- A leading block that is not directly followed by a documentable item — for example one followed by
  a blank line, by an ordinary comment, by an expression, or by the end of the file.

```nx invalid
/// Theme colors.

type Theme = light | dark
```

- A trailing doc comment on a line where no item both starts and ends, such as the line that closes
  a record, or on a line with more than one item.

```nx invalid
type Contact = {
  name:string
}   /// A contact.
```

- A `///` line directly below a trailing doc comment that is not aligned with it. Align it to
  continue the comment, or put a blank line before it to document the item below.

```nx invalid
type Shelf = {
  label:string     /// The label.
  /// Which way the shelf faces.
  facing:string
}
```

- An item documented both ways. The trailing doc comment is the one reported.
