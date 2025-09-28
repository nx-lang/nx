# NX Grammar Spec (Token-Oriented, AI-Friendly)

This is the machine-oriented grammar for NX. It is:
- Non-left-recursive and left-factored (single-token lookahead chooses branches).
- Operator-free for conventional expressions; a separate precedence table drives a Pratt parser.
- Token-oriented: productions expand to token kinds (UPPER_SNAKE) or other nonterminals (CamelCase).
- AST-oriented: each rule includes its node type and fields.
- Naming follows Roslyn conventions: AST node types use the `Syntax` suffix (e.g., `FunctionDefinitionSyntax`, `ValueIfExpressionSyntax`).

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

ModuleDefinition (AST: ModuleDefinitionSyntax)
- ModuleDefinition → ImportStatement* (TypeDefinition* FunctionDefinition* | Element) EOF
  - fields: imports: ImportStatementSyntax[], types: TypeDefinitionSyntax[], functions: FunctionDefinitionSyntax[], moduleElement?: MarkupElementSyntax
    (either types/functions or moduleElement is present, not both)

ImportStatement (AST: ImportStatementSyntax)
- ImportStatement → IMPORT QualifiedName
  - fields: name: QualifiedNameSyntax

TypeDefinition (AST: TypeDefinitionSyntax)
- TypeDefinition → TYPE IDENTIFIER EQ Type
  - fields: name: string, type: TypeSyntax

Type (AST: TypeSyntax)
- Type → PrimitiveType TypeOptModifier
- Type → UserDefinedType TypeOptModifier
  - fields: kind: "primitive"|"user", name: string (qualified), modifier?: "nullable"|"list"

TypeOptModifier
- TypeOptModifier → QMARK
- TypeOptModifier → ELLIPSIS
- TypeOptModifier → ε

PrimitiveType (AST: PrimitiveTypeSyntax)
- PrimitiveType → STRING | INT | LONG | FLOAT | DOUBLE | BOOLEAN | VOID | OBJECT
  - fields: name: "string"|"int"|"long"|"float"|"double"|"boolean"|"void"|"object"

UserDefinedType (AST: UserTypeSyntax)
- UserDefinedType → QualifiedName
  - fields: name: QualifiedNameSyntax

FunctionDefinition (AST: FunctionDefinitionSyntax)
- FunctionDefinition → LET LT ElementName PropertyDefinition* SLASH GT EQ RhsExpression
  - fields: elementName: QualifiedMarkupNameSyntax, props: PropertyDefinitionSyntax[], body: ExpressionSyntax

PropertyDefinition (AST: PropertyDefinitionSyntax)
- PropertyDefinition → MARKUP_IDENTIFIER COLON Type [EQ RhsExpression]
  - fields: name: string, type: TypeSyntax, default?: ExpressionSyntax



RhsExpression (AST: ExpressionSyntax; see mappings below)
- RhsExpression → Element
- RhsExpression → Literal
- RhsExpression → InterpolationExpression

InterpolationExpression (AST: InterpolationExpressionSyntax)
- InterpolationExpression → LBRACE ValueExpression RBRACE
  - fields: expr: ExpressionSyntax

ValueExpression (AST: ExpressionSyntax; Pratt-parsed for operators)
- ValueExpression → Element
- ValueExpression → ValueIfExpression
- ValueExpression → ValueSwitchExpression
- ValueExpression → ValueForExpression
- ValueExpression → ValueExpr

ValueExpr (parsed by Pratt; not a standalone AST node)
- Primaries (nud): Literal | IDENTIFIER | Unit | ParenthesizedExpression
- Postfix/infix handled via the operator table

Unit (AST: UnitLiteralSyntax)
- Unit → LPAREN RPAREN
  - fields: (none)

ParenthesizedExpression (AST: ParenthesizedExpressionSyntax)
- ParenthesizedExpression → LPAREN ValueExpression RPAREN
  - fields: expr: ExpressionSyntax

Literal (AST: LiteralExpressionSyntax)
- Literal → STRING_LITERAL | INT_LITERAL | REAL_LITERAL | HEX_LITERAL | BOOL_LITERAL | NULL_LITERAL
  - fields: kind: "string"|"int"|"real"|"hex"|"bool"|"null", value: token payload

