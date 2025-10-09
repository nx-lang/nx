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
- FOR ("for"), IN ("in")
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

Trivia (whitespace/comments; skipped by lexer)
- WHITESPACE (spaces, tabs, newlines)
- LINE_COMMENT ("//" to end of line)
- BLOCK_COMMENT ("/*" … "*/"; nests with same-kind openers)
- HTML_BLOCK_COMMENT ("<!--" … "-->"; nests with same-kind openers)

Notes
- The lexer does not produce tokens for trivia; they are attached as trivia or discarded.
- Comments and whitespace may appear between any tokens.
- Block comments are nestable with same-kind openers only. The lexer maintains a depth counter: increment on opener, decrement on closer, emit one token at depth 0. Unterminated blocks are lexing errors.
- Comments are not recognized inside string literals or text content tokens (TEXT_CHUNK/ENTITY/ESCAPED_*).

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

20: Conditional, right-associative
- led token: QMARK … COLON …
  - form: condition QMARK consequent COLON alternative → Conditional(condition, consequent, alternative)

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
- ValueExpression → ValueForExpression
- ValueExpression → ValueExpr

ValueExpr (parsed by Pratt; not a standalone AST node)
- Primaries (nud): Literal | IDENTIFIER | Unit | ParenthesizedExpression
- Postfix/infix handled via the operator table (including the conditional operator `?:` at precedence 20)

Unit (AST: UnitLiteralSyntax)
- Unit → LPAREN RPAREN
  - fields: (none)

ParenthesizedExpression (AST: ParenthesizedExpressionSyntax)
- ParenthesizedExpression → LPAREN ValueExpression RPAREN
  - fields: expr: ExpressionSyntax

ConditionalExpression (AST: ConditionalExpressionSyntax)
- ConditionalExpression → ValueExpression QMARK ValueExpression COLON ValueExpression  (parsed via Pratt entry at precedence 20; right-associative)
  - fields: condition: ExpressionSyntax, whenTrue: ExpressionSyntax, whenFalse: ExpressionSyntax

Literal (AST: LiteralExpressionSyntax)
- Literal → STRING_LITERAL | INT_LITERAL | REAL_LITERAL | HEX_LITERAL | BOOL_LITERAL | NULL_LITERAL
  - fields: kind: "string"|"int"|"real"|"hex"|"bool"|"null", value: token payload

ValueIfExpression (AST: ExpressionSyntax is a sum type)
- ValueIfExpression → ValueIfSimpleExpression        (ValueIfSimpleExpressionSyntax)
- ValueIfExpression → ValueIfMatchExpression         (ValueIfMatchExpressionSyntax)
- ValueIfExpression → ValueIfConditionListExpression (ValueIfConditionListExpressionSyntax)

ValueIfSimpleExpression (AST: ValueIfSimpleExpressionSyntax)
- ValueIfSimpleExpression → IF ValueExpression LBRACE ValueExpression RBRACE ValueIfElseClauseOpt
  - fields: condition: ExpressionSyntax, thenExpr: ExpressionSyntax, elseExpr?: ExpressionSyntax

ValueIfElseClauseOpt
- ValueIfElseClauseOpt → ELSE LBRACE ValueExpression RBRACE
- ValueIfElseClauseOpt → ε
  - fields (on ValueIfSimpleExpressionSyntax): elseExpr?: ExpressionSyntax

ValueIfMatchExpression (AST: ValueIfMatchExpressionSyntax)
- ValueIfMatchExpression → IF ValueIfMatchScrutineeOpt IS LBRACE ValueIfMatchArm+ ValueIfMatchElseOpt RBRACE
  - fields: scrutinee?: ExpressionSyntax, arms: ValueIfMatchArmSyntax[], elseExpr?: ExpressionSyntax

ValueIfMatchScrutineeOpt
- ValueIfMatchScrutineeOpt → ValueExpression
- ValueIfMatchScrutineeOpt → ε        (selected when next token is IS)
  - fields (on ValueIfMatchExpressionSyntax): scrutinee?: ExpressionSyntax

