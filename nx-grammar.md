# NX Grammar (EBNF)

This document defines the grammar for the NX markup language using Extended Backus-Naur Form (EBNF).
It's intended to be readable, human friendly. For a machine readable version used for AI code
generation, see [nx-grammar-spec.md](nx-grammar-spec.md).

Note: The postfix "+" meta-operator denotes one-or-more repetitions.
Notation: "{…}" means zero-or-more, "[…]" means optional, and "(…)" denotes grouping.

<a id="module-definition"></a>
## Module Definition

```ebnf
ModuleDefinition ::=
    {ImportStatement}
    { TypeDefinition | ValueDefinition | FunctionDefinition }
    [Element]

ImportStatement ::=
    "import" QualifiedName
```

A module can mix any number of definitions with an optional trailing root `Element`, which is present when the module defines rendered markup alongside its declarations.

<a id="types"></a>
## Types

```ebnf
TypeDefinition ::=
    RecordDefinition
    | EnumDefinition
    | TypeAliasDefinition

EnumDefinition ::=
    "enum" Identifier "=" ["|"] Identifier { "|" Identifier }

RecordDefinition ::=
    "type" Identifier "=" "{"
        {PropertyDefinition}
    "}"

TypeAliasDefinition ::=
    "type" Identifier "=" TypeDeclaration

TypeDeclaration ::=
    PrimitiveType [TypeModifier]
    | UserDefinedType [TypeModifier]

TypeModifier ::=
    "?"             (* nullable: 0 or 1 *)
    | "[]"          (* sequence: 0 or more *)

PrimitiveType ::=
    "string"
    | "i32" | "i64" | "int" | "f32" | "f64" | "float"
    | "bool"
    | "void"
    | "object"

UserDefinedType ::=
    QualifiedName
```

<a id="values"></a>
## Values

```ebnf
ValueDefinition ::=
    "let" Identifier [":" TypeDeclaration] "=" RhsExpression
```

<a id="functions"></a>
## Functions

```ebnf
FunctionDefinition ::=
    ElementFunctionDefinition
    | ParenFunctionDefinition

ElementFunctionDefinition ::=
    "let" "<" ElementName {PropertyDefinition} "/>" [":" TypeDeclaration] "=" RhsExpression

ParenFunctionDefinition ::=
    "let" Identifier "(" [PropertyDefinition {"," PropertyDefinition}] ")" [":" TypeDeclaration] "=" RhsExpression

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
    | ParenFunctionCall
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
    "if" ValueExpression "is" "{"
    ( Pattern {"," Pattern} "=>" ValueExpression )+
    ["else" "=>" ValueExpression]
    "}"

ValueIfConditionListExpression ::=
    "if" "{"
    ( ValueExpression "=>" ValueExpression )+
    ["else" "=>" ValueExpression]
    "}"

ValueForExpression ::=
    "for" Identifier "in" ValueExpression "{" ValueExpression "}"
    | "for" Identifier "," Identifier "in" ValueExpression "{" ValueExpression "}"  (* With index *)

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
    ( Element | ElementsIfExpression | ElementsForExpression | InterpolationExpression )+

ElementsIfExpression ::=
    ElementsIfSimpleExpression
    | ElementsIfMatchExpression
    | ElementsIfConditionListExpression

ElementsIfSimpleExpression ::=
    "if" ValueExpression "{" ElementsExpression "}" ["else" "{" ElementsExpression "}"]

ElementsIfMatchExpression ::=
    "if" ValueExpression "is" "{"
    ( Pattern {"," Pattern} "=>" ElementsExpression )+
    ["else" "=>" ElementsExpression]
    "}"

ElementsIfConditionListExpression ::=
    "if" "{"
    ( ValueExpression "=>" ElementsExpression )+
    ["else" "=>" ElementsExpression]
    "}"

ElementsForExpression ::=
    "for" Identifier "in" ValueExpression "{" ElementsExpression "}"
    | "for" Identifier "," Identifier "in" ValueExpression "{" ElementsExpression "}"  (* With index *)

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
    ( TextRun | TextChildElement | InterpolationExpression )+

(* Text allows other text elements as children, without needing the ":" *)
TextChildElement ::=
    "<" ElementName PropertyList "/>"
    | "<" ElementName PropertyList ">" TextContent "</" ElementName ">"

EmbedTextContent ::=
    ( EmbedTextRun | EmbedInterpolationExpression )+

EmbedInterpolationExpression ::=
    "@{" ValueExpression "}"

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