ValueIfExpression (AST: ValueIfExpressionSyntax)
- ValueIfExpression → IF ValueExpression COLON ValueExpression [ELSE COLON ValueExpression] END_IF
  - fields: condition: ExpressionSyntax, thenExpr: ExpressionSyntax, elseExpr?: ExpressionSyntax

ValueSwitchExpression (AST: ValueSwitchExpressionSyntax)
- ValueSwitchExpression → SWITCH ValueSwitchScrutineeOpt ValueSwitchCase+ ValueSwitchDefaultOpt END_SWITCH
  - fields: scrutinee?: ExpressionSyntax, cases: ValueSwitchCaseSyntax[], default?: ExpressionSyntax

ValueSwitchScrutineeOpt
- ValueSwitchScrutineeOpt → ValueExpression
- ValueSwitchScrutineeOpt → ε        (selected when next token is CASE or DEFAULT)
  - fields (on ValueSwitchExpressionSyntax): scrutinee?: ExpressionSyntax

ValueSwitchCase (AST: ValueSwitchCaseSyntax)
- ValueSwitchCase → CASE Pattern (COMMA Pattern)* COLON ValueExpression
  - fields: patterns: PatternSyntax[], expr: ExpressionSyntax

ValueSwitchDefault (AST: ValueSwitchDefaultSyntax)
- ValueSwitchDefault → DEFAULT COLON ValueExpression
  - fields: expr: ExpressionSyntax

ValueSwitchDefaultOpt
- ValueSwitchDefaultOpt → ValueSwitchDefault | ε

ValueForExpression (AST: ValueForExpressionSyntax)
- ValueForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression COLON ValueExpression END_FOR
  - fields: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: ExpressionSyntax

ForIndexOpt
- ForIndexOpt → COMMA IDENTIFIER | ε
  - fields (on ValueForExpressionSyntax/MarkupForExpressionSyntax): itemVar: string, indexVar?: string

ElementsExpression (AST: MarkupListSyntax)
- ElementsExpression → ElementsExpressionItem+
  - fields: items: MarkupItemSyntax[]

ElementsExpressionItem (AST: MarkupItemSyntax is a sum type)
- ElementsExpressionItem → Element                     (MarkupElementSyntax)
- ElementsExpressionItem → ElementsIfExpression        (MarkupIfExpressionSyntax)
- ElementsExpressionItem → ElementsSwitchExpression    (MarkupSwitchExpressionSyntax)
- ElementsExpressionItem → ElementsForExpression       (MarkupForExpressionSyntax)

ElementsIfExpression (AST: MarkupIfExpressionSyntax)
- ElementsIfExpression → IF ValueExpression COLON ElementsExpression [ELSE COLON ElementsExpression] END_IF
  - fields: condition: ExpressionSyntax, thenElements: MarkupListSyntax, elseElements?: MarkupListSyntax

ElementsSwitchExpression (AST: MarkupSwitchExpressionSyntax)
- ElementsSwitchExpression → SWITCH ValueSwitchScrutineeOpt ElementsSwitchCase+ ElementsSwitchDefaultOpt END_SWITCH
  - fields: scrutinee?: ExpressionSyntax, cases: MarkupSwitchCaseSyntax[], default?: MarkupListSyntax

ElementsSwitchCase (AST: MarkupSwitchCaseSyntax)
- ElementsSwitchCase → CASE Pattern (COMMA Pattern)* COLON ElementsExpression
  - fields: patterns: PatternSyntax[], elements: MarkupListSyntax

ElementsSwitchDefault (AST: MarkupSwitchDefaultSyntax)
- ElementsSwitchDefault → DEFAULT COLON ElementsExpression
  - fields: elements: MarkupListSyntax

ElementsSwitchDefaultOpt
- ElementsSwitchDefaultOpt → ElementsSwitchDefault | ε

ElementsForExpression (AST: MarkupForExpressionSyntax)
- ElementsForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression COLON ElementsExpression END_FOR
  - fields: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: MarkupListSyntax

