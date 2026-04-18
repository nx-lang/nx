# NX Grammar (EBNF)

This document defines the grammar for the NX markup language using Extended Backus-Naur Form (EBNF).
It's intended to be readable, human friendly. For a machine readable version used for AI code
generation, see [nx-grammar-spec.md](nx-grammar-spec.md).

Note: The postfix "+" meta-operator denotes one-or-more repetitions.
Notation: "{…}" means zero-or-more, "[…]" means optional, and "(…)" denotes grouping.

Implementation note: The NX language specification itself does not place a maximum size limit on
source files. The current implementation requires NX source files to remain under roughly 2 GB
(currently enforced as less than 2 GiB) so source offsets fit within signed 32-bit values across
language bindings. That is also a reasonable limit for other implementations.

<a id="module-definition"></a>
## Module Definition

```ebnf
ModuleDefinition ::=
    {ImportStatement}
    { ActionDefinition | TypeDefinition | ValueDefinition | FunctionDefinition | ComponentDefinition }
    [Element]

ImportStatement ::=
    WildcardImportStatement
    | SelectiveImportStatement

WildcardImportStatement ::=
    "import" LibraryPath ["as" Identifier]

SelectiveImportStatement ::=
    "import" SelectiveImportList "from" LibraryPath

SelectiveImportList ::=
    "{" [SelectiveImport {"," SelectiveImport} [","]] "}"

SelectiveImport ::=
    Identifier ["as" QualifiedImportAlias]

QualifiedImportAlias ::=
    Identifier "." Identifier

LibraryPath ::=
    StringLiteral
```

A module can include any number of imports, followed by top-level declarations and an optional
trailing root `Element` for rendered markup. Imports target libraries rather than individual source
files. A local library is a directory containing `.nx` files, and every `.nx` file under that
directory contributes declarations to the imported library recursively.

Imports introduce unqualified names by default. `import "<library>" as Prefix` keeps imported names
under `Prefix.Name`, while `import { Name as Prefix.Name } from "<library>"` adds a qualified
prefix for just that imported declaration. The qualified selective alias must contain exactly one
dot, and its final identifier must match the imported declaration name.

Declarations are internal by default. `private` restricts a declaration to its defining file, and
`export` makes it visible to external consumers as well as to other files in the same library. In
non-library program builds, default visibility remains visible throughout the same program, while
`export` has no additional external effect until the code is consumed as a library.

<a id="types"></a>
## Types

```ebnf
TypeDefinition ::=
    RecordDefinition
    | EnumDefinition
    | TypeAliasDefinition

EnumDefinition ::=
    [VisibilityModifier] "enum" Identifier "=" ["|"] Identifier { "|" Identifier }

RecordDefinition ::=
    [VisibilityModifier] ["abstract"] "type" Identifier ["extends" QualifiedName] "=" "{"
        {PropertyDefinition}
    "}"

TypeAliasDefinition ::=
    [VisibilityModifier] "type" Identifier "=" TypeDeclaration

ActionDefinition ::=
    [VisibilityModifier] ["abstract"] "action" Identifier ["extends" QualifiedName] "=" "{"
        {PropertyDefinition}
    "}"

VisibilityModifier ::=
    "private"
    | "export"

TypeDeclaration ::=
    PrimitiveType {TypeSuffix}
    | UserDefinedType {TypeSuffix}

TypeSuffix ::=
    "?"             (* nullable wrapper *)
    | "[]"          (* sequence/list wrapper *)

PrimitiveType ::=
    "string"
    | "i32" | "i64" | "int" | "f32" | "f64" | "float"
    | "bool"
    | "void"
    | "object"

UserDefinedType ::=
    QualifiedName
```

Type suffixes compose in source order. `string?[]` means a list of nullable strings, while
`string[]?` means a nullable list of strings.
A nullable suffix may only be applied once per outer type layer. `string?[]?` is valid because
`[]` introduces a new list layer before the final `?`, while `string?[]??` is rejected during
post-parse validation as a redundant nullable suffix.

Record and action declarations reserve the `abstract` and `extends` keywords. `abstract type Name = { ... }`
declares a non-instantiable record root, `abstract type Name extends Base = { ... }` declares an
abstract derived record, and `type Name extends Base = { ... }` declares a concrete derived record.
Only abstract records and actions may appear in the `extends` clause.

