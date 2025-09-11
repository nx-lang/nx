# NX Grammar (EBNF)

<a id="module-definition"></a>
## Module Definition

```ebnf
ModuleDefinition ::=
    {ImportStatement} {TypeDefinition} {FunctionDefinition} [MainElement]

ImportStatement ::=
    "import" (ImportAll | ImportSpecific) "from" StringLiteral

ImportAll ::= "*" ["as" Identifier]

ImportSpecific ::= "{" ImportItem {"," ImportItem} "}"

ImportItem ::= Identifier ["as" Identifier]

TypeDefinition ::=
    "type" Identifier "=" TypeDeclaration
```

<a id="types"></a>
## Types

```ebnf
TypeDeclaration ::=
    PrimitiveType [TypeModifier]
    | UserDefinedType [TypeModifier]

TypeModifier ::=
    "?"             (* nullable: 0 or 1 *)
    | "..."         (* list: 0 or more *)

PrimitiveType ::= "string"
                  | "int" | "long" | "float" | "double"
                  | "boolean"
                  | "void"
                  | "object"
                  | "uitext" | "text"

UserDefinedType ::= TypeIdentifier

TypeIdentifier ::= Identifier
```
<a id="function-definition"></a>
## Function Definition

```ebnf
FunctionDefinition  ::=
    "let" "<" ElementName {PropertyDefinition} "/>" "=" Expression

PropertyDefinition ::= Identifier ":" TypeDeclaration ["=" Expression]
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
    | ArithmeticExpression
    | ComparisonExpression
    | LogicalExpression
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

SwitchCase ::= "case" Pattern {"," Pattern} ":" Expression
SwitchDefault ::= "default" ":" Expression

ForExpression ::=
    "for" {Identifier} "in" Expression ":" Expression "/for"
    | "for" Identifier "," Identifier "in" Expression ":" Expression "/for"  (* With index *)

QualifiedName ::= Identifier { "." Identifier }

ArithmeticExpression ::= Expression ("+" | "-" | "*" | "/") Expression
ComparisonExpression ::= Expression (">" | "<" | ">=" | "<=" | "==" | "!=") Expression
LogicalExpression ::= Expression ("&&" | "||") Expression
MemberAccess ::= Expression "." Identifier
FunctionCall ::= Expression "(" {Expression} ")"

Unit ::= "()"

ParenthesizedExpression ::= "(" Expression ")"

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

ElementName ::= QualifiedName

PropertyArgument ::=
    QualifiedName "=" Expression

TextContent ::=
    { TextPart }

TextPart ::=
    TextRun
    | InterpolationExpression

TextRun  ::= { TextChar | Entity | EscapedBrace }
```

<a id="core-tokens"></a>
## Core Tokens

```ebnf
Letter      ::= "A" ... "Z" | "a" ... "z"
Digit       ::= "0" ... "9"
HexDigit    ::= Digit | "A" ... "F" | "a" ... "f"
Whitespace  ::= " " | "\t" | "\r" | "\n"

Identifier  ::= (Letter | "_") { Letter | Digit | "_" }
MarkupIdentifier  ::= (Letter | "_") { Letter | Digit | "_" | "-" }

Entity      ::= NamedEntity | NumericEntity

NamedEntity      ::= "&" EntityName ";"
EntityName       ::= "lt" | "gt" | "amp" | "quot" | "apos" | "lbrace" | "rbrace" | "nbsp"

NumericEntity    ::= "&#" Digits ";" | "&#x" HexDigits ";"
Digits           ::= Digit { Digit }
HexDigits        ::= HexDigit { HexDigit }

TextChar         ::= ? any character except "<", "&", and "{" ?

EscapedBrace     ::= "{{" | "}}"

InterpolationExpression  ::= "{" Expression "}"
```
