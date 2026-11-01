---
title: 'NX Style Guide'
description: 'Formatting conventions for readable, consistent NX source.'
---

These conventions apply to handwritten NX source, including standard libraries, examples, and
documentation. They describe the preferred formatting; the syntax reference defines what the
language accepts. Existing examples may still use older formatting.

## Indentation and line length

- Use **2 spaces** per indentation level, including continuation lines. Do not use tabs.
- Aim for **120 characters** per line, including indentation and trailing comments. Treat this as a
  readability target rather than a hard limit: preserve long URLs, indivisible tokens, and
  meaningful text rather than breaking them arbitrarily.
  A small overrun is preferable to awkward wrapping. Keep standalone prose comments around
  **100 characters** where practical.
- Use LF line endings, remove trailing whitespace, and end each file with one newline.
- Let nesting determine indentation. Avoid aligning continuation lines under a distant opening
  parenthesis or attribute.

## File layout and blank lines

Put a file overview first, followed by imports, then declarations. Keep related declarations
together in a useful reading order, such as public types, constants, and functions.

Use one blank line:

- After the file overview and after the import group.
- Between top-level declarations, including types, constants, and functions.
- Between logical steps inside a longer function, when it helps explain the flow.

Closely related, self-explanatory single-line constants may form a compact group without blank
lines between declarations. Introduce the group with one comment instead of repeating each
constant's name in an individual documentation comment.

Keep documentation comments directly attached to their declaration. Do not insert blank lines
between every record field, consecutive imports, or each item in a simple sequence. Avoid blank
lines immediately inside braces and avoid multiple consecutive blank lines.

```nx
import "@nx/color" as color

type Theme = {
  accent: color.Color
  surface: color.Color
}

let theme = <Theme accent=#5b6ef5 surface=#ffffff />

let root() = { theme }
```

## Imports and aliases

Use unqualified imports for a file's main vocabulary. For example, a drawing-focused file can
import `@nx/graphics` without an alias so its elements read as `Drawing`, `Circle`, and `Paint`.
Use namespace aliases for supporting libraries when the prefix explains an expression's origin,
such as `math.sqrt` or `color.mix`.

```nx
import "@nx/graphics"
import "@nx/math" as math

let root() =
  <Drawing viewBox=<Rect x=0 y=0 width=100 height=100 />>
    <Circle cx=50 cy=50 r={math.sqrt(1600)} />
  </Drawing>
```

- Use namespace aliases to disambiguate overlapping names. When several libraries contribute
  equally, prefer descriptive aliases that distinguish their roles.
- Prefer meaningful aliases such as `math`, `color`, or `graphics` over single-letter aliases
  such as `m`, `c`, or `g`.
- Use selective imports when only a few declarations are needed: `import { Color } from "@nx/color"`.
- Keep the choice consistent within a file. Qualify names when the prefix adds useful context;
  omit it when the file already establishes the domain.

## Spaces and punctuation

- Put a space after a type annotation colon and after a comma: `value: Color`, `mix(first, second)`.
- Put spaces around declaration/default `=` and binary operators: `let total = price * count`.
- Keep attribute assignment compact: `accent=#5b6ef5`, `value={total}`.
- Do not put spaces just inside parentheses or around member access: `f(value)`, `theme.accent`.
- Use spaces inside a one-line braced expression or record: `{ total }`, `{ name: string }`.
  The empty form is `{}`.
- Separate inline attributes with a single space and put a space before `/>`.
- Avoid padding declarations with spaces to align their colons or equals signs across lines.
- Do not add statement terminators or punctuation that the syntax does not require.

```nx
type Size = { width: float64 height: float64 }

let area(width: float64, height: float64): float64 = { width * height }

let root() = { area(12, 8) }
```

## Braces and wrapping

Keep an opening brace on the same line as its declaration or control expression when it fits.
Put the closing brace on its own line for a multiline body, aligned with the start of the
declaration or expression. A short, simple body can stay on one line.

Use one field per line for a record with several fields or field documentation. A small record
can stay on one line. Do not add commas between record fields.

When a function signature or call exceeds the line-length target, put each parameter or argument
on its own line, indented by 2 spaces. Keep separating commas, omit the trailing comma, and align
the closing parenthesis with the start of the declaration or call. Keep the return annotation
beside the closing parenthesis.

```nx fragment
export external mix(
  first: Color,
  second: Color,
  weight: float64 = 0.5,
  space: InterpolationSpace = oklab
): Color
```

If a declaration's initializer needs a separate line, indent it by 2 spaces. Indent any further
nesting relative to that line.

## Union declarations

Omit the leading `|` for a single-line union with multiple cases. When the case list spans
multiple lines, indent it by 2 spaces and put a leading `|` on every case, including the first.

A single-case union always needs its leading `|`: without it, the declaration is a type alias.

```nx
type ThemeMode = light | dark

type LoadState =
  | idle
  | loading
  | failed { message: string }

type Wrapper = | only
```

## Elements