Action inheritance example:

```nx
abstract action InputAction = {
  source: string
}

action ValueChanged extends InputAction = {
  value: string
}
```

<a id="values"></a>
## Values

```ebnf
ValueDefinition ::=
    [VisibilityModifier] "let" Identifier [":" TypeDeclaration] "=" RhsExpression
```

<a id="functions"></a>
## Functions

```ebnf
FunctionDefinition ::=
    ElementFunctionDefinition
    | ParenFunctionDefinition

ElementFunctionDefinition ::=
    [VisibilityModifier] "let" "<" ElementName {PropertyDefinition} "/>" [":" TypeDeclaration] "=" RhsExpression

ParenFunctionDefinition ::=
    [VisibilityModifier] "let" Identifier "(" [PropertyDefinition {"," PropertyDefinition}] ")" [":" TypeDeclaration] "=" RhsExpression

PropertyDefinition ::=
    ["content"] MarkupIdentifier ":" TypeDeclaration ["=" RhsExpression]
```

## Components

```ebnf
ComponentDefinition ::=
    [VisibilityModifier] "component" ComponentSignature "=" ComponentBody

ComponentSignature ::=
    "<" ElementName {PropertyDefinition} [EmitsGroup] "/>"

EmitsGroup ::=
    "emits" "{"
        EmitDefinition+
    "}"

EmitDefinition ::=
    Identifier ["extends" QualifiedName] "{"
        {PropertyDefinition}
    "}"

ComponentBody ::=
    "{"
        [StateGroup]
        ValueExpression
    "}"

StateGroup ::=
    "state" "{"
        {PropertyDefinition}
    "}"
```

Components are distinct from `let` functions. They keep the element-shaped signature syntax, but
they can also declare emitted action payloads in `emits` and persistent local state fields in
`state`. The `emits` block must contain at least one `EmitDefinition`, each emit name is a plain
identifier, and each emit/state field uses the same `PropertyDefinition` shape as other record-like
members. Inline emitted actions may optionally include an `extends` clause but remain concrete.

Inline emitted action inheritance example:

```nx
abstract action InputAction = {
  source: string
}

component <SearchBox emits { ValueChanged extends InputAction { value: string } } /> = {
  <TextInput />
}
```

Within a single record, function signature, component signature, emitted action payload, or state
block, at most one `PropertyDefinition` may be prefixed by `content`. That content-marked property
receives element body content during markup-style invocation.

<a id="expressions"></a>
## Expressions

```ebnf
(* Right-hand side of a property/let definition, after "=" *)
RhsExpression ::=
    Element
    | Literal
    | ValuesBracedExpression

(* A braced expression can be a single value or muliple, space delimited *)
ValuesBracedExpression ::=
    "{" ValueExpressions "}"

ValuesExpression ::=
    ( ValueExpression | (ValueListItemExpression)+ )

(* Expressions that can appear in a space delimited list; other expressions need to have parens *)
ValueListItemExpression ::=
    Element
    | Literal
    | Identifier
    | ValueIfExpression
    | ValueForExpression
    | MemberAccess
    | ParenFunctionCall
    | Unit
    | ParenthesizedExpression

ValueExpression ::=
    ValueListItemExpression
    | ConditionalExpression
    | PrefixUnaryExpression
    | BinaryExpression

ValueOrValuesBracedExpression ::=
    ( ValueExpression | ValuesBracedExpression )

ConditionalExpression ::=
    ValueExpression "?" ValueExpression ":" ValueExpression    (* right-associative *)

ParenthesizedExpression ::=
    "(" ValueExpression ")"

ValueIfExpression ::=
    ValueIfSimpleExpression
    | ValueIfMatchExpression
    | ValueIfConditionListExpression

ValueIfSimpleExpression ::=
    "if" ValueExpression ValuesBracedExpression ["else" ValuesBracedExpression]

ValueIfMatchExpression ::=
    "if" ValueExpression "is" "{"
    ( Pattern {"," Pattern} "=>" ValueOrValuesBracedExpression )+
    ["else" "=>" ValueOrValuesBracedExpression]
    "}"

ValueIfConditionListExpression ::=
    "if" "{"
    ( ValueExpression "=>" ValueOrValuesBracedExpression )+
    ["else" "=>" ValueOrValuesBracedExpression]
    "}"

ValueForExpression ::=
    "for" Identifier "in" ValueExpression ValuesBracedExpression
    | "for" Identifier "," Identifier "in" ValueExpression ValuesBracedExpression  (* With index *)

PrefixUnaryExpression ::=
    ( "-" | "!" ) ValueExpression
BinaryExpression ::=
    ValueExpression ( "+" | "-" | "*" | "/" | "%" | ">" | "<" | ">=" | "<=" | "==" | "!=" | "&&" | "||" ) ValueExpression
MemberAccess ::=
    ValueExpression "." Identifier  (* includes both property/field access and enum member access; semantic analysis distinguishes *)
ParenFunctionCall ::=
    ValueExpression "(" [ ValueExpression { "," ValueExpression } ] ")"

Pattern ::=
    Literal | QualifiedName

Unit ::=
    "()"

Literal ::=
    StringLiteral
    | IntegerLiteral
    | RealLiteral
    | HexLiteral
    | BooleanLiteral
    | NullLiteral

```

