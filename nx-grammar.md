# NX Grammar (EBNF)

This document defines the grammar for the NX markup language using Extended Backus-Naur Form (EBNF).
It's intended to be readable, human friendly. For a machine readable version used for AI code
generation, see [nx-grammar-spec.md](nx-grammar-spec.md).

<a id="module-definition"></a>
## Module Definition

```ebnf
ModuleDefinition ::=
    {ImportStatement}
    { TypeDefinition | ValueDefinition | FunctionDefinition }
    | Element

ImportStatement ::=
    "import" QualifiedName
```

<a id="types"></a>
## Types

```ebnf
TypeDefinition ::=
    "type" Identifier "=" TypeDeclaration

TypeDeclaration ::=
    PrimitiveType [TypeModifier]
    | UserDefinedType [TypeModifier]

TypeModifier ::=
    "?"             (* nullable: 0 or 1 *)
    | "[]"          (* sequence: 0 or more *)

PrimitiveType ::=
    "string"
    | "int" | "long" | "float" | "double"
    | "boolean"
    | "void"
    | "object"

UserDefinedType ::=
    QualifiedName
```

<a id="values"></a>
## Values

```ebnf
ValueDefinition ::=
    "let" IdentifierName [":" TypeDeclaration] "=" RhsExpression
```

<a id="functions"></a>
## Functions

```ebnf
FunctionDefinition ::=
    "let" "<" ElementName {PropertyDefinition} "/>" "=" RhsExpression

PropertyDefinition ::=
    MarkupIdentifier ":" TypeDeclaration ["=" RhsExpression]
```

<a id="expressions"></a>
## Expressions

```ebnf
(* Right-hand side of a property/let definition, after "=" *)
RhsExpression ::=
    Element
    | Literal
    | InterpolationExpression

InterpolationExpression  ::=
    "{" ValueExpression "}"

ValueExpression ::=
    Element
    | Literal
    | Identifier
    | ValueIfExpression
    | ValueForExpression
    | ConditionalExpression
    | PrefixUnaryExpression
    | BinaryExpression
    | MemberAccess
    | FunctionCall
    | Unit
    | ParenthesizedExpression

ConditionalExpression ::=
    ValueExpression "?" ValueExpression ":" ValueExpression    (* right-associative *)

ParenthesizedExpression ::=
    "(" ValueExpression ")"

ValueIfExpression ::=
    ValueIfSimpleExpression
    | ValueIfMatchExpression
    | ValueIfConditionListExpression

ValueIfSimpleExpression ::=
    "if" ValueExpression "{" ValueExpression "}" ["else" "{" ValueExpression "}"]

ValueIfMatchExpression ::=
    "if" [ValueExpression] "is" "{"
    {Pattern {"," Pattern} ":" ValueExpression}
    ["else" ":" ValueExpression]
    "}"

ValueIfConditionListExpression ::=
    "if" [ValueExpression] "{"
    {ValueExpression ":" ValueExpression}
    ["else" ":" ValueExpression]
    "}"

ValueForExpression ::=
    "for" {Identifier} "in" ValueExpression "{" ValueExpression "}"
    | "for" Identifier "," Identifier "in" ValueExpression "{" ValueExpression "}"  (* With index *)

PrefixUnaryExpression ::=
    "-" ValueExpression
BinaryExpression ::=
    ValueExpression ("+" | "-" | "*" | "/" | ">" | "<" | ">=" | "<=" | "==" | "!=" | "&&" | "||") ValueExpression
MemberAccess ::=
    ValueExpression "." Identifier
FunctionCall ::=
    ValueExpression "(" {ValueExpression} ")"

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
(* list of elements, with if/for allowed *)
ElementsExpression ::=
    (Element | ElementsIfExpression | ElementsForExpression)+

ElementsIfExpression ::=
    ElementsIfSimpleExpression
    | ElementsIfMatchExpression
    | ElementsIfConditionListExpression

ElementsIfSimpleExpression ::=
    "if" ValueExpression "{" ElementsExpression "}" ["else" "{" ElementsExpression "}"]

ElementsIfMatchExpression ::=
    "if" [ValueExpression] "is" "{"
    {Pattern {"," Pattern} ":" ElementsExpression}
    ["else" ":" ElementsExpression]
    "}"

ElementsIfConditionListExpression ::=
    "if" [ValueExpression] "{"
    {ValueExpression ":" ElementsExpression}
    ["else" ":" ElementsExpression]
    "}"

ElementsForExpression ::=
    "for" {Identifier} "in" ValueExpression "{" ElementsExpression "}"
    | "for" Identifier "," Identifier "in" ValueExpression "{" ElementsExpression "}"  (* With index *)

Element ::=
    "<" ElementName PropertyList "/>"
    | "<" ElementName PropertyList ">" Content "</" ElementName ">"
    | EmbedElement

EmbedElement ::=
    "<" ElementName ":" EmbedTextType PropertyList ">" EmbedContent "</" ElementName ">"
    | "<" ElementName ":" EmbedTextType "raw" PropertyList ">" RawEmbedContent "</" ElementName ">"

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
    "if" [ValueExpression] "is" "{"
    {Pattern {"," Pattern} ":" PropertyList}
    ["else" ":" PropertyList]
    "}"

PropertyListIfConditionList ::=
    "if" [ValueExpression] "{"
    {ValueExpression ":" PropertyList}
    ["else" ":" PropertyList]
    "}"

Content ::=
    ElementsExpression |     (* list of elements, with if/for allowed *)
    MixedContentExpression   (* text with optional embedded elements and interpolations; no if/for allowed except inside interpolated expressions *)

MixedContentExpression ::=
    { TextPart | Element | InterpolationExpression }

EmbedTextType ::=
    Identifier

ElementName ::=
    QualifiedMarkupName

PropertyValue ::=
    QualifiedMarkupName "=" RhsExpression

EmbedContent ::=
    { TextRun | InterpolationExpression }

RawEmbedContent ::=
    TextRun
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
   string literals or textual content (e.g., TextRun/Embed text). *)

Identifier  ::=
    (Letter | "_") { Letter | Digit | "_" }
MarkupIdentifier  ::=
    (Letter | "_") { Letter | Digit | "_" | "-" }

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
    ("e" | "E") ["+" | "-"] DigitsUnderscore

HexLiteral       ::=
    ("0x" | "0X") HexDigitsUnderscore
HexDigitsUnderscore  ::=
    HexDigit { ["_"] HexDigit }

BooleanLiteral  ::=
    "true" | "false"
NullLiteral     ::=
    "null"

TextRun          ::= { TextChar | Entity | EscapedBrace }
TextChar         ::=
    ? any character except "<", "&", and "{" ?

EscapedBrace     ::=
    "{{" | "}}"
```