Keep short elements on one line. Once an opening tag needs multiple lines, including because an
attribute contains a multiline value, put each attribute on its own line, indented by 2 spaces.
Put `>` or `/>` on a separate line aligned with the opening `<`. For an element used as a
multiline declaration initializer, start the element on the line after `=`, indented by 2 spaces.

Keep an attribute's opening `{` beside its `=`, indent its value by another 2 spaces, and put the
closing `}` on its own line aligned with the attribute. Short elements inside that value can stay
on one line. For a nested element attribute value, indent its attributes relative to the containing
attribute line rather than aligning under its distant opening `<`.

Indent child elements by 2 spaces and align closing tags with their opening tags.

```nx
type Panel = {
  title: string
  foreground: string
  background: string
}

<Panel
  title="Account settings"
  foreground="#111827"
  background="#ffffff"
/>
```

```nx
import "@nx/graphics"

let chartFade(): Paint =
  <Paint.linearGradient
    x1=0
    y1=160
    x2=0
    y2=240
    stops={
      <GradientStop offset=0 color=#38bdf8c0 />
      <GradientStop offset=1 color=#38bdf800 />
    }
  />
```

Preserve meaningful whitespace in textual content. Do not reflow prose, strings, or mixed
text and elements solely to satisfy indentation or line length.

## Comments and documentation

Use `//` for implementation notes and file overviews, and `///` for API documentation.
Document public types, fields, constants, and functions. Explain meaning, units, valid ranges,
defaults, and notable behavior rather than repeating the declaration's name or type.
For self-explanatory constant groups, a shared comment is sufficient.

```nx
// CSS named colors, using camelCase NX identifiers.
export let lightSkyBlue = #87cefa
export let darkOliveGreen = #556b2f
```

Prefer trailing `///` documentation for properties, including record fields, union-case fields,
and component props. Keeping the declaration and its short description together makes property
lists compact and easy to scan.

Align trailing comments within a related group, such as one record or component's props, rather
than across unrelated declarations. Leave at least 2 spaces after the longest declaration with
a trailing comment in that group; properties with leading documentation do not set that column.
Include the comments when judging the 120-character line-length target; do not pad the
declarations' colons or equals signs to align them.

A trailing comment may continue onto a second line, with its `///` aligned with the first line's
`///`. If an explanation needs more than 2 trailing lines, move the entire comment above the
property as a leading documentation block. Use a leading block as well when a long declaration
leaves too little room for a readable trailing comment. Keep standalone prose documentation
around 100 characters per line where practical.

When a leading documentation block follows a field with a trailing `///` comment, put a blank
line before that block. This separates it from the previous field's documentation; without the
blank line, aligned comments continue the trailing comment and unaligned comments produce an
attachment error. See
[documentation attachment rules](/reference/syntax/comments#attachment-errors).

```nx
type Sample = {
  value: float64  /// The measured value.

  /// The interval between measurements, in seconds.
  /// Choose a value that keeps sampling responsive without
  /// overwhelming the source of the measurements.
  interval: float64
}
```

```nx
/// Encoded sRGB channels and alpha.
type RgbChannels = {
  r: float64      /// Red in [0, 255].
  g: float64      /// Green in [0, 255].
  b: float64      /// Blue in [0, 255].
  alpha: float64  /// Alpha in [0, 1].
}
```

```nx
/// A rectangle with finite coordinates and nonnegative dimensions.
type Rect = {
  x: float64 where finite                 /// Origin along the first axis.
  y: float64 where finite                 /// Origin along the second axis.
  width: float64 where finite && >= 0.0   /// Nonnegative extent along the first axis.
  height: float64 where finite && >= 0.0  /// Nonnegative extent along the second axis.
}
```

```nx
type Limits = {
  attempts: int  /// Maximum attempts before stopping; includes the initial attempt and retries.
                 /// A value of one disables retries.
}
```

## Literals

Use double quotes for strings. Prefer lowercase six-digit hex color literals for opaque colors
and eight-digit literals when alpha is needed: `#5b6ef5`, `#00000080`. Short hex forms remain
useful when they make intent clearer.

Write numbers with enough precision to preserve their intended value. Avoid unnecessary digits,
but do not round numerical constants as part of a formatting edit without verifying that their
stored value is unchanged.

## Constrained types

Write a constraint after the type with one space on each side of `where`, `&&` and each comparison,
and none inside a range: `float32 where in 0.0..=1.0`, `int where >= 1 && < 100`. Write a
floating-point bound with its decimal point, `0.0..=1.0` rather than `0..=1`, so it reads as the
type it bounds. Name a constrained type that is used more than once, and use the name:

```nx
type Ratio = float32 where in 0.0..=1.0

type Fade = { from: Ratio  to: Ratio }
```

## Applying the guide

Apply these conventions to new NX source and to declarations you edit. Keep broad formatting
changes separate from unrelated behavior changes. Formatting tools and generated source should
follow these conventions where possible; diagnostic fixtures may intentionally exercise other
valid forms or invalid syntax.
