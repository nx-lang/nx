# NX Grammar Spec (Token-Oriented, AI-Friendly)

This is the machine-oriented grammar for NX. It is:
- Non-left-recursive and left-factored (single-token lookahead chooses branches).
- Operator-free for conventional expressions; a separate precedence table drives a Pratt parser.
- Token-oriented: productions expand to token kinds (UPPER_SNAKE) or other nonterminals (CamelCase).
- AST-oriented: each rule includes its node type and fields.

Source of truth for language shape: nx-grammar.md. This spec re-expresses it in a parser-ready form.

## Token Vocabulary

Terminals are UPPER_SNAKE token kinds produced by the lexer. Lexeme hints are illustrative; actual lexing rules are defined by the lexer.

Keywords
- IMPORT ("import")
- TYPE ("type")
- LET ("let")
- IF ("if"), ELSE ("else"), END_IF ("/if")
- SWITCH ("switch"), CASE ("case"), DEFAULT ("default"), END_SWITCH ("/switch")
- FOR ("for"), IN ("in"), END_FOR ("/for")

Primitive types (keywords)
- STRING ("string")
- INT ("int"), LONG ("long")
- FLOAT ("float"), DOUBLE ("double")
- BOOLEAN ("boolean")
- VOID ("void")
- OBJECT ("object")

Identifiers and names
- IDENTIFIER (letters, digits, underscore; starts with letter/underscore)
- MARKUP_IDENTIFIER (letters, digits, underscore, hyphen)

Literals
- STRING_LITERAL
- INT_LITERAL
- REAL_LITERAL
- HEX_LITERAL
- BOOL_LITERAL (true|false)
- NULL_LITERAL (null)

 Punctuation and operators
 - LT (<), GT (>)
 - LPAREN (() , RPAREN ())
 - LBRACE ({), RBRACE (})
 - SLASH (/), COLON (:), COMMA (,), DOT (.)
 - EQ (=)
 - QMARK (?)
 - ELLIPSIS (...)
 - PLUS (+), MINUS (-), STAR (*), SLASH (/)
 - LT_EQ (<=), GT_EQ (>=), EQ_EQ (==), BANG_EQ (!=)
 - AMP_AMP (&&), PIPE_PIPE (||)