Element (AST: MarkupElementSyntax is a sum type)
- Element → LT ElementName ElementSuffix

ElementSuffix (builds either Element or EmbedElement AST)
- ElementSuffix → PropertyArgument* RegularElementTail
- ElementSuffix → COLON EmbedTextType EmbedElementTail

RegularElementTail (AST: ElementSyntax)
- RegularElementTail → SLASH GT
  - fields: name (from ElementName), props: PropertyArgumentSyntax[], children: []
- RegularElementTail → GT Content LT SLASH ElementName GT
  - fields: name (from ElementName), props: PropertyArgumentSyntax[], children: ElementContentSyntax.items

EmbedElementTail (AST: EmbedElementSyntax)
- EmbedElementTail → PropertyArgument* GT EmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "parsed", props: PropertyArgumentSyntax[], content: EmbedContentSyntax.items
- EmbedElementTail → RAW PropertyArgument* GT RawEmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "raw", props: PropertyArgumentSyntax[], content: RawEmbedContentSyntax.text

ElementName
- ElementName → QualifiedMarkupName

EmbedTextType
- EmbedTextType → IDENTIFIER
  - fields (on EmbedElement): textType: string

PropertyArgument (AST: PropertyArgumentSyntax)
- PropertyArgument → QualifiedMarkupName EQ RhsExpression
  - fields: name: QualifiedMarkupNameSyntax, value: ExpressionSyntax

Content (AST: ElementContentSyntax is a sum type)
- Content → ElementsExpression
- Content → MixedContentExpression
  - fields: items: MarkupItemSyntax[]

MixedContentExpression (AST: MixedContentSyntax)
- MixedContentExpression → MixedContentItem*
  - fields: items: MixedContentItemSyntax[]

MixedContentItem (AST: MixedContentItemSyntax is a sum type)
- MixedContentItem → TextPart        (TextPartSyntax)
- MixedContentItem → Element         (MarkupElementSyntax)
- MixedContentItem → InterpolationExpression (InterpolationExpressionSyntax)

EmbedContent (AST: EmbedContentSyntax)
- EmbedContent → EmbedContentItem*
  - fields: items: EmbedContentItemSyntax[]

EmbedContentItem (AST: EmbedContentItemSyntax is a sum type)
- EmbedContentItem → TextRun         (TextRunSyntax)
- EmbedContentItem → InterpolationExpression (InterpolationExpressionSyntax)

RawEmbedContent (AST: RawEmbedContentSyntax)
- RawEmbedContent → TextRun
  - fields: text: string

TextPart (AST: TextPartSyntax)
- TextPart → TextRun
  - fields: text: string

TextRun (AST: TextRunSyntax)
- TextRun → (TEXT_CHUNK | ENTITY | ESCAPED_LBRACE | ESCAPED_RBRACE)+
  - fields: text: string  (concatenated, entities preserved or decoded by later phase)

QualifiedName (AST: QualifiedNameSyntax)
- QualifiedName → IDENTIFIER (DOT IDENTIFIER)*
  - fields: parts: string[]

QualifiedMarkupName (AST: QualifiedMarkupNameSyntax)
- QualifiedMarkupName → IDENTIFIER (DOT MARKUP_IDENTIFIER)*
  - fields: parts: string[]

Pattern (AST: PatternSyntax)
- Pattern → Literal
- Pattern → QualifiedName
  - fields: kind: "literal"|"name", value: LiteralExpressionSyntax|QualifiedNameSyntax

## AST Mapping Summary

This section lists the AST node types with fields for implementers.

