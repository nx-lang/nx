Help me plan creating a next generation markup language, called NX.

Here are high level goals:
* Based on XML and JSX/TSX, drawing from their syntax
* Pure functional language that's strongly typed
* Unlike most template languages, which combine two separate languages like XML and separate language for programmatic features (JavaScript in the case of JSX), this is a single language incorporating both markup syntax and programmatic constructs together, making for a cleaner syntax
* Properties (same as XML attributes) can have arbitrary markup for their content, not restricted to strings

Implementation:
- Conceptually, the language can be implemented in any language, but the initial implementation should be in C#.
- The implementation should support an interpreter, by generating expression trees. Later, it will also offer a transpiler option, generating C# code, but the transpiler won't be in the initial implementation.

# Core Syntax Elements

## Module Syntax

### Example 1

private let MyChildElement = <ElementName prop1="foo"/>

<MyElement prop1="foo">
  <MyChildElement/>
</MyElement>

### Example 2

private let MyChildElement = <ElementName prop1="foo"/>

let <AggregateElement prop1:int/> =
  <MyElement prop1="foo">
    <MyChildElement/>
  </MyElement>

### EBNF

ModuleDefinition ::=
    {ImportStatement} {FunctionDefinition} [Element]

## Function Definition Syntax

### Examples

let <HomeView context:PageContext/> =
  <div className="p-4">
    <div className="mb-6">
      <h1>Welcome, {context.UserName}</h1>
    </div>
  </div>

let <HomeView context:PageContext>childContent:Content</HomeView> =
  <div className="p-4">
    <div className="mb-6">
      <h1>Welcome, {context.UserName}</h1>
    </div>
    {childContent}
  </div>

### EBNF

FunctionDefinition ::=
    "let" "<" FunctionName {ParamDefinition} "/> "=" RhsExpression
    | "let" "<" FunctionName {ParamDefinition} ">" ParamDefinition "</" FunctionName ">" "=" RhsExpression

ParamDefinition ::= ParamName ":" TypeDeclaration ["=" DefaultValue]

DefaultValue ::=
    RhsExpression

## Expression Syntax

### EBNF

RhsExpression ::=
    StringLiteral | BraceExpression | Element | IfExpression | ForExpression | SwitchExpression

BraceExpression ::=
    "{" Expression "}"

Expression ::= 
    Literal | Identifier | Arithmetic | FunctionCall | Element | IfExpression | ForExpression | SwitchExpression | "(" Expression ")"

IfExpression ::=
    "if" Expression BraceExpression ["else" BraceExpression]

SwitchExpression ::=
    "switch" Expression "{" {Pattern "=>" Expression} ["_" => Expression] "}"

## Type Syntax

### EBNF

TypeDeclaration ::=
    BasicTypeDeclaration | BasicTypeDeclaration "..." | "BasicTypeDeclaration" "?"

BasicTypeDeclaration ::=
    "string" | "int" | "boolean" | TypeIdentifer

TypeIdentifier ::= Letter {Letter | Digit | "_" }