ValueIfMatchArm (AST: ValueIfMatchArmSyntax)
- ValueIfMatchArm → Pattern (COMMA Pattern)* COLON ValueExpression
  - fields: patterns: PatternSyntax[], expr: ExpressionSyntax

ValueIfMatchElseOpt
- ValueIfMatchElseOpt → ELSE COLON ValueExpression
- ValueIfMatchElseOpt → ε
  - fields (on ValueIfMatchExpressionSyntax): elseExpr?: ExpressionSyntax

ValueIfConditionListExpression (AST: ValueIfConditionListExpressionSyntax)
- ValueIfConditionListExpression → IF ValueIfConditionScrutineeOpt LBRACE ValueIfConditionArm+ ValueIfConditionElseOpt RBRACE
  - fields: scrutinee?: ExpressionSyntax, arms: ValueIfConditionArmSyntax[], elseExpr?: ExpressionSyntax

ValueIfConditionScrutineeOpt
- ValueIfConditionScrutineeOpt → ValueExpression
- ValueIfConditionScrutineeOpt → ε        (selected when next token starts a condition arm)
  - fields (on ValueIfConditionListExpressionSyntax): scrutinee?: ExpressionSyntax

ValueIfConditionArm (AST: ValueIfConditionArmSyntax)
- ValueIfConditionArm → ValueExpression COLON ValueExpression
  - fields: condition: ExpressionSyntax, expr: ExpressionSyntax

ValueIfConditionElseOpt
- ValueIfConditionElseOpt → ELSE COLON ValueExpression
- ValueIfConditionElseOpt → ε
  - fields (on ValueIfConditionListExpressionSyntax): elseExpr?: ExpressionSyntax

ValueForExpression (AST: ValueForExpressionSyntax)
- ValueForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression LBRACE ValueExpression RBRACE
  - fields: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: ExpressionSyntax

ForIndexOpt
- ForIndexOpt → COMMA IDENTIFIER | ε
  - fields (on ValueForExpressionSyntax/MarkupForExpressionSyntax): itemVar: string, indexVar?: string

ElementsExpression (AST: MarkupListSyntax)
- ElementsExpression → ElementsExpressionItem+
  - fields: items: MarkupItemSyntax[]

- ElementsExpressionItem → Element                     (MarkupElementSyntax)
- ElementsExpressionItem → ElementsIfExpression        (MarkupIfSimpleExpressionSyntax | MarkupIfMatchExpressionSyntax | MarkupIfConditionListExpressionSyntax)
- ElementsExpressionItem → ElementsForExpression       (MarkupForExpressionSyntax)

ElementsIfExpression (AST: MarkupItemSyntax is a sum type)
- ElementsIfExpression → ElementsIfSimpleExpression        (MarkupIfSimpleExpressionSyntax)
- ElementsIfExpression → ElementsIfMatchExpression         (MarkupIfMatchExpressionSyntax)
- ElementsIfExpression → ElementsIfConditionListExpression (MarkupIfConditionListExpressionSyntax)

ElementsIfSimpleExpression (AST: MarkupIfSimpleExpressionSyntax)
- ElementsIfSimpleExpression → IF ValueExpression LBRACE ElementsExpression RBRACE ElementsIfElseClauseOpt
  - fields: condition: ExpressionSyntax, thenElements: MarkupListSyntax, elseElements?: MarkupListSyntax

ElementsIfElseClauseOpt
- ElementsIfElseClauseOpt → ELSE LBRACE ElementsExpression RBRACE
- ElementsIfElseClauseOpt → ε
  - fields (on MarkupIfSimpleExpressionSyntax): elseElements?: MarkupListSyntax