Text content tokens (inside text elements)
- TEXT_CHUNK (sequence of text chars excluding '<', '&', '{')
- ENTITY (named or numeric entity; e.g., &amp; &#10;)
- ESCAPED_LBRACE ("{{")
- ESCAPED_RBRACE ("}}")

Special
- EOF (end of file)

## Operator Precedence (Pratt)

Conventional expressions (non-markup) use a Pratt parser with the following precedence and associativity. Higher number binds tighter.

 140: Postfix call, member access
- Postfix call: led token: LPAREN … RPAREN, left-associative
  - form: callee LPAREN [Expr (COMMA Expr)*] RPAREN → Call(callee, args)
- Member access: led token: DOT IDENTIFIER, left-associative
  - form: left DOT IDENTIFIER → Member(left, name)

 130: Prefix unary, right-associative
 - Prefix minus: nud token: MINUS
   - form: MINUS Expr → PrefixUnaryExpression(op: MINUS, expr)

 120: Multiplicative, left-associative
- STAR (*), SLASH (/)

110: Additive, left-associative
- PLUS (+), MINUS (-)

90: Relational, left-associative
- LT (<), GT (>), LT_EQ (<=), GT_EQ (>=)

80: Equality, left-associative
- EQ_EQ (==), BANG_EQ (!=)

40: Logical AND, left-associative
- AMP_AMP (&&)

30: Logical OR, left-associative
- PIPE_PIPE (||)

Grouping
- LPAREN Expr RPAREN binds as a primary (handled in nud for LPAREN).

 Notes
 - Unit literal is distinct: LPAREN RPAREN → Unit.

## Grammar (Left-Factored, Token-Oriented)

Nonterminals are CamelCase. Terminals are UPPER_SNAKE tokens.

ModuleDefinition (AST: Module)
- ModuleDefinition → ImportStatement* TypeDefinition* FunctionDefinition* MainElement? EOF
  - fields: imports: Import[], types: TypeDef[], functions: FunctionDef[], main?: Element

ImportStatement (AST: Import)
- ImportStatement → IMPORT QualifiedName
  - fields: name: QualifiedName

TypeDefinition (AST: TypeDef)
- TypeDefinition → TYPE IDENTIFIER EQ Type
  - fields: name: string, type: Type

Type (AST: TypeRef)
- Type → PrimitiveType TypeOptModifier
- Type → UserDefinedType TypeOptModifier
  - fields: kind: "primitive"|"user", name: string (qualified), modifier?: "nullable"|"list"

TypeOptModifier
- TypeOptModifier → QMARK
- TypeOptModifier → ELLIPSIS
- TypeOptModifier → ε

PrimitiveType (AST: PrimitiveType)
- PrimitiveType → STRING | INT | LONG | FLOAT | DOUBLE | BOOLEAN | VOID | OBJECT
  - fields: name: "string"|"int"|"long"|"float"|"double"|"boolean"|"void"|"object"

UserDefinedType (AST: UserType)
- UserDefinedType → QualifiedName
  - fields: name: QualifiedName

FunctionDefinition (AST: FunctionDef)
- FunctionDefinition → LET LT ElementName PropertyDefinition* SLASH GT EQ Expression
  - fields: elementName: QualifiedMarkupName, props: PropDef[], body: Expression

PropertyDefinition (AST: PropDef)
- PropertyDefinition → MARKUP_IDENTIFIER COLON Type [EQ Expression]
  - fields: name: string, type: Type, default?: Expression

MainElement (AST: Element)
- MainElement → Element

Expression (AST: Expression; see mappings below)
- Expression → MarkupExpression
- Expression → Expr      (conventional expression, parsed by Pratt using the operator table)

Expr (parsed by Pratt; not a standalone AST node)
- Primaries (nud): Literal | IDENTIFIER | Unit | ParenthesizedExpression
- Postfix/infix handled via the operator table

Unit (AST: UnitLiteral)
- Unit → LPAREN RPAREN
  - fields: (none)

ParenthesizedExpression (AST: Grouped)
- ParenthesizedExpression → LPAREN Expression RPAREN
  - fields: expr: Expression

Literal (AST: Literal)
- Literal → STRING_LITERAL | INT_LITERAL | REAL_LITERAL | HEX_LITERAL | BOOL_LITERAL | NULL_LITERAL
  - fields: kind: "string"|"int"|"real"|"hex"|"bool"|"null", value: token payload

MarkupExpression (AST: MarkupList)
- MarkupExpression → MarkupExpressionItem+
  - fields: items: MarkupItem[]

MarkupExpressionItem (AST: MarkupItem is a sum type)
- MarkupExpressionItem → Element            (Element)
- MarkupExpressionItem → IfExpression       (IfExpr)
- MarkupExpressionItem → SwitchExpression   (SwitchExpr)
- MarkupExpressionItem → ForExpression      (ForExpr)

IfExpression (AST: IfExpr)
- IfExpression → IF Expression COLON Expression [ELSE COLON Expression] END_IF
  - fields: condition: Expression, thenExpr: Expression, elseExpr?: Expression

SwitchExpression (AST: SwitchExpr)
- SwitchExpression → SWITCH SwitchScrutineeOpt SwitchCase+ SwitchDefaultOpt END_SWITCH

SwitchScrutineeOpt
- SwitchScrutineeOpt → Expression
- SwitchScrutineeOpt → ε        (selected when next token is CASE or DEFAULT)
  - fields (on SwitchExpr): scrutinee?: Expression

SwitchCase (AST: SwitchCase)
- SwitchCase → CASE Pattern (COMMA Pattern)* COLON Expression
  - fields: patterns: Pattern[], expr: Expression

SwitchDefault (AST: SwitchDefault)
- SwitchDefault → DEFAULT COLON Expression
  - fields: expr: Expression

SwitchDefaultOpt
- SwitchDefaultOpt → SwitchDefault | ε

ForExpression (AST: ForExpr)
- ForExpression → FOR IDENTIFIER ForIndexOpt IN Expression COLON Expression END_FOR

ForIndexOpt
- ForIndexOpt → COMMA IDENTIFIER | ε
  - fields (on ForExpr): itemVar: string, indexVar?: string, iterable: Expression, body: Expression

Element (AST: Element)
- Element → LT ElementName PropertyArgument* ElementTail

ElementTail
- ElementTail → SLASH GT
  - fields (on Element): name: QualifiedMarkupName, props: PropArg[], children: []
- ElementTail → GT MarkupExpression LT SLASH ElementName GT
  - fields (on Element): name: QualifiedMarkupName, props: PropArg[], children: MarkupList.items

TextElement (AST: TextElement)
- TextElement → LT ElementName COLON MARKUP_IDENTIFIER PropertyArgument* GT TextContent LT SLASH ElementName GT
  - fields: name: QualifiedMarkupName, textType: string, props: PropArg[], content: TextPart[]

ElementName
- ElementName → QualifiedMarkupName

PropertyArgument (AST: PropArg)
- PropertyArgument → QualifiedMarkupName EQ Expression
  - fields: name: QualifiedMarkupName, value: Expression

TextContent (AST: TextSequence)
- TextContent → TextPart*
  - fields: parts: TextPart[]

TextPart (AST: TextPart is a sum type)
- TextPart → TextRun            (TextRun)
- TextPart → InterpolationExpression  (Interpolation)

TextRun (AST: TextRun)
- TextRun → (TEXT_CHUNK | ENTITY | ESCAPED_LBRACE | ESCAPED_RBRACE)+
  - fields: text: string  (concatenated, entities preserved or decoded by later phase)

InterpolationExpression (AST: Interpolation)
- InterpolationExpression → LBRACE Expression RBRACE
  - fields: expr: Expression

QualifiedName (AST: QualifiedName)
- QualifiedName → IDENTIFIER (DOT IDENTIFIER)*
  - fields: parts: string[]

QualifiedMarkupName (AST: QualifiedMarkupName)
- QualifiedMarkupName → IDENTIFIER (DOT MARKUP_IDENTIFIER)*
  - fields: parts: string[]

Pattern (AST: Pattern)
- Pattern → Literal
- Pattern → QualifiedName
  - fields: kind: "literal"|"name", value: Literal|QualifiedName

## AST Mapping Summary

This section lists the AST node types with fields for implementers.

- Module: imports: Import[], types: TypeDef[], functions: FunctionDef[], main?: Element
- Import: name: QualifiedName
- TypeDef: name: string, type: TypeRef
- TypeRef: kind: "primitive"|"user", name: string (qualified), modifier?: "nullable"|"list"
- PrimitiveType: name: string
- UserType: name: QualifiedName
- FunctionDef: elementName: QualifiedMarkupName, props: PropDef[], body: Expression
- PropDef: name: string, type: TypeRef, default?: Expression
- Expression: union of MarkupList | IfExpr | SwitchExpr | ForExpr | PrattExpr results
 - PrattExpr results (from operator table):
  - Call: callee: Expression, args: Expression[]
  - Member: target: Expression, name: string
  - BinaryExpression: op: token, left: Expression, right: Expression
  - PrefixUnaryExpression: op: token, expr: Expression
  - Grouped: expr: Expression (may be elided)
  - UnitLiteral
  - Literal: kind, value
  - Identifier: name: string
- MarkupList: items: MarkupItem[]
- IfExpr: condition: Expression, thenExpr: Expression, elseExpr?: Expression
- SwitchExpr: scrutinee?: Expression, cases: SwitchCase[], default?: Expression
- SwitchCase: patterns: Pattern[], expr: Expression
- SwitchDefault: expr: Expression (usually folded into SwitchExpr.default)
- ForExpr: itemVar: string, indexVar?: string, iterable: Expression, body: Expression
- Element: name: QualifiedMarkupName, props: PropArg[], children: MarkupItem[]
- TextElement: name: QualifiedMarkupName, textType: string, props: PropArg[], content: TextPart[]
- PropArg: name: QualifiedMarkupName, value: Expression
- TextSequence: parts: TextPart[]
- TextRun: text: string
- Interpolation: expr: Expression
- QualifiedName: parts: string[]
- QualifiedMarkupName: parts: string[]
- Pattern: kind: "literal"|"name", value: Literal|QualifiedName

## Disambiguation and Lookahead Notes

- Expression branch selection:
  - If next token is one of {LT, IF, SWITCH, FOR} → MarkupExpression
  - Otherwise → Expr (Pratt)
- SwitchExpression scrutinee:
  - After SWITCH, if next token ∈ {CASE, DEFAULT} → no scrutinee
  - Else → parse Expression as scrutinee
- Element is left-factored: after LT ElementName PropertyArgument*, choose SLASH GT (self-closing) or GT … LT SLASH ElementName GT (with children) by single-token lookahead at SLASH vs GT.

## Validation Rules (post-parse)

- Element closing tag name must match opening ElementName.
- TextElement closing tag name must match opening ElementName.
- PropertyDefinition names within a single FunctionDefinition should be unique.
- Type modifiers: at most one of QMARK or ELLIPSIS.
- SwitchExpression: at least one SwitchCase; patterns per case must be non-empty.

## Notes and Gaps

- Pattern is limited to constant-like forms (Literal or QualifiedName). Extend as needed.
- Entities in TextRun are preserved as ENTITY tokens; decoding can be a later phase.
- ParenthesizedExpression may be elided in AST after parsing.
