# NX Grammar (EBNF)

This document defines the grammar for the NX markup language using Extended Backus-Naur Form (EBNF).
It's intended to be readable, human friendly.
For a machine readable version used for AI code generation, see [nx-grammar-spec.md](nx-grammar-spec.md).

<a id="module-definition"></a>
## Module Definition

```ebnf
ModuleDefinition ::=
    {ImportStatement} {TypeDefinition} {FunctionDefinition} [MainElement]

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
    "let" "<" ElementName {PropertyDefinition} "/>" "=" Expression

PropertyDefinition ::=
    MarkupIdentifier ":" TypeDeclaration ["=" Expression]
```

<a id="expressions"></a>
## Expressions

```ebnf
Expression ::=
    MarkupExpression            (* list of elements, with if/switch/for allowed *)
    | Literal                   (* conventional expressions below *)
    | Identifier
    | IfExpression
    | SwitchExpression
    | ForExpression
    | PrefixUnaryExpression
    | BinaryExpression
    | MemberAccess
    | FunctionCall
    | Unit
    | ParenthesizedExpression

MarkupExpression ::=
    {MarkupExpressionItem}

MarkupExpressionItem ::=
    Element
    | IfExpression
    | SwitchExpression
    | ForExpression

IfExpression ::=
    "if" Expression ":" Expression ["else" ":" Expression] "/if"

SwitchExpression ::=
    "switch" Expression {SwitchCase} [SwitchDefault] "/switch"
    | "switch" {SwitchCase} [SwitchDefault] "/switch"

SwitchCase ::=
    "case" Pattern {"," Pattern} ":" Expression
SwitchDefault ::=
    "default" ":" Expression

ForExpression ::=
    "for" {Identifier} "in" Expression ":" Expression "/for"
    | "for" Identifier "," Identifier "in" Expression ":" Expression "/for"  (* With index *)

PrefixUnaryExpression ::= 
    "-" Expression
BinaryExpression ::= 
    Expression ("+" | "-" | "*" | "/" | ">" | "<" | ">=" | "<=" | "==" | "!=" | "&&" | "||") Expression
MemberAccess ::=
    Expression "." Identifier
FunctionCall ::=
    Expression "(" {Expression} ")"

Unit ::=
    "()"

ParenthesizedExpression ::=
    "(" Expression ")"

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
Element ::=
    "<" ElementName {PropertyArgument} "/>"
    | "<" ElementName {PropertyArgument} ">" MarkupExpression "</" ElementName ">"
    | TextElement

TextElement ::=
    "<" ElementName ":" TextType {PropertyArgument} ">" TextContent "</" ElementName ">"

ElementName ::=
    QualifiedMarkupName

PropertyArgument ::=
    QualifiedMarkupName "=" Expression

TextContent ::=
    { TextPart }

TextPart ::=
    TextRun
    | InterpolationExpression
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

InterpolationExpression  ::=
    "{" Expression "}"
```