ElementsIfMatchExpression (AST: MarkupIfMatchExpressionSyntax)
- ElementsIfMatchExpression → IF ElementsIfMatchScrutineeOpt IS LBRACE ElementsIfMatchArm+ ElementsIfMatchElseOpt RBRACE
  - fields: scrutinee?: ExpressionSyntax, arms: MarkupIfMatchArmSyntax[], elseElements?: MarkupListSyntax

ElementsIfMatchScrutineeOpt
- ElementsIfMatchScrutineeOpt → ValueExpression
- ElementsIfMatchScrutineeOpt → ε        (selected when next token is IS)
  - fields (on MarkupIfMatchExpressionSyntax): scrutinee?: ExpressionSyntax

ElementsIfMatchArm (AST: MarkupIfMatchArmSyntax)
- ElementsIfMatchArm → Pattern (COMMA Pattern)* COLON ElementsExpression
  - fields: patterns: PatternSyntax[], elements: MarkupListSyntax

ElementsIfMatchElseOpt
- ElementsIfMatchElseOpt → ELSE COLON ElementsExpression
- ElementsIfMatchElseOpt → ε
  - fields (on MarkupIfMatchExpressionSyntax): elseElements?: MarkupListSyntax

ElementsIfConditionListExpression (AST: MarkupIfConditionListExpressionSyntax)
- ElementsIfConditionListExpression → IF ElementsIfConditionScrutineeOpt LBRACE ElementsIfConditionArm+ ElementsIfConditionElseOpt RBRACE
  - fields: scrutinee?: ExpressionSyntax, arms: MarkupIfConditionArmSyntax[], elseElements?: MarkupListSyntax

ElementsIfConditionScrutineeOpt
- ElementsIfConditionScrutineeOpt → ValueExpression
- ElementsIfConditionScrutineeOpt → ε        (selected when next token starts a condition arm)
  - fields (on MarkupIfConditionListExpressionSyntax): scrutinee?: ExpressionSyntax

ElementsIfConditionArm (AST: MarkupIfConditionArmSyntax)
- ElementsIfConditionArm → ValueExpression COLON ElementsExpression
  - fields: condition: ExpressionSyntax, elements: MarkupListSyntax

ElementsIfConditionElseOpt
- ElementsIfConditionElseOpt → ELSE COLON ElementsExpression
- ElementsIfConditionElseOpt → ε
  - fields (on MarkupIfConditionListExpressionSyntax): elseElements?: MarkupListSyntax

ValueSwitchScrutineeOpt
- ValueSwitchScrutineeOpt → ValueExpression
- ValueSwitchScrutineeOpt → ε        (selected when next token is CASE or DEFAULT)
  - fields (on PropertySwitchSyntax): scrutinee?: ExpressionSyntax

ElementsForExpression (AST: MarkupForExpressionSyntax)
- ElementsForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression LBRACE ElementsExpression RBRACE
  - fields: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: MarkupListSyntax

Element (AST: MarkupElementSyntax is a sum type)
- Element → LT ElementName ElementSuffix

ElementSuffix (builds either Element or EmbedElement AST)
- ElementSuffix → PropertyList RegularElementTail
- ElementSuffix → COLON EmbedTextType EmbedElementTail

RegularElementTail (AST: ElementSyntax)
- RegularElementTail → SLASH GT
  - fields: name (from ElementName), props: PropertyListSyntax, children: []
- RegularElementTail → GT Content LT SLASH ElementName GT
  - fields: name (from ElementName), props: PropertyListSyntax, children: ElementContentSyntax.items

EmbedElementTail (AST: EmbedElementSyntax)
- EmbedElementTail → PropertyList GT EmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "parsed", props: PropertyListSyntax, content: EmbedContentSyntax.items
- EmbedElementTail → RAW PropertyList GT RawEmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "raw", props: PropertyListSyntax, content: RawEmbedContentSyntax.text

ElementName
- ElementName → QualifiedMarkupName

EmbedTextType
- EmbedTextType → IDENTIFIER
  - fields (on EmbedElement): textType: string

