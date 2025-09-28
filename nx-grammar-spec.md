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
- RAW ("raw")

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
- ModuleDefinition → ImportStatement* (TypeDefinition* FunctionDefinition* | Element) EOF
  - fields: imports: Import[], types: TypeDef[], functions: FunctionDef[], moduleElement?: Element
    (either types/functions or moduleElement is present, not both)

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
- FunctionDefinition → LET LT ElementName PropertyDefinition* SLASH GT EQ RhsExpression
  - fields: elementName: QualifiedMarkupName, props: PropDef[], body: Expression

PropertyDefinition (AST: PropDef)
- PropertyDefinition → MARKUP_IDENTIFIER COLON Type [EQ RhsExpression]
  - fields: name: string, type: Type, default?: Expression



RhsExpression (AST: Expression; see mappings below)
- RhsExpression → Element
- RhsExpression → Literal
- RhsExpression → InterpolationExpression

InterpolationExpression (AST: Interpolation)
- InterpolationExpression → LBRACE ValueExpression RBRACE
  - fields: expr: Expression

ValueExpression (AST: Expression; Pratt-parsed for operators)
- ValueExpression → Element
- ValueExpression → ValueIfExpression
- ValueExpression → ValueSwitchExpression
- ValueExpression → ValueForExpression
- ValueExpression → ValueExpr

ValueExpr (parsed by Pratt; not a standalone AST node)
- Primaries (nud): Literal | IDENTIFIER | Unit | ParenthesizedExpression
- Postfix/infix handled via the operator table

Unit (AST: UnitLiteral)
- Unit → LPAREN RPAREN
  - fields: (none)

ParenthesizedExpression (AST: Grouped)
- ParenthesizedExpression → LPAREN ValueExpression RPAREN
  - fields: expr: Expression

Literal (AST: Literal)
- Literal → STRING_LITERAL | INT_LITERAL | REAL_LITERAL | HEX_LITERAL | BOOL_LITERAL | NULL_LITERAL
  - fields: kind: "string"|"int"|"real"|"hex"|"bool"|"null", value: token payload

ValueIfExpression (AST: ValueIfExpr)
- ValueIfExpression → IF ValueExpression COLON ValueExpression [ELSE COLON ValueExpression] END_IF
  - fields: condition: Expression, thenExpr: Expression, elseExpr?: Expression

ValueSwitchExpression (AST: ValueSwitchExpr)
- ValueSwitchExpression → SWITCH ValueSwitchScrutineeOpt ValueSwitchCase+ ValueSwitchDefaultOpt END_SWITCH
  - fields: scrutinee?: Expression, cases: ValueSwitchCase[], default?: Expression

ValueSwitchScrutineeOpt
- ValueSwitchScrutineeOpt → ValueExpression
- ValueSwitchScrutineeOpt → ε        (selected when next token is CASE or DEFAULT)
  - fields (on ValueSwitchExpr): scrutinee?: Expression

ValueSwitchCase (AST: ValueSwitchCase)
- ValueSwitchCase → CASE Pattern (COMMA Pattern)* COLON ValueExpression
  - fields: patterns: Pattern[], expr: Expression

ValueSwitchDefault (AST: ValueSwitchDefault)
- ValueSwitchDefault → DEFAULT COLON ValueExpression
  - fields: expr: Expression

ValueSwitchDefaultOpt
- ValueSwitchDefaultOpt → ValueSwitchDefault | ε

ValueForExpression (AST: ValueForExpr)
- ValueForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression COLON ValueExpression END_FOR
  - fields: itemVar: string, indexVar?: string, iterable: Expression, body: Expression

ForIndexOpt
- ForIndexOpt → COMMA IDENTIFIER | ε
  - fields (on ValueForExpr/MarkupForExpr): itemVar: string, indexVar?: string

ElementsExpression (AST: MarkupList)
- ElementsExpression → ElementsExpressionItem+
  - fields: items: MarkupItem[]

ElementsExpressionItem (AST: MarkupItem is a sum type)
- ElementsExpressionItem → Element                     (Element)
- ElementsExpressionItem → ElementsIfExpression        (MarkupIfExpr)
- ElementsExpressionItem → ElementsSwitchExpression    (MarkupSwitchExpr)
- ElementsExpressionItem → ElementsForExpression       (MarkupForExpr)