<a id="elements"></a>
## Elements

```ebnf
(* list of elements, with if/for and interpolations allowed *)
ElementsExpression ::=
    ( Element | ElementsIfExpression | ElementsForExpression | ValuesBracedExpression )+

ElementsBracedExpression ::=
    "{" ElementsExpression "}"

ElementOrElementsBracedExpression ::=
    Element | ElementBracedExpression

ElementsIfExpression ::=
    ElementsIfSimpleExpression
    | ElementsIfMatchExpression
    | ElementsIfConditionListExpression

ElementsIfSimpleExpression ::=
    "if" ValueExpression ElementsBracedExpression ["else" ElementsBracedExpression]

ElementsIfMatchExpression ::=
    "if" ValueExpression "is" "{"
    ( Pattern {"," Pattern} "=>" ElementOrElementsBracedExpression )+
    ["else" "=>" ElementOrElementsBracedExpression]
    "}"

ElementsIfConditionListExpression ::=
    "if" "{"
    ( ValueExpression "=>" ElementOrElementsBracedExpression )+
    ["else" "=>" ElementOrElementsBracedExpression]
    "}"

ElementsForExpression ::=
    "for" Identifier "in" ValueExpression ElementsBracedExpression
    | "for" Identifier "," Identifier "in" ValueExpression ElementsBracedExpression  (* With index *)

Element ::=
    "<" ElementName PropertyList "/>"
    | "<" ElementName PropertyList ">" ElementsExpression "</" ElementName ">"
    | TextElement

TextElement ::=
    "<" ElementName ":" PropertyList ">" TextContent "</" ElementName ">"
    | "<" ElementName ":" "raw" PropertyList ">" RawTextRun "</" ElementName ">"
    | "<" ElementName ":" TextType PropertyList ">" EmbedTextContent "</" ElementName ">"
    | "<" ElementName ":" TextType "raw" PropertyList ">" RawTextRun "</" ElementName ">"

(* list of properties, with if allowed *)
PropertyList ::=
    {PropertyValue | PropertyListIf}

PropertyListIf ::=
    PropertyListIfSimple
    | PropertyListIfMatch
    | PropertyListIfConditionList

PropertyListIfSimple ::=
    "if" ValueExpression "{" PropertyList "}" ["else" "{" PropertyList "}"]

PropertyListIfMatch ::=
    "if" ValueExpression "is" "{"
    ( Pattern {"," Pattern} "=>" PropertyList )+
    ["else" "=>" PropertyList]
    "}"

PropertyListIfConditionList ::=
    "if" "{"
    ( ValueExpression "=>" PropertyList )+
    ["else" "=>" PropertyList]
    "}"

TextContent ::=
    ( TextRun | TextChildElement | ValuesBracedExpression )+

(* Text allows other text elements as children, without needing the ":" *)
TextChildElement ::=
    "<" ElementName PropertyList "/>"
    | "<" ElementName PropertyList ">" TextContent "</" ElementName ">"

EmbedTextContent ::=
    ( EmbedTextRun | EmbedBracedExpression )+

EmbedBracedExpression ::=
    "@{" (ValueExpression)+ "}"

TextType ::=
    Identifier

ElementName ::=
    QualifiedMarkupName

PropertyValue ::=
    QualifiedMarkupName "=" RhsExpression

```