PropertyList (AST: PropertyListSyntax)
- PropertyList → PropertyListItem*
  - fields: items: PropertyListItemSyntax[]

PropertyListItem (AST: PropertyListItemSyntax is a sum type)
- PropertyListItem → PropertyValue                (PropertyValueSyntax)
- PropertyListItem → PropertyListIf               (PropertyIfSyntax)
- PropertyListItem → PropertyListSwitch           (PropertySwitchSyntax)

PropertyValue (AST: PropertyValueSyntax)
- PropertyValue → QualifiedMarkupName EQ RhsExpression
  - fields: name: QualifiedMarkupNameSyntax, value: ExpressionSyntax

PropertyListIf (AST: PropertyIfSyntax)
- PropertyListIf → IF ValueExpression COLON PropertyList [ELSE COLON PropertyList] END_IF
  - fields: condition: ExpressionSyntax, thenProps: PropertyListSyntax, elseProps?: PropertyListSyntax

PropertyListSwitch (AST: PropertySwitchSyntax)
- PropertyListSwitch → SWITCH ValueSwitchScrutineeOpt PropertyListSwitchCase+ PropertyListSwitchDefaultOpt END_SWITCH
  - fields: scrutinee?: ExpressionSyntax, cases: PropertySwitchCaseSyntax[], default?: PropertyListSyntax

PropertyListSwitchCase (AST: PropertySwitchCaseSyntax)
- PropertyListSwitchCase → CASE Pattern (COMMA Pattern)* COLON PropertyList
  - fields: patterns: PatternSyntax[], props: PropertyListSyntax

PropertyListSwitchDefault (AST: PropertySwitchDefaultSyntax)
- PropertyListSwitchDefault → DEFAULT COLON PropertyList
  - fields: props: PropertyListSyntax

PropertyListSwitchDefaultOpt
- PropertyListSwitchDefaultOpt → PropertyListSwitchDefault | ε

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
- ExpressionSyntax: union of MarkupElementSyntax | LiteralExpressionSyntax | IdentifierNameSyntax | ValueIfSimpleExpressionSyntax | ValueIfMatchExpressionSyntax | ValueIfConditionListExpressionSyntax | ValueForExpressionSyntax | ConditionalExpressionSyntax | CallExpressionSyntax | MemberAccessExpressionSyntax | BinaryExpressionSyntax | PrefixUnaryExpressionSyntax | ParenthesizedExpressionSyntax | UnitLiteralSyntax
 - CallExpressionSyntax: callee: ExpressionSyntax, args: ExpressionSyntax[]
 - MemberAccessExpressionSyntax: target: ExpressionSyntax, name: string
 - ConditionalExpressionSyntax: condition: ExpressionSyntax, whenTrue: ExpressionSyntax, whenFalse: ExpressionSyntax
 - BinaryExpressionSyntax: op: token, left: ExpressionSyntax, right: ExpressionSyntax
 - PrefixUnaryExpressionSyntax: op: token, expr: ExpressionSyntax
 - ParenthesizedExpressionSyntax: expr: ExpressionSyntax (may be elided)
 - UnitLiteralSyntax
 - LiteralExpressionSyntax: kind, value
 - IdentifierNameSyntax: name: string