- ModuleDefinitionSyntax: imports: ImportStatementSyntax[], types: TypeDefinitionSyntax[], functions: FunctionDefinitionSyntax[], moduleElement?: MarkupElementSyntax
- ImportStatementSyntax: name: QualifiedNameSyntax
- TypeDefinitionSyntax: name: string, type: TypeSyntax
- TypeSyntax: kind: "primitive"|"user", name: string (qualified), modifier?: "nullable"|"list"
- PrimitiveTypeSyntax: name: string
- UserTypeSyntax: name: QualifiedNameSyntax
- FunctionDefinitionSyntax: elementName: QualifiedMarkupNameSyntax, props: PropertyDefinitionSyntax[], body: ExpressionSyntax
- PropertyDefinitionSyntax: name: string, type: TypeSyntax, default?: ExpressionSyntax
- ExpressionSyntax: union of MarkupElementSyntax | LiteralExpressionSyntax | IdentifierNameSyntax | ValueIfExpressionSyntax | ValueSwitchExpressionSyntax | ValueForExpressionSyntax | CallExpressionSyntax | MemberAccessExpressionSyntax | BinaryExpressionSyntax | PrefixUnaryExpressionSyntax | ParenthesizedExpressionSyntax | UnitLiteralSyntax
 - CallExpressionSyntax: callee: ExpressionSyntax, args: ExpressionSyntax[]
 - MemberAccessExpressionSyntax: target: ExpressionSyntax, name: string
 - BinaryExpressionSyntax: op: token, left: ExpressionSyntax, right: ExpressionSyntax
 - PrefixUnaryExpressionSyntax: op: token, expr: ExpressionSyntax
 - ParenthesizedExpressionSyntax: expr: ExpressionSyntax (may be elided)
 - UnitLiteralSyntax
 - LiteralExpressionSyntax: kind, value
 - IdentifierNameSyntax: name: string
- ValueIfExpressionSyntax: condition: ExpressionSyntax, thenExpr: ExpressionSyntax, elseExpr?: ExpressionSyntax
- ValueSwitchExpressionSyntax: scrutinee?: ExpressionSyntax, cases: ValueSwitchCaseSyntax[], default?: ExpressionSyntax
- ValueSwitchCaseSyntax: patterns: PatternSyntax[], expr: ExpressionSyntax
- ValueSwitchDefaultSyntax: expr: ExpressionSyntax (usually folded into ValueSwitchExpressionSyntax.default)
- ValueForExpressionSyntax: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: ExpressionSyntax
- MarkupListSyntax: items: MarkupItemSyntax[]
- MarkupItemSyntax: MarkupElementSyntax | MarkupIfExpressionSyntax | MarkupSwitchExpressionSyntax | MarkupForExpressionSyntax
- MarkupIfExpressionSyntax: condition: ExpressionSyntax, thenElements: MarkupListSyntax, elseElements?: MarkupListSyntax
- MarkupSwitchExpressionSyntax: scrutinee?: ExpressionSyntax, cases: MarkupSwitchCaseSyntax[], default?: MarkupListSyntax
- MarkupSwitchCaseSyntax: patterns: PatternSyntax[], elements: MarkupListSyntax
- MarkupSwitchDefaultSyntax: elements: MarkupListSyntax (usually folded into MarkupSwitchExpressionSyntax.default)
- MarkupForExpressionSyntax: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: MarkupListSyntax
- MarkupElementSyntax: name: QualifiedMarkupNameSyntax, props: PropertyArgumentSyntax[], children: ElementContentSyntax (MarkupListSyntax or MixedContentSyntax)
- EmbedElementSyntax: name: QualifiedMarkupNameSyntax, textType: string, mode: "parsed"|"raw", props: PropertyArgumentSyntax[], content: EmbedContentSyntax|RawEmbedContentSyntax
- PropertyArgumentSyntax: name: QualifiedMarkupNameSyntax, value: ExpressionSyntax
- ElementContentSyntax: items: MarkupItemSyntax[] | MixedContentItemSyntax[]
- MixedContentSyntax: items: MixedContentItemSyntax[]
- MixedContentItemSyntax: kind: "text"|"element"|"interpolation", value: TextRunSyntax|MarkupElementSyntax|EmbedElementSyntax|InterpolationExpressionSyntax
- EmbedContentSyntax: items: (TextRunSyntax|InterpolationExpressionSyntax)[]
- RawEmbedContentSyntax: text: string
- TextPartSyntax: text: string
- TextRunSyntax: text: string
- InterpolationExpressionSyntax: expr: ExpressionSyntax
- QualifiedNameSyntax: parts: string[]
- QualifiedMarkupNameSyntax: parts: string[]
- PatternSyntax: kind: "literal"|"name", value: LiteralExpressionSyntax|QualifiedNameSyntax

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