ElementsIfExpression (AST: MarkupIfExpr)
- ElementsIfExpression → IF ValueExpression COLON ElementsExpression [ELSE COLON ElementsExpression] END_IF
  - fields: condition: Expression, thenElements: MarkupList, elseElements?: MarkupList

ElementsSwitchExpression (AST: MarkupSwitchExpr)
- ElementsSwitchExpression → SWITCH ValueSwitchScrutineeOpt ElementsSwitchCase+ ElementsSwitchDefaultOpt END_SWITCH
  - fields: scrutinee?: Expression, cases: MarkupSwitchCase[], default?: MarkupList

ElementsSwitchCase (AST: MarkupSwitchCase)
- ElementsSwitchCase → CASE Pattern (COMMA Pattern)* COLON ElementsExpression
  - fields: patterns: Pattern[], elements: MarkupList

ElementsSwitchDefault (AST: MarkupSwitchDefault)
- ElementsSwitchDefault → DEFAULT COLON ElementsExpression
  - fields: elements: MarkupList

ElementsSwitchDefaultOpt
- ElementsSwitchDefaultOpt → ElementsSwitchDefault | ε

ElementsForExpression (AST: MarkupForExpr)
- ElementsForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression COLON ElementsExpression END_FOR
  - fields: itemVar: string, indexVar?: string, iterable: Expression, body: MarkupList

Element (AST: MarkupElement is a sum type)
- Element → LT ElementName ElementSuffix

ElementSuffix (builds either Element or EmbedElement AST)
- ElementSuffix → PropertyArgument* RegularElementTail
- ElementSuffix → COLON EmbedTextType EmbedElementTail

RegularElementTail (AST: Element)
- RegularElementTail → SLASH GT
  - fields: name (from ElementName), props: PropArg[], children: []
- RegularElementTail → GT Content LT SLASH ElementName GT
  - fields: name (from ElementName), props: PropArg[], children: ElementContent.items

EmbedElementTail (AST: EmbedElement)
- EmbedElementTail → PropertyArgument* GT EmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "parsed", props: PropArg[], content: EmbedContent.items
- EmbedElementTail → RAW PropertyArgument* GT RawEmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "raw", props: PropArg[], content: RawEmbedContent.text

ElementName
- ElementName → QualifiedMarkupName

EmbedTextType
- EmbedTextType → IDENTIFIER
  - fields (on EmbedElement): textType: string

PropertyArgument (AST: PropArg)
- PropertyArgument → QualifiedMarkupName EQ RhsExpression
  - fields: name: QualifiedMarkupName, value: Expression

Content (AST: ElementContent is a sum type)
- Content → ElementsExpression
- Content → MixedContentExpression
  - fields: items: MarkupItem[]

MixedContentExpression (AST: MixedContent)
- MixedContentExpression → MixedContentItem*
  - fields: items: MixedContentItem[]

MixedContentItem (AST: MixedContentItem is a sum type)
- MixedContentItem → TextPart        (TextRun)
- MixedContentItem → Element         (MarkupElement)
- MixedContentItem → InterpolationExpression (Interpolation)

EmbedContent (AST: EmbedContent)
- EmbedContent → EmbedContentItem*
  - fields: items: EmbedContentItem[]

EmbedContentItem (AST: EmbedContentItem is a sum type)
- EmbedContentItem → TextRun         (TextRun)
- EmbedContentItem → InterpolationExpression (Interpolation)

RawEmbedContent (AST: RawEmbedContent)
- RawEmbedContent → TextRun
  - fields: text: string

TextPart (AST: TextRun)
- TextPart → TextRun
  - fields: text: string

TextRun (AST: TextRun)
- TextRun → (TEXT_CHUNK | ENTITY | ESCAPED_LBRACE | ESCAPED_RBRACE)+
  - fields: text: string  (concatenated, entities preserved or decoded by later phase)

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