- ValueIfSimpleExpressionSyntax: condition: ExpressionSyntax, thenExpr: ExpressionSyntax, elseExpr?: ExpressionSyntax
- ValueIfMatchExpressionSyntax: scrutinee?: ExpressionSyntax, arms: ValueIfMatchArmSyntax[], elseExpr?: ExpressionSyntax
- ValueIfMatchArmSyntax: patterns: PatternSyntax[], expr: ExpressionSyntax
- ValueIfConditionListExpressionSyntax: scrutinee?: ExpressionSyntax, arms: ValueIfConditionArmSyntax[], elseExpr?: ExpressionSyntax
- ValueIfConditionArmSyntax: condition: ExpressionSyntax, expr: ExpressionSyntax
- ValueForExpressionSyntax: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: ExpressionSyntax
- MarkupListSyntax: items: MarkupItemSyntax[]
- MarkupItemSyntax: MarkupElementSyntax | MarkupIfSimpleExpressionSyntax | MarkupIfMatchExpressionSyntax | MarkupIfConditionListExpressionSyntax | MarkupForExpressionSyntax
- MarkupIfSimpleExpressionSyntax: condition: ExpressionSyntax, thenElements: MarkupListSyntax, elseElements?: MarkupListSyntax
- MarkupIfMatchExpressionSyntax: scrutinee?: ExpressionSyntax, arms: MarkupIfMatchArmSyntax[], elseElements?: MarkupListSyntax
- MarkupIfMatchArmSyntax: patterns: PatternSyntax[], elements: MarkupListSyntax
- MarkupIfConditionListExpressionSyntax: scrutinee?: ExpressionSyntax, arms: MarkupIfConditionArmSyntax[], elseElements?: MarkupListSyntax
- MarkupIfConditionArmSyntax: condition: ExpressionSyntax, elements: MarkupListSyntax
- MarkupForExpressionSyntax: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: MarkupListSyntax
- MarkupElementSyntax: name: QualifiedMarkupNameSyntax, props: PropertyListSyntax, children: ElementContentSyntax (MarkupListSyntax or MixedContentSyntax)
- EmbedElementSyntax: name: QualifiedMarkupNameSyntax, textType: string, mode: "parsed"|"raw", props: PropertyListSyntax, content: EmbedContentSyntax|RawEmbedContentSyntax
- PropertyListSyntax: items: PropertyListItemSyntax[]
- PropertyListItemSyntax: PropertyValueSyntax | PropertyIfSyntax | PropertySwitchSyntax
- PropertyValueSyntax: name: QualifiedMarkupNameSyntax, value: ExpressionSyntax
- PropertyIfSyntax: condition: ExpressionSyntax, thenProps: PropertyListSyntax, elseProps?: PropertyListSyntax
- PropertySwitchSyntax: scrutinee?: ExpressionSyntax, cases: PropertySwitchCaseSyntax[], default?: PropertyListSyntax
- PropertySwitchCaseSyntax: patterns: PatternSyntax[], props: PropertyListSyntax
- PropertySwitchDefaultSyntax: props: PropertyListSyntax (usually folded into PropertySwitchSyntax.default)
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
  - If next token is FOR → ValueForExpression
  - Otherwise → ValueExpr (Pratt)
- ElementsExpression item selection:
  - If next token is LT → Element
  - If next token ∈ {IF, FOR} → the corresponding Elements* form
- SWITCH scrutinee (elements/property variants):
  - After SWITCH, if next token ∈ {CASE, DEFAULT} → no scrutinee
  - Else → parse ValueExpression as the scrutinee
- Element is left-factored: after LT ElementName, COLON selects the embed branch; otherwise parse PropertyList and choose SLASH GT (self-closing) or GT … LT SLASH ElementName GT using lookahead at SLASH vs GT.

## Validation Rules (post-parse)

- Element closing tag name must match opening ElementName.
- EmbedElement closing tag name must match opening ElementName.
- PropertyDefinition names within a single FunctionDefinition should be unique.
- Type modifiers: at most one of QMARK or ELLIPSIS.
- Switch expressions (property variants): at least one case; patterns per case must be non-empty.
- ValueIfMatchExpression / ElementsIfMatchExpression: at least one pattern arm; each arm requires ≥1 pattern.
- ValueIfConditionListExpression / ElementsIfConditionListExpression: at least one condition arm.

## Notes and Gaps

- Pattern is limited to constant-like forms (Literal or QualifiedName). Extend as needed.
- Entities in TextRun are preserved as ENTITY tokens; decoding can be a later phase.
- ParenthesizedExpression may be elided in AST after parsing.