<a id="lexical-structure"></a>
## Lexical Structure

```ebnf
Letter      ::=
    "A" ... "Z" | "a" ... "z"
Digit       ::=
    "0" ... "9"
HexDigit    ::=
    Digit | "A" ... "F" | "a" ... "f"
Whitespace  ::=
    " " | "\t" | "\r" | "\n"

(* Comments are treated as whitespace/trivia and ignored by the parser. *)
Comment              ::= LineComment | BlockComment
LineComment          ::= "//" { ? any character except "\r" and "\n" ? } [ "\r" | "\n" ]

(* Block comments support nesting of the same kind. *)
BlockComment         ::= CBlockComment | HtmlBlockComment
CBlockComment        ::= "/*" CBlockContent "*/"
HtmlBlockComment     ::= "<!--" HtmlBlockContent "-->"
CBlockContent        ::= { CBlockChar | CBlockComment }        (* allows nested /* */ *)
HtmlBlockContent     ::= { HtmlBlockChar | HtmlBlockComment }  (* allows nested <!-- --> *)
CBlockChar           ::= ? any character including newline, except "/*" and "*/" ?
HtmlBlockChar        ::= ? any character including newline, except "<!--" and "-->" ?

(* Notes: Comments may appear anywhere whitespace is allowed, but are not recognized inside
   string literals or textual content (e.g., TextRun/Text elements). *)

Identifier  ::=
    ( Letter | "_" ) { Letter | Digit | "_" }
MarkupIdentifier  ::=
    ( Letter | "_" ) { Letter | Digit | "_" | "-" }

QualifiedName  ::=
    Identifier { "." Identifier }
QualifiedMarkupName  ::=
    Identifier { "." MarkupIdentifier }

Entity      ::=
    NamedEntity | NumericEntity

NamedEntity      ::=
    "&" EntityName ";"
EntityName       ::=
    "lt" | "gt" | "amp" | "quot" | "apos" | "lbrace" | "rbrace" | "nbsp"

NumericEntity    ::=
    "&#" Digits ";" | "&#x" HexDigits ";"
Digits           ::=
    Digit { Digit }
DigitsUnderscore ::=
    Digit { ["_"] Digit }
HexDigits        ::=
    HexDigit { HexDigit }

StringLiteral ::=
    '"' { StringCharDoubleQuoted | Entity } '"'
    "'" { StringCharSingleQuoted | Entity } "'"
StringCharDoubleQuoted ::=
    ? any character except '"' and "&" ?
StringCharSingleQuoted ::=
    ? any character except "'" and "&" ?

IntegerLiteral   ::=
    DigitsUnderscore
RealLiteral      ::=
    DigitsUnderscore "." DigitsUnderscore [ExponentPart]
    | DigitsUnderscore ExponentPart
ExponentPart     ::=
    ( "e" | "E" ) [ "+" | "-" ] DigitsUnderscore

HexLiteral       ::=
    ( "0x" | "0X" ) HexDigitsUnderscore
HexDigitsUnderscore  ::=
    HexDigit { ["_"] HexDigit }

BooleanLiteral  ::=
    "true" | "false"
NullLiteral     ::=
    "null"

TextRun          ::= ( TextChar | Entity | EscapedBrace )+
EmbedTextRun     ::= ( TextChar | Entity | EscapedBrace | EscapedAtSign )+

TextChar         ::=
    ? any character except "<", "&", and "{" ?

RawTextRun          ::= ( RawTextChar )+
RawTextChar         ::=
     ? any character other than '<'; the sequence '</' terminates the run ?

EscapedBrace     ::=
    "\{" | "\}"
    ? "\{" and "\}" sequences are treated as escapes; other backslashes remain literal. ?

EscapedAtSign    ::=
    "\@"
    ? "\@" is treated as an escape; other backslashes remain literal. ?
```