- Module: imports: Import[], types: TypeDef[], functions: FunctionDef[], moduleElement?: Element
- Import: name: QualifiedName
- TypeDef: name: string, type: TypeRef
- TypeRef: kind: "primitive"|"user", name: string (qualified), modifier?: "nullable"|"list"
- PrimitiveType: name: string
- UserType: name: QualifiedName
- FunctionDef: elementName: QualifiedMarkupName, props: PropDef[], body: Expression
- PropDef: name: string, type: TypeRef, default?: Expression
- Expression: union of Element | Literal | Identifier | ValueIfExpr | ValueSwitchExpr | ValueForExpr | Call | Member | BinaryExpression | PrefixUnaryExpression | Grouped | UnitLiteral
 - Call: callee: Expression, args: Expression[]
 - Member: target: Expression, name: string
 - BinaryExpression: op: token, left: Expression, right: Expression
 - PrefixUnaryExpression: op: token, expr: Expression
 - Grouped: expr: Expression (may be elided)
 - UnitLiteral
 - Literal: kind, value
 - Identifier: name: string
- ValueIfExpr: condition: Expression, thenExpr: Expression, elseExpr?: Expression
- ValueSwitchExpr: scrutinee?: Expression, cases: ValueSwitchCase[], default?: Expression
- ValueSwitchCase: patterns: Pattern[], expr: Expression
- ValueSwitchDefault: expr: Expression (usually folded into ValueSwitchExpr.default)
- ValueForExpr: itemVar: string, indexVar?: string, iterable: Expression, body: Expression
- MarkupList: items: MarkupItem[]
- MarkupItem: Element | MarkupIfExpr | MarkupSwitchExpr | MarkupForExpr
- MarkupIfExpr: condition: Expression, thenElements: MarkupList, elseElements?: MarkupList
- MarkupSwitchExpr: scrutinee?: Expression, cases: MarkupSwitchCase[], default?: MarkupList
- MarkupSwitchCase: patterns: Pattern[], elements: MarkupList
- MarkupSwitchDefault: elements: MarkupList (usually folded into MarkupSwitchExpr.default)
- MarkupForExpr: itemVar: string, indexVar?: string, iterable: Expression, body: MarkupList
- Element: name: QualifiedMarkupName, props: PropArg[], children: ElementContent (MarkupList or MixedContent)
- EmbedElement: name: QualifiedMarkupName, textType: string, mode: "parsed"|"raw", props: PropArg[], content: EmbedContent|RawEmbedContent
- PropArg: name: QualifiedMarkupName, value: Expression
- ElementContent: items: MarkupItem[] | MixedContentItem[]
- MixedContent: items: MixedContentItem[]
- MixedContentItem: kind: "text"|"element"|"interpolation", value: TextRun|Element|EmbedElement|Interpolation
- EmbedContent: items: (TextRun|Interpolation)[]
- RawEmbedContent: text: string
- TextRun: text: string
- Interpolation: expr: Expression
- QualifiedName: parts: string[]
- QualifiedMarkupName: parts: string[]
- Pattern: kind: "literal"|"name", value: Literal|QualifiedName

## Disambiguation and Lookahead Notes

- ValueExpression branch selection:
  - If next token is LT → Element
  - If next token is IF → ValueIfExpression
  - If next token is SWITCH → ValueSwitchExpression
  - If next token is FOR → ValueForExpression
  - Otherwise → ValueExpr (Pratt)
- ElementsExpression item selection:
  - If next token is LT → Element
  - If next token ∈ {IF, SWITCH, FOR} → the corresponding Elements* form
- SWITCH scrutinee (value and elements variants):
  - After SWITCH, if next token ∈ {CASE, DEFAULT} → no scrutinee
  - Else → parse ValueExpression as the scrutinee
- Element is left-factored: after LT ElementName, COLON selects the embed branch; otherwise parse PropertyArgument* and choose SLASH GT (self-closing) or GT … LT SLASH ElementName GT using lookahead at SLASH vs GT.

## Validation Rules (post-parse)

- Element closing tag name must match opening ElementName.
- EmbedElement closing tag name must match opening ElementName.
- PropertyDefinition names within a single FunctionDefinition should be unique.
- Type modifiers: at most one of QMARK or ELLIPSIS.
- Switch expressions (value or elements variants): at least one case; patterns per case must be non-empty.

## Notes and Gaps

- Pattern is limited to constant-like forms (Literal or QualifiedName). Extend as needed.
- Entities in TextRun are preserved as ENTITY tokens; decoding can be a later phase.
- ParenthesizedExpression may be elided in AST after parsing.
