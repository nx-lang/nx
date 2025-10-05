# NX Grammar (EBNF)

This document defines the grammar for the NX markup language using Extended Backus-Naur Form (EBNF).
It's intended to be readable, human friendly. For a machine readable version used for AI code
generation, see [nx-grammar-spec.md](nx-grammar-spec.md).

<a id="module-definition"></a>
## Module Definition

```ebnf
ModuleDefinition ::=
    {ImportStatement}
        ( {TypeDefinition} {FunctionDefinition} )
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
    | "..."         (* list: 0 or more *)

PrimitiveType ::=
    "string"
    | "int" | "long" | "float" | "double"
    | "boolean"
    | "void"
    | "object"

UserDefinedType ::=
    QualifiedName
```
<a id="function-definition"></a>
## Function Definition

```ebnf
FunctionDefinition ::=
    "let" "<" ElementName {PropertyDefinition} "/>" "=" RhsExpression

PropertyDefinition ::=
    MarkupIdentifier ":" TypeDeclaration ["=" RhsExpression]
```

<a id="expressions"></a>
## Expressions

```ebnf
(* Right-hand side of a property/let definition, after "-" *)
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
    | ValueSwitchExpression
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
    "if" ValueExpression ":" ValueExpression["else" ":" ValueExpression] "/if"

ValueSwitchExpression ::=
    "switch" [ValueExpression]
    {"case" Pattern {"," Pattern} ":" ValueExpression}
    ["default" ":" ValueExpression]
    "/switch"

ValueForExpression ::=
    "for" {Identifier} "in" ValueExpression ":" ValueExpression "/for"
    | "for" Identifier "," Identifier "in" ValueExpression ":" ValueExpression "/for"  (* With index *)

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
(* list of elements, with if/switch/for allowed *)
ElementsExpression ::=
    (Element | ElementsIfExpression | ElementsSwitchExpression | ElementsForExpression)+

ElementsIfExpression ::=
    "if" ValueExpression ":" ElementsExpression ["else" ":" ElementsExpression] "/if"

ElementsSwitchExpression ::=
    "switch" [ValueExpression]
    {"case" Pattern {"," Pattern} ":" ElementsExpression}
    ["default" ":" ElementsExpression]
    "/switch"

ElementsForExpression ::=
    "for" {Identifier} "in" ValueExpression ":" ElementsExpression "/for"
    | "for" Identifier "," Identifier "in" ValueExpression ":" ElementsExpression "/for"  (* With index *)

Element ::=
    "<" ElementName PropertyList "/>"
    | "<" ElementName PropertyList ">" Content "</" ElementName ">"
    | EmbedElement

EmbedElement ::=
    "<" ElementName ":" EmbedTextType PropertyList ">" EmbedContent "</" ElementName ">"
    | "<" ElementName ":" EmbedTextType "raw" PropertyList ">" RawEmbedContent "</" ElementName ">"

(* list of properties, with if/switch allowed *)
PropertyList ::=
    {PropertyValue | PropertyListIf | PropertyListSwitch}

PropertyListIf ::=
    "if" ValueExpression ":" PropertyList ["else" ":" PropertyList] "/if"

PropertyListSwitch ::=
    "switch" [ValueExpression]
    {"case" Pattern {"," Pattern} ":" PropertyList}
    ["default" ":" PropertyList]
    "/switch"

Content ::=
    ElementsExpression |     (* list of elements, with if/switch/for allowed *)
    MixedContentExpression   (* text with optional embedded elements and interpolations; no if/switch/for allowed except inside interpolated expressions *)

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
